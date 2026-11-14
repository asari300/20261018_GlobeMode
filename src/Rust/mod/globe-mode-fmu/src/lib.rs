//! GlobeMode の自前の FMI 2.0 Model Exchange のランタイム（P5b、S095・S099、ADR-0032）。
//!
//! S099: モデルは FMU の `resources/<完全修飾名>.mo`（1 ファイル）から `fmi2Instantiate` で compile し、
//! `globe_mode_exec::ModelInstance` で微分を求める。`modelDescription.xml` は [`model_description`] が同じ道で作る
//! （手書きをやめた）。
//!
//! S109: 事象。`fmi2GetEventIndicators` は root の条件の値（`numberOfEventIndicators` 本）、`fmi2EnterEventMode` →
//! `fmi2NewDiscreteStates` で `when` と `reinit` を回し、次の時刻事象（`sample`）を `nextEventTime` で返す。
//!
//! S112: 代数・離散の変数（`ModelInstance::variables`）を `fmi2GetReal`・`fmi2GetInteger`・`fmi2GetBoolean` で読む。
//!
//! S114: 入力（`causality="input"`）を `fmi2SetReal` で書く（`ModelInstance::set_input`）。
//!
//! S137: 同じ cdylib で FMI 3.0 の ME も出す（`fmi3*`。2.0 の関数を形だけ変えて呼ぶ。`modelDescription.xml` は
//! [`model_description_3`]）。3.0 の EventIndicator は値参照 `2n+p+q..`（q は代数・離散・入力の変数の数）の
//! Float64 の変数として載せ、その次に independent の `time` を置く。
//!
//! S231: Co-Simulation。`fmi2Instantiate(CoSimulation)`・`fmi3InstantiateCoSimulation` の instance は `fmi2DoStep`・`fmi3DoStep` で進む
//! （中は固定刻みの RK4 と、ME と同じ事象処理。`modelDescription.xml` は [`model_description_kind`]）。入力・出力の微分、
//! `fmi2CancelStep`、`fmi2Get*Status` は `Error`。
//!
//! S251: Scheduled Execution（FMI 3.0 のみ）。`fmi3InstantiateScheduledExecution` の instance は周期クロック 1 つ（パーティション 1 つ）を持ち、
//! `fmi3ActivateModelPartition(clock, t)` が前の時刻から `t` まで [`do_step`] で進める（CS と同じ固定刻みの RK4・事象処理）。
//! クロックの値参照は変数の最後（`time` の次。[`clock_ref`]）、周期は `experiment(Interval)`（無ければ 0.01）。
//!
//! S237: MSL を使うモデル。FMU の `resources/lib/` にライブラリ（MSL の `.mo`）を入れると `fmi2Instantiate` がそれを `MODELICAPATH` として
//! 読む（`resources/<完全修飾名>.mo` はそのモデルの文面。モデルがライブラリの中ならコメントだけでよい）。関係式の記憶は初期化の点から決める
//! （`globe-mode-exec` の `ModelInstance::new`）。内部の名前 `$formal_derivative.1.x` は引用符で囲む（structured の文法）。
//!
//! - 値参照: `0..n` = 状態、`n..2n` = その微分（`der(…)`）、`2n..2n+p` = tunable なパラメータ、
//!   `2n+p..` = 代数・出力・離散・入力の変数（S112・S114。書けるのは Real の入力だけ）。
//! - パラメータの変更は `fmi2ExitInitializationMode` で lowering からやり直して反映する（依存するパラメータにも届く）。
//! - S224: Enumeration は Integer（3.0 は Int64）の ordinal として読み書きし（`declaredType` と `TypeDefinitions` を書く）、
//!   String のパラメータは `fmi2GetString` で読む（`fixed` なので書くと `fmi2Error`）。値参照はパラメータの枠のまま。
//! - 状態の保存（FMUstate）・方向微分と、Boolean と Enumeration でない Integer の書き込みは無いので `fmi2Error` を返す（`nvr = 0` は OK）。
//! - ログの callback は呼ばない（失敗の理由は標準エラーに出す）。panic は `extern "C"` の境界で abort する（Rust 2024）。
// 安全の約束は FMI 2.0 の規格（呼び手は fmi2Instantiate が返した c と、nvr 個の有効な配列を渡す）
#![allow(clippy::missing_safety_doc)]

use std::collections::BTreeMap;
use std::ffi::{CStr, c_char, c_int, c_uint, c_void};
use std::path::{Path, PathBuf};

use globe_mode_exec::{CompiledModel, ModelInstance, ValueType, VariableRole};
use globe_mode_frontend::{Checker, Source};

type Component = *mut c_void;
type Status = c_int;
const OK: Status = 0;
const ERROR: Status = 3;
/// `fmi2ModelExchange`
const MODEL_EXCHANGE: c_int = 0;
/// `fmi2CoSimulation`
const CO_SIMULATION: c_int = 1;

/// `modelDescription.xml` の `guid`。モデルの名前と文面の FNV-1a（64 ビット）なので、`fmi2Instantiate` が
/// resources の文面から同じ値を作って突き合わせられる。
pub fn guid(model: &str, source: &str) -> String {
    let hash = [model.as_bytes(), &[0], source.as_bytes()]
        .concat()
        .iter()
        .fold(0xcbf2_9ce4_8422_2325_u64, |h, &b| {
            (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3)
        });
    format!("{{globe-mode-{hash:016x}}}")
}

/// 1 ファイルの Modelica の文面から `model` を DAE まで変換する（MSL は読まない）。
pub fn compile(model: &str, source: &str) -> Result<CompiledModel, String> {
    compile_in(model, source, None)
}

/// [`compile`] に、ライブラリのディレクトリ（`MODELICAPATH` の 1 要素。FMU では `resources/lib`、S237）を足す。
/// `model` がライブラリの中のクラスなら `source` は空（コメントだけ）でよい。
pub fn compile_in(model: &str, source: &str, lib: Option<&Path>) -> Result<CompiledModel, String> {
    let checker = Checker::new(lib.as_slice()).map_err(|d| format!("{d:?}"))?;
    let sources = [Source {
        name: "model.mo",
        text: source,
    }];
    checker
        .compile_retaining(&sources, model)
        .map_err(|d| format!("{d:?}"))
}

/// FMU の `modelDescription.xml`（FMI 2.0 ME）。変数表は [`ModelInstance`] の状態とパラメータから作り、
/// XML は `globe_mode_fmi::write_model_description_2` が書く（S117、P5a と型を共有）。
pub fn model_description(model: &str, source: &str) -> Result<String, String> {
    describe(model, source, false, Kind::Me, None)
}

/// FMU の `modelDescription.xml`（FMI 3.0 ME、S137）。変数表は 2.0 と同じで、EventIndicator の変数を足す。
pub fn model_description_3(model: &str, source: &str) -> Result<String, String> {
    describe(model, source, true, Kind::Me, None)
}

/// `modelDescription.xml` に書くインターフェース（S231）。既定の [`model_description`]・[`model_description_3`] は `Me`。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Me,
    Cs,
    Both,
    /// S251: FMI 3.0 の Scheduled Execution だけ（周期クロック 1 つ）
    Se,
}

/// [`model_description`]（`v3` が偽）・[`model_description_3`]（真）と同じで、`kind` に Co-Simulation を含めると
/// `<CoSimulation modelIdentifier="globe_mode_fmu" canHandleVariableCommunicationStepSize="true"/>` を書く（S231）。
pub fn model_description_kind(
    model: &str,
    source: &str,
    v3: bool,
    kind: Kind,
) -> Result<String, String> {
    describe(model, source, v3, kind, None)
}

/// [`model_description_kind`] に、`resources/lib` に入れるライブラリのディレクトリ（MSL など、S237）を足す。
pub fn model_description_lib(
    model: &str,
    source: &str,
    v3: bool,
    kind: Kind,
    lib: &Path,
) -> Result<String, String> {
    describe(model, source, v3, kind, Some(lib))
}

fn describe(
    model: &str,
    source: &str,
    v3: bool,
    kind: Kind,
    lib: Option<&Path>,
) -> Result<String, String> {
    use globe_mode_fmi::{
        Causality, DefaultExperiment, EnumerationType, Initial, Interface, ModelDescription,
        ModelStructure, ScalarVariable, VarType, Variability,
    };
    if kind == Kind::Se && !v3 {
        return Err("Scheduled Execution は FMI 3.0 だけ".into());
    }
    let compiled = compile_in(model, source, lib)?;
    let inst =
        ModelInstance::new(&compiled, &BTreeMap::new(), 0.0).map_err(|d| format!("{d:?}"))?;
    let e = compiled.experiment.clone().unwrap_or_default();
    let start = e.start_time.map_or(0.0, |b| b.get());
    let n = inst.n_states();
    // ModelStructure と derivative の参照: 2.0 は変数の 1 始まりの索引、3.0 は valueReference
    let refer = |r: usize| r as u32 + u32::from(!v3);
    let var = |name: String, r: usize, causality, variability, initial, ty, start| ScalarVariable {
        name,
        value_reference: r as u32,
        causality,
        variability,
        ty,
        initial,
        start,
        derivative: None,
        declared_type: None,
        dimensions: false,
    };
    let mut variables = Vec::new();
    for (i, (name, x)) in inst.state_names().iter().zip(inst.states()).enumerate() {
        variables.push(var(
            name.clone(),
            i,
            Causality::Local,
            Variability::Continuous,
            Some(Initial::Exact),
            VarType::Float64,
            Some(format!("{x:?}")),
        ));
    }
    for (i, name) in inst.state_names().iter().enumerate() {
        variables.push(ScalarVariable {
            derivative: Some(refer(i)),
            ..var(
                format!("der({name})"),
                n + i,
                Causality::Local,
                Variability::Continuous,
                Some(Initial::Calculated),
                VarType::Float64,
                None,
            )
        });
    }
    let strings = compiled.string_parameters();
    let mut enum_types = Vec::new();
    // S224: Enumeration の変数は Int32 の値（ordinal）に declaredType を付け、型の定義を TypeDefinitions に書く
    let mut declare = |name: &str, v: &mut ScalarVariable| {
        if let Some(t) = compiled.enum_type_of(name) {
            v.ty = VarType::Enumeration;
            v.declared_type = Some(t.name.clone());
            v.start = v.start.take().map(|s| {
                s.parse::<f64>()
                    .map_or(s, |x| format!("{}", x.round() as i64))
            });
            if !enum_types
                .iter()
                .any(|e: &EnumerationType| e.name == t.name)
            {
                let items = t
                    .literals
                    .iter()
                    .zip(1..)
                    .map(|(l, i)| (l.clone(), i))
                    .collect();
                enum_types.push(EnumerationType {
                    name: t.name.clone(),
                    items,
                });
            }
        }
    };
    for (i, (name, p)) in inst.parameters().iter().enumerate() {
        let mut v = var(
            name.clone(),
            2 * n + i,
            Causality::Parameter,
            Variability::Fixed,
            Some(Initial::Exact),
            VarType::Float64,
            Some(format!("{p:?}")),
        );
        declare(name, &mut v);
        // S224: String のパラメータも Rumoca のパラメータの枠に 1 つ入っている（Real の 0.0）。型と値を直す
        if let Some((_, text)) = strings.iter().find(|(n, _)| n == name) {
            (v.ty, v.start) = (VarType::String, Some(text.clone()));
        }
        variables.push(v);
    }
    let first = 2 * n + inst.parameters().len();
    let values = inst.variable_values()?;
    let mut outputs = Vec::new();
    for (i, v) in inst.variables().enumerate() {
        let r = first + i;
        let (ty, discrete) = match v.value_type {
            ValueType::Real => (VarType::Float64, v.role == VariableRole::Discrete),
            ValueType::Integer => (VarType::Int32, true),
            ValueType::Boolean => (VarType::Boolean, true),
            ValueType::Enumeration => (VarType::Int32, true),
        };
        let (causality, initial, start) = match v.role {
            VariableRole::Output => {
                outputs.push(refer(r));
                (Causality::Output, Some(Initial::Calculated), None)
            }
            // 入力は initial を書かず start を必須とする（FMI 2.0 の表）
            VariableRole::Input => {
                // start は変数の型で書く（Integer・Boolean に f64 の `2.0` を書くと XSD に反する。#51）
                let start = match ty {
                    VarType::Int32 => format!("{}", values[i] as i32),
                    VarType::Boolean => format!("{}", values[i] != 0.0),
                    _ => format!("{:?}", values[i]),
                };
                (Causality::Input, None, Some(start))
            }
            _ => (Causality::Local, Some(Initial::Calculated), None),
        };
        let variability = if discrete {
            Variability::Discrete
        } else {
            Variability::Continuous
        };
        let mut sv = var(
            v.name.clone(),
            r,
            causality,
            variability,
            initial,
            ty,
            start,
        );
        if v.value_type == ValueType::Enumeration {
            declare(&v.name, &mut sv);
        }
        variables.push(sv);
    }
    let mut event_indicators = Vec::new();
    if v3 {
        for i in 0..inst.n_event_indicators() {
            let r = first + values.len() + i;
            event_indicators.push(r as u32);
            variables.push(var(
                format!("_eventIndicator{}", i + 1),
                r,
                Causality::Local,
                Variability::Continuous,
                Some(Initial::Calculated),
                VarType::Float64,
                None,
            ));
        }
        // 3.0 は independent の変数がちょうど 1 つ要る（FMPy の validate）
        variables.push(var(
            "time".into(),
            first + values.len() + inst.n_event_indicators(),
            Causality::Independent,
            Variability::Continuous,
            None,
            VarType::Float64,
            None,
        ));
    }
    let dependencies = compiled
        .dae_structure_with_knowns()
        .ok()
        .and_then(|(structure, known)| {
            globe_mode_analysis::unknown_dependencies(&structure, &known)
        })
        .map(|deps| globe_mode_fmi::model_structure_dependencies(&variables, v3, &deps))
        .unwrap_or_default();
    // S237: `$formal_derivative.1.x` のような内部の名前は structured の文法に合わない（FMPy の validate が断る）ので、
    // 引用符で 1 つの識別子にする
    for v in &mut variables {
        if v.name.contains('$') {
            v.name = format!("'{}'", v.name);
        }
    }
    let derivatives: Vec<u32> = (n..2 * n).map(refer).collect();
    // 出力（initial="calculated"）は InitialUnknowns にも並べる（FMI 2.0。索引は微分より後なので昇順のまま）
    let initial_unknowns = [derivatives.clone(), outputs.clone()].concat();
    let md = ModelDescription {
        fmi_version: if v3 { "3.0" } else { "2.0" }.into(),
        model_name: model.into(),
        instantiation_token: guid(model, source),
        model_exchange: matches!(kind, Kind::Me | Kind::Both).then(|| Interface {
            model_identifier: "globe_mode_fmu".into(),
        }),
        co_simulation: matches!(kind, Kind::Cs | Kind::Both).then(|| Interface {
            model_identifier: "globe_mode_fmu".into(),
        }),
        scheduled_execution: (kind == Kind::Se).then(|| Interface {
            model_identifier: "globe_mode_fmu".into(),
        }),
        clock: (kind == Kind::Se).then(|| globe_mode_fmi::PeriodicClock {
            name: "clock".into(),
            value_reference: clock_ref(&inst),
            // shortcut: 周期は experiment(Interval) か 0.01 の 1 つだけ。複数のクロック・パーティションは未対応
            interval: format!("{:?}", e.interval.map_or(0.01, |b| b.get())),
        }),
        variables,
        generation_tool: Some("GlobeMode globe-mode-fmu".into()),
        variable_naming_convention: Some("structured".into()),
        number_of_event_indicators: inst.n_event_indicators() as u32,
        default_experiment: Some(DefaultExperiment {
            start_time: Some(format!("{start:?}")),
            stop_time: Some(format!(
                "{:?}",
                e.stop_time.map_or(start + 1.0, |b| b.get())
            )),
            tolerance: Some(format!("{:?}", e.tolerance.map_or(1e-6, |b| b.get()))),
            step_size: None,
        }),
        enum_types,
        model_structure: ModelStructure {
            outputs,
            derivatives,
            initial_unknowns,
            event_indicators,
            dependencies,
        },
    };
    if v3 {
        globe_mode_fmi::write_model_description_3(&md)
    } else {
        globe_mode_fmi::write_model_description_2(&md)
    }
    .map_err(|e| e.to_string())
}

/// SE の周期クロックの値参照（S251）。3.0 の変数表（状態・微分・パラメータ・代数／離散／入力・EventIndicator・`time`）の次。
fn clock_ref(inst: &ModelInstance) -> u32 {
    let (n, p) = (inst.n_states(), inst.parameters().len());
    (2 * n + p + inst.variables().count() + inst.n_event_indicators() + 1) as u32
}

/// 1 つの FMU のインスタンス。
struct Instance {
    model: CompiledModel,
    inst: ModelInstance,
    start: f64,
    /// `fmi2SetReal` で変えたパラメータ（`fmi2ExitInitializationMode` で lowering からやり直して反映する）
    overrides: BTreeMap<String, f64>,
    /// `fmi2SetReal` で与えた入力（作り直したときに置き直す）
    inputs: BTreeMap<String, f64>,
    /// `fmi2SetReal` で与えた状態の値（添字 → 値。作り直したときに置き直す。start 値と同じ）
    states: BTreeMap<usize, f64>,
    /// String のパラメータの値（`fmi2GetString` が返すポインタの置き場）。値参照は Rumoca のパラメータの枠の位置
    /// （`2n+i`。String も 1 枠を取るので、Real のパラメータと同じ数え方）
    strings: BTreeMap<String, std::ffi::CString>,
    dirty: bool,
    /// S231: Co-Simulation で作られたか（`fmi2DoStep` / `fmi3DoStep` を受ける）
    cs: bool,
    /// S251: Scheduled Execution で作られたか（`cs` も真。`fmi3ActivateModelPartition` を受ける）
    se: bool,
    /// SE: `fmi3ActivateModelPartition` を 1 度でも呼んだか（最初の呼び出しだけ開始時刻と同じ時刻を許す）
    activated: bool,
    /// CS: 次の時刻事象（`fmi2ExitInitializationMode` と事象のたびに更新）
    t_next: Option<f64>,
    /// CS: 直前の点の事象指示関数（領域が変わったら状態事象）
    z_prev: Vec<f64>,
}

impl Instance {
    /// パラメータか開始時刻が変わっていれば、作り直す。
    fn rebuild(&mut self) -> bool {
        if self.dirty {
            // shortcut: 初期化は入力の start で解き、作り直した後に与えた入力を置く。初期値が入力に依るモデルを載せるときに
            // `ModelInstance::new` へ入力を渡す
            match ModelInstance::new(&self.model, &self.overrides, self.start) {
                Ok(mut inst) => {
                    for (name, v) in &self.inputs {
                        let _ = inst.set_input(name, *v);
                    }
                    let mut x = inst.states().to_vec();
                    for (&i, &v) in &self.states {
                        x[i] = v;
                    }
                    if let Err(e) = inst.set_states(&x) {
                        eprintln!("globe-mode-fmu: {e}");
                        return false;
                    }
                    self.inst = inst
                }
                Err(d) => {
                    eprintln!("globe-mode-fmu: {d:?}");
                    return false;
                }
            }
            self.dirty = false;
        }
        true
    }
}

impl Instance {
    /// CS: 初期化の後、次の時刻事象と事象指示関数の最初の値を控える（ME の `fmi2NewDiscreteStates` の最初の呼び出しに当たる）。
    fn start_cs(&mut self) -> bool {
        if !self.cs {
            return true;
        }
        let r = self.inst.update_discrete_states().and_then(|u| {
            self.t_next = u.next_event_time;
            self.z_prev = self.inst.event_indicators()?;
            Ok(())
        });
        r.map_err(|e| eprintln!("globe-mode-fmu: {e}")).is_ok()
    }
}

/// CS の内部の 1 刻みの上限。
// shortcut: 固定刻みの RK4 で、誤差の制御は無い（`globe-mode-fmi-import` の ode.rs は Rumoca を引かないよう pub(crate) で分けてあり、
// 共有ライブラリの読み込みも引くので使えない）。硬い系・速い系で足りなければ上限を小さくするか陰的な積分に替える。
const CS_MAX_SUBSTEP: f64 = 1e-3;
/// 同じ時刻に事象が止まらない（Zeno）と見なす回数。
const MAX_EVENTS_AT_ONE_TIME: usize = 100;

/// 古典 RK4 の 1 歩（`t` から `h`）。
fn rk4(inst: &mut ModelInstance, t: f64, x: &[f64], h: f64) -> Result<Vec<f64>, String> {
    let mut f = |t: f64, x: &[f64]| -> Result<Vec<f64>, String> {
        inst.set_time(t);
        inst.set_states(x)?;
        inst.derivatives()
    };
    let at = |k: &[f64], c: f64| -> Vec<f64> { x.iter().zip(k).map(|(x, k)| x + c * k).collect() };
    let k1 = f(t, x)?;
    let k2 = f(t + h / 2.0, &at(&k1, h / 2.0))?;
    let k3 = f(t + h / 2.0, &at(&k2, h / 2.0))?;
    let k4 = f(t + h, &at(&k3, h))?;
    let x1: Vec<f64> = (0..x.len())
        .map(|i| x[i] + h / 6.0 * (k1[i] + 2.0 * k2[i] + 2.0 * k3[i] + k4[i]))
        .collect();
    if x1.iter().any(|v| !v.is_finite()) {
        return Err(format!("t = {t} からの 1 歩で状態が有限でなくなった"));
    }
    Ok(x1)
}

/// CS の 1 歩 `[t0, t0 + h]`。ME と同じ事象処理（状態事象は二分法で詰め、時刻事象は刻みを区切る）を中で回す。
/// `Ok(true)` はモデルの `terminate()` で、そのとき時刻は事象の時刻で止まる。
fn do_step(m: &mut Instance, t0: f64, h: f64) -> Result<bool, String> {
    if !m.cs {
        return Err("Co-Simulation で作った instance ではない".into());
    }
    if (m.inst.time() - t0).abs() > 1e-9 * t0.abs().max(1.0) {
        return Err(format!(
            "currentCommunicationPoint {t0} が今の時刻 {} と違う（巻き戻しは未対応）",
            m.inst.time()
        ));
    }
    let domain_changed =
        |a: &[f64], b: &[f64]| a.iter().zip(b).any(|(a, b)| (*a > 0.0) != (*b > 0.0));
    let (t_end, mut t, mut x, mut here) = (t0 + h, t0, m.inst.states().to_vec(), 0usize);
    while t < t_end {
        let t_lim = m.t_next.map_or(t_end, |tn| tn.min(t_end));
        let mut t1 = if t_lim <= t {
            t
        } else {
            (t + CS_MAX_SUBSTEP).min(t_lim)
        };
        let mut x1 = if t1 > t && !x.is_empty() {
            rk4(&mut m.inst, t, &x, t1 - t)?
        } else {
            x.clone()
        };
        m.inst.set_time(t1);
        m.inst.set_states(&x1)?;
        let z1 = m.inst.event_indicators()?;
        let state_event = domain_changed(&m.z_prev, &z1);
        if state_event {
            // 領域が変わった最初の時刻を二分法で詰める（hi は変わった側）
            let (mut lo, mut hi) = (t, t1);
            while hi - lo > 1e-12 * hi.abs().max(1.0) {
                let mid = 0.5 * (lo + hi);
                let xm = if x.is_empty() {
                    Vec::new()
                } else {
                    rk4(&mut m.inst, t, &x, mid - t)?
                };
                m.inst.set_time(mid);
                m.inst.set_states(&xm)?;
                if domain_changed(&m.z_prev, &m.inst.event_indicators()?) {
                    (hi, x1) = (mid, xm);
                } else {
                    lo = mid;
                }
            }
            t1 = hi;
            m.inst.set_time(t1);
            m.inst.set_states(&x1)?;
        }
        let time_event = m.t_next.is_some_and(|tn| t1 >= tn);
        here = if t1 == t { here + 1 } else { 0 };
        (t, x) = (t1, x1);
        if state_event || time_event {
            if here > MAX_EVENTS_AT_ONE_TIME {
                return Err(format!("t = {t} で事象が止まらない（Zeno かもしれない）"));
            }
            m.inst.enter_event_mode()?;
            let u = m.inst.update_discrete_states()?;
            m.t_next = u.next_event_time;
            if u.terminate {
                return Ok(true);
            }
            x = m.inst.states().to_vec();
            m.z_prev = m.inst.event_indicators()?;
        } else {
            m.z_prev = z1;
        }
    }
    Ok(false)
}

/// `resources` の場所の URI（またはそのままのパス）をパスの文字列にする。
fn resource_path(location: &str) -> &str {
    let p = location
        .strip_prefix("file:")
        .map_or(location, |p| p.strip_prefix("//").unwrap_or(p));
    // Windows の `/C:/x` は `C:/x`
    match p.as_bytes() {
        [b'/', d, b':', ..] if d.is_ascii_alphabetic() => &p[1..],
        _ => p,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// #51: Integer・Boolean の入力の start は XSD の型（xs:int・xs:boolean）で書く（`2.0` ではなく `2`、`true`）。
    #[test]
    fn integer_and_boolean_inputs_start_in_their_own_types() {
        let src = "model M\n  input Integer n(start = 2);\n  input Boolean b(start = true);\n  Real x(start = 1);\nequation\n  der(x) = -x;\nend M;\n";
        for xml in [
            model_description("M", src).unwrap(),
            model_description_3("M", src).unwrap(),
        ] {
            assert!(
                xml.contains("start=\"2\"") && xml.contains("start=\"true\""),
                "{xml}"
            );
        }
    }

    /// S237: noEvent でない関係式（`abs(w) >= 1`）の記憶は、初期化の点（w = 100）から決める。無いと既定の false の枝
    /// （`w`、微分 -100·100）で始まり、最初の事象まで微分が違う。
    #[test]
    fn relation_memory_starts_from_the_initial_point() {
        let src = "model M\n  Real w(start = 100, fixed = true);\nequation\n  der(w) = -100 * (if abs(w) >= 1 then sign(w) else w);\nend M;\n";
        let model = compile("M", src).unwrap();
        let inst = ModelInstance::new(&model, &BTreeMap::new(), 0.0).unwrap();
        assert_eq!(inst.derivatives().unwrap(), [-100.0]);
    }

    #[test]
    fn resource_path_from_uri() {
        assert_eq!(resource_path("file:///home/u/r"), "/home/u/r");
        assert_eq!(resource_path("/home/u/r"), "/home/u/r");
        // Windows: ドライブ文字の前の / を落とす（FMPy は file:///C:/… を渡す）
        assert_eq!(resource_path("file:///C:/Users/u/r"), "C:/Users/u/r");
        assert_eq!(resource_path("file:///c:/r"), "c:/r");
        // 3.0 の resourcePath はそのままのパス
        assert_eq!(resource_path(r"C:\Users\u\r"), r"C:\Users\u\r");
    }
}

/// `resources` の場所（2.0 は `file:///…` の URI、3.0 はそのままのパス）から、ただ 1 つの `.mo` の
/// （完全修飾名、文面、`resources/lib` があればそのパス）を読む。`lib` は MSL などのライブラリの置き場（S237）。
// shortcut: %xx のエスケープを戻さない。空白などを含む置き場で使うときに戻す
fn read_resource(location: &str) -> Option<(String, String, Option<PathBuf>)> {
    let dir = PathBuf::from(resource_path(location));
    let mo = std::fs::read_dir(&dir)
        .ok()?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .find(|p| p.extension().is_some_and(|x| x == "mo"))?;
    let name = Path::new(mo.file_stem()?).to_str()?.to_string();
    let lib = Some(dir.join("lib")).filter(|l| l.is_dir());
    Some((name, std::fs::read_to_string(&mo).ok()?, lib))
}

/// FMI 2.0 の `fmi2EventInfo`。
#[repr(C)]
#[derive(Debug)]
pub struct EventInfo {
    pub new_discrete_states_needed: c_int,
    pub terminate_simulation: c_int,
    pub nominals_of_continuous_states_changed: c_int,
    pub values_of_continuous_states_changed: c_int,
    pub next_event_time_defined: c_int,
    pub next_event_time: f64,
}

unsafe fn inst<'a>(c: Component) -> Option<&'a mut Instance> {
    // SAFETY: c は fmi2Instantiate が返し、まだ放していないもの（規格の約束）
    unsafe { c.cast::<Instance>().as_mut() }
}

/// `nvr` 個の値参照と値の配列を組にして `f` に渡す。値参照が範囲外なら `fmi2Error`。
unsafe fn each<T>(
    c: Component,
    vr: *const c_uint,
    nvr: usize,
    value: *mut T,
    mut f: impl FnMut(&mut Instance, c_uint, &mut T) -> bool,
) -> Status {
    if nvr == 0 {
        return OK;
    }
    let Some(m) = (unsafe { inst(c) }) else {
        return ERROR;
    };
    if vr.is_null() || value.is_null() {
        return ERROR;
    }
    // SAFETY: 呼び手は nvr 個の有効な配列を渡す
    let (vr, value) = unsafe {
        (
            std::slice::from_raw_parts(vr, nvr),
            std::slice::from_raw_parts_mut(value, nvr),
        )
    };
    if vr.iter().zip(value).all(|(&r, v)| f(m, r, v)) {
        OK
    } else {
        ERROR
    }
}

/// 長さ `n`（状態の数）の配列を `f` に渡す。`f` が失敗したら `fmi2Error`。
unsafe fn states(
    c: Component,
    p: *mut f64,
    n: usize,
    f: impl FnOnce(&mut Instance, &mut [f64]) -> Result<(), String>,
) -> Status {
    let Some(m) = (unsafe { inst(c) }) else {
        return ERROR;
    };
    if n != m.inst.n_states() || (n > 0 && p.is_null()) {
        return ERROR;
    }
    let v: &mut [f64] = if n == 0 {
        &mut []
    } else {
        // SAFETY: 呼び手は n 個の有効な配列を渡す
        unsafe { std::slice::from_raw_parts_mut(p, n) }
    };
    match f(m, v) {
        Ok(()) => OK,
        Err(e) => {
            eprintln!("globe-mode-fmu: {e}");
            ERROR
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn fmi2GetTypesPlatform() -> *const c_char {
    c"default".as_ptr()
}

#[unsafe(no_mangle)]
pub extern "C" fn fmi2GetVersion() -> *const c_char {
    c"2.0".as_ptr()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fmi2SetDebugLogging(
    _c: Component,
    _on: c_int,
    _n: usize,
    _cats: *const *const c_char,
) -> Status {
    OK
}

/// ME と CS を受け（S231）、resources の `.mo` を compile する。`guid` が文面と合わないか、載せられないモデルなら NULL を返す。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fmi2Instantiate(
    _instance_name: *const c_char,
    fmu_type: c_int,
    guid: *const c_char,
    resource_location: *const c_char,
    _functions: *const c_void,
    _visible: c_int,
    _logging_on: c_int,
) -> Component {
    if fmu_type != MODEL_EXCHANGE && fmu_type != CO_SIMULATION {
        return std::ptr::null_mut();
    }
    unsafe { instantiate(guid, resource_location, fmu_type == CO_SIMULATION) }
}

/// `fmi2Instantiate`・`fmi3InstantiateModelExchange`・`fmi3InstantiateCoSimulation` の中身。
unsafe fn instantiate(
    guid: *const c_char,
    resource_location: *const c_char,
    cs: bool,
) -> Component {
    if guid.is_null() || resource_location.is_null() {
        return std::ptr::null_mut();
    }
    // SAFETY: guid と resource_location は NUL 終端の文字列（規格の約束）
    let (guid_given, location) = unsafe {
        (
            CStr::from_ptr(guid).to_string_lossy(),
            CStr::from_ptr(resource_location).to_string_lossy(),
        )
    };
    let Some((name, source, lib)) = read_resource(&location) else {
        eprintln!("globe-mode-fmu: {location} に .mo が無い");
        return std::ptr::null_mut();
    };
    if self::guid(&name, &source) != guid_given {
        return std::ptr::null_mut();
    }
    let built = compile_in(&name, &source, lib.as_deref()).and_then(|model| {
        ModelInstance::new(&model, &BTreeMap::new(), 0.0)
            .map(|inst| (model, inst))
            .map_err(|d| format!("{d:?}"))
    });
    match built {
        Ok((model, inst)) => {
            let strings = model
                .string_parameters()
                .into_iter()
                .map(|(n, v)| (n, std::ffi::CString::new(v).unwrap_or_default()))
                .collect();
            Box::into_raw(Box::new(Instance {
                model,
                inst,
                start: 0.0,
                overrides: BTreeMap::new(),
                inputs: BTreeMap::new(),
                states: BTreeMap::new(),
                strings,
                dirty: false,
                cs,
                se: false,
                activated: false,
                t_next: None,
                z_prev: Vec::new(),
            }))
            .cast()
        }
        Err(e) => {
            eprintln!("globe-mode-fmu: {e}");
            std::ptr::null_mut()
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fmi2FreeInstance(c: Component) {
    if !c.is_null() {
        // SAFETY: fmi2Instantiate の Box::into_raw を 1 度だけ戻す
        drop(unsafe { Box::from_raw(c.cast::<Instance>()) });
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fmi2SetupExperiment(
    c: Component,
    _tol_defined: c_int,
    _tol: f64,
    start: f64,
    _stop_defined: c_int,
    _stop: f64,
) -> Status {
    unsafe { inst(c) }.map_or(ERROR, |m| {
        if m.start != start {
            (m.start, m.dirty) = (start, true);
        }
        OK
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fmi2EnterInitializationMode(c: Component) -> Status {
    if unsafe { inst(c) }.is_some() {
        OK
    } else {
        ERROR
    }
}

/// パラメータか開始時刻が変わっていれば、lowering からやり直して初期化し直す。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fmi2ExitInitializationMode(c: Component) -> Status {
    unsafe { inst(c) }.map_or(ERROR, |m| {
        if m.rebuild() && m.start_cs() {
            OK
        } else {
            ERROR
        }
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fmi2Terminate(c: Component) -> Status {
    unsafe { fmi2EnterInitializationMode(c) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fmi2Reset(c: Component) -> Status {
    unsafe { inst(c) }.map_or(ERROR, |m| {
        m.overrides.clear();
        m.inputs.clear();
        m.states.clear();
        (m.start, m.dirty) = (0.0, true);
        if m.rebuild() { OK } else { ERROR }
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fmi2GetReal(
    c: Component,
    vr: *const c_uint,
    nvr: usize,
    value: *mut f64,
) -> Status {
    unsafe {
        each(c, vr, nvr, value, |m, r, v| {
            let (n, r) = (m.inst.n_states(), r as usize);
            let got = if r < n {
                Some(m.inst.states()[r])
            } else if r < 2 * n {
                m.inst.derivatives().ok().map(|d| d[r - n])
            } else if let Some((name, p)) = m.inst.parameters().get(r - 2 * n) {
                Some(*m.overrides.get(name).unwrap_or(p))
            } else {
                variable(m, r, ValueType::Real)
            };
            got.map(|x| *v = x).is_some()
        })
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fmi2SetReal(
    c: Component,
    vr: *const c_uint,
    nvr: usize,
    value: *const f64,
) -> Status {
    unsafe {
        each(c, vr, nvr, value.cast_mut(), |m, r, v| {
            let (n, r) = (m.inst.n_states(), r as usize);
            if r < n {
                let mut x = m.inst.states().to_vec();
                x[r] = *v;
                let ok = m.inst.set_states(&x).is_ok();
                if ok {
                    m.states.insert(r, *v);
                }
                return ok;
            }
            let p = m.inst.parameters().len();
            if let Some(var) = (r.checked_sub(2 * n + p)).and_then(|i| m.inst.variables().nth(i)) {
                let name = var.name.clone();
                let ok = var.role == VariableRole::Input && m.inst.set_input(&name, *v).is_ok();
                if ok {
                    m.inputs.insert(name, *v);
                }
                return ok;
            }
            let Some((name, _)) = r
                .checked_sub(2 * n)
                .and_then(|i| m.inst.parameters().get(i))
            else {
                return false;
            };
            m.overrides.insert(name.clone(), *v);
            m.dirty = true;
            true
        })
    }
}

/// 値参照 `r`（`2n+p..`）の代数・離散の変数の今の値。型が `ty` でなければ None。
// shortcut: 値参照ごとに解き直す（nvr 個なら nvr 回）。変数の多いモデルで遅ければ 1 回の呼び出しの中で使い回す
fn variable(m: &Instance, r: usize, ty: ValueType) -> Option<f64> {
    let i = r.checked_sub(2 * m.inst.n_states() + m.inst.parameters().len())?;
    // その後ろは 3.0 の EventIndicator の変数と time（S137）
    if let Some(k) = i.checked_sub(m.inst.variables().count()) {
        if ty != ValueType::Real {
            return None;
        }
        if k == m.inst.n_event_indicators() {
            return Some(m.inst.time());
        }
        return m.inst.event_indicators().ok()?.get(k).copied();
    }
    let vt = m.inst.variables().nth(i)?.value_type;
    // Enumeration は FMI では Integer
    if vt != ty && !(ty == ValueType::Integer && vt == ValueType::Enumeration) {
        return None;
    }
    match m.inst.variable_values() {
        Ok(v) => Some(v[i]),
        Err(e) => {
            eprintln!("globe-mode-fmu: {e}");
            None
        }
    }
}

/// Boolean の書き込みと、Integer の書き込みのうち Enumeration でないものは無い（`nvr = 0` だけ OK）。
fn no_variables(nvr: usize) -> Status {
    if nvr == 0 { OK } else { ERROR }
}

/// 値参照 `r` の Integer（Enumeration を含む）の今の値。
fn integer(m: &Instance, r: usize) -> Option<i64> {
    let n = m.inst.n_states();
    // Enumeration の tunable パラメータは P の枠の値（ordinal）を読む
    let param = r
        .checked_sub(2 * n)
        .and_then(|i| m.inst.parameters().get(i))
        .filter(|(name, _)| m.model.enum_type_of(name).is_some());
    // 値は整数の f64（Rumoca は離散の値も f64 で持つ）
    let got = match param {
        Some((name, p)) => Some(*m.overrides.get(name).unwrap_or(p)),
        None => variable(m, r, ValueType::Integer),
    };
    got.map(|x| x.round() as i64)
}

/// Enumeration の tunable パラメータか入力 `r` に ordinal `v`（1 以上リテラルの数以下）を書く。
fn set_enum(m: &mut Instance, r: usize, v: i64) -> bool {
    let Some((name, input)) = enum_slot(m, r) else {
        return false;
    };
    let count = m
        .model
        .enum_type_of(&name)
        .map_or(i64::MAX, |t| t.literals.len() as i64);
    if !(1..=count).contains(&v) {
        eprintln!("globe-mode-fmu: {name} = {v} は Enumeration の範囲（1..={count}）の外");
        return false;
    }
    let x = v as f64;
    if input {
        let ok = m.inst.set_input(&name, x).is_ok();
        if ok {
            m.inputs.insert(name, x);
        }
        return ok;
    }
    m.overrides.insert(name, x);
    m.dirty = true;
    true
}

/// 値参照 `r` が Enumeration の tunable パラメータか入力なら、その名前と、入力かどうか。
fn enum_slot(m: &Instance, r: usize) -> Option<(String, bool)> {
    let (n, p) = (m.inst.n_states(), m.inst.parameters().len());
    if let Some((name, _)) = r
        .checked_sub(2 * n)
        .and_then(|i| m.inst.parameters().get(i))
    {
        return m.model.enum_type_of(name).map(|_| (name.clone(), false));
    }
    let v = r
        .checked_sub(2 * n + p)
        .and_then(|i| m.inst.variables().nth(i))?;
    (v.value_type == ValueType::Enumeration && v.role == VariableRole::Input)
        .then(|| (v.name.clone(), true))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fmi2GetInteger(
    c: Component,
    vr: *const c_uint,
    nvr: usize,
    value: *mut c_int,
) -> Status {
    unsafe {
        each(c, vr, nvr, value, |m, r, v| {
            integer(m, r as usize).map(|x| *v = x as c_int).is_some()
        })
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fmi2GetBoolean(
    c: Component,
    vr: *const c_uint,
    nvr: usize,
    value: *mut c_int,
) -> Status {
    unsafe {
        each(c, vr, nvr, value, |m, r, v| {
            variable(m, r as usize, ValueType::Boolean)
                .map(|x| *v = c_int::from(x != 0.0))
                .is_some()
        })
    }
}

/// String のパラメータ（S224）を読む。ポインタは次の呼び出しまで（実際はインスタンスの寿命の間）有効。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fmi2GetString(
    c: Component,
    vr: *const c_uint,
    nvr: usize,
    v: *mut *const c_char,
) -> Status {
    unsafe {
        each(c, vr, nvr, v, |m, r, out| {
            let name = (r as usize)
                .checked_sub(2 * m.inst.n_states())
                .and_then(|i| m.inst.parameters().get(i));
            let s = name.and_then(|(n, _)| m.strings.get(n));
            s.map(|s| *out = s.as_ptr()).is_some()
        })
    }
}

/// Enumeration の tunable パラメータと入力を書く（S224。ordinal は 1 以上リテラルの数以下）。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fmi2SetInteger(
    c: Component,
    vr: *const c_uint,
    nvr: usize,
    v: *const c_int,
) -> Status {
    unsafe {
        each(c, vr, nvr, v.cast_mut(), |m, r, v| {
            set_enum(m, r as usize, i64::from(*v))
        })
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fmi2SetBoolean(
    _c: Component,
    _vr: *const c_uint,
    nvr: usize,
    _v: *const c_int,
) -> Status {
    no_variables(nvr)
}

/// String のパラメータは `variability="fixed"` で、`fmi2Instantiate` で .mo から決まる値を変えられない。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fmi2SetString(
    _c: Component,
    _vr: *const c_uint,
    nvr: usize,
    _v: *const *const c_char,
) -> Status {
    if nvr > 0 {
        eprintln!("globe-mode-fmu: String の値は書けない（String のパラメータは fixed）");
    }
    no_variables(nvr)
}

// canGetAndSetFMUstate・canSerializeFMUstate・providesDirectionalDerivative は false（modelDescription.xml の既定）
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fmi2GetFMUstate(_c: Component, _s: *mut *mut c_void) -> Status {
    ERROR
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fmi2SetFMUstate(_c: Component, _s: *mut c_void) -> Status {
    ERROR
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fmi2FreeFMUstate(_c: Component, _s: *mut *mut c_void) -> Status {
    ERROR
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fmi2SerializedFMUstateSize(
    _c: Component,
    _s: *mut c_void,
    _size: *mut usize,
) -> Status {
    ERROR
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fmi2SerializeFMUstate(
    _c: Component,
    _s: *mut c_void,
    _bytes: *mut c_char,
    _size: usize,
) -> Status {
    ERROR
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fmi2DeSerializeFMUstate(
    _c: Component,
    _bytes: *const c_char,
    _size: usize,
    _s: *mut *mut c_void,
) -> Status {
    ERROR
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fmi2GetDirectionalDerivative(
    _c: Component,
    _unknown: *const c_uint,
    _n_unknown: usize,
    _known: *const c_uint,
    _n_known: usize,
    _dv_known: *const f64,
    _dv_unknown: *mut f64,
) -> Status {
    ERROR
}

/// 今の時刻と状態を `pre` の値として控える。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fmi2EnterEventMode(c: Component) -> Status {
    unsafe { inst(c) }.map_or(ERROR, |m| match m.inst.enter_event_mode() {
        Ok(()) => OK,
        Err(e) => {
            eprintln!("globe-mode-fmu: {e}");
            ERROR
        }
    })
}

/// `when` と `reinit` を回す。Rumoca は 1 回の呼び出しで事象の反復を収束させるので、
/// `newDiscreteStatesNeeded` はいつも偽。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fmi2NewDiscreteStates(c: Component, info: *mut EventInfo) -> Status {
    let (Some(m), Some(info)) = (unsafe { inst(c) }, unsafe { info.as_mut() }) else {
        return ERROR;
    };
    match m.inst.update_discrete_states() {
        Ok(u) => {
            *info = EventInfo {
                new_discrete_states_needed: 0,
                terminate_simulation: c_int::from(u.terminate),
                nominals_of_continuous_states_changed: 0,
                values_of_continuous_states_changed: c_int::from(u.states_changed),
                next_event_time_defined: c_int::from(u.next_event_time.is_some()),
                next_event_time: u.next_event_time.unwrap_or(0.0),
            };
            OK
        }
        Err(e) => {
            eprintln!("globe-mode-fmu: {e}");
            ERROR
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fmi2EnterContinuousTimeMode(c: Component) -> Status {
    unsafe { fmi2EnterInitializationMode(c) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fmi2CompletedIntegratorStep(
    c: Component,
    _no_set_prior: c_int,
    enter_event_mode: *mut c_int,
    terminate: *mut c_int,
) -> Status {
    match (
        unsafe { inst(c) },
        unsafe { enter_event_mode.as_mut() },
        unsafe { terminate.as_mut() },
    ) {
        (Some(_), Some(e), Some(t)) => {
            (*e, *t) = (0, 0);
            OK
        }
        _ => ERROR,
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fmi2SetTime(c: Component, time: f64) -> Status {
    unsafe { inst(c) }.map_or(ERROR, |m| {
        m.inst.set_time(time);
        OK
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fmi2SetContinuousStates(c: Component, x: *const f64, nx: usize) -> Status {
    unsafe { states(c, x.cast_mut(), nx, |m, v| m.inst.set_states(v)) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fmi2GetContinuousStates(c: Component, x: *mut f64, nx: usize) -> Status {
    unsafe {
        states(c, x, nx, |m, v| {
            v.copy_from_slice(m.inst.states());
            Ok(())
        })
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fmi2GetDerivatives(c: Component, dx: *mut f64, nx: usize) -> Status {
    unsafe {
        states(c, dx, nx, |m, v| {
            v.copy_from_slice(&m.inst.derivatives()?);
            Ok(())
        })
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fmi2GetNominalsOfContinuousStates(
    c: Component,
    x: *mut f64,
    nx: usize,
) -> Status {
    unsafe {
        states(c, x, nx, |_, v| {
            v.fill(1.0);
            Ok(())
        })
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fmi2GetEventIndicators(c: Component, z: *mut f64, ni: usize) -> Status {
    let Some(m) = (unsafe { inst(c) }) else {
        return ERROR;
    };
    if ni != m.inst.n_event_indicators() || (ni > 0 && z.is_null()) {
        return ERROR;
    }
    if ni == 0 {
        return OK;
    }
    match m.inst.event_indicators() {
        Ok(v) => {
            // SAFETY: 呼び手は ni 個の有効な配列を渡す
            unsafe { std::slice::from_raw_parts_mut(z, ni) }.copy_from_slice(&v);
            OK
        }
        Err(e) => {
            eprintln!("globe-mode-fmu: {e}");
            ERROR
        }
    }
}

/// 1 歩進める（S231）。モデルの `terminate()` は `fmi2Discard` + `fmi2GetBooleanStatus` を実装していないので `fmi2Error`。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fmi2DoStep(
    c: Component,
    current_communication_point: f64,
    communication_step_size: f64,
    _no_set_fmu_state_prior: c_int,
) -> Status {
    let Some(m) = (unsafe { inst(c) }) else {
        return ERROR;
    };
    match do_step(m, current_communication_point, communication_step_size) {
        Ok(false) => OK,
        Ok(true) => {
            eprintln!("globe-mode-fmu: モデルが terminate() した（FMI 2.0 の CS では未対応）");
            ERROR
        }
        Err(e) => {
            eprintln!("globe-mode-fmu: {e}");
            ERROR
        }
    }
}

// ---- FMI 3.0 の ME（S137）。2.0 の関数を形だけ変えて呼ぶ（fmi3Boolean は C の bool） ----

#[unsafe(no_mangle)]
pub extern "C" fn fmi3GetVersion() -> *const c_char {
    c"3.0".as_ptr()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fmi3SetDebugLogging(
    _c: Component,
    _on: bool,
    _n: usize,
    _cats: *const *const c_char,
) -> Status {
    OK
}

/// resources の `.mo` を compile する（2.0 の `fmi2Instantiate` と同じ。`resourcePath` はそのままのパス）。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fmi3InstantiateModelExchange(
    _instance_name: *const c_char,
    instantiation_token: *const c_char,
    resource_path: *const c_char,
    _visible: bool,
    _logging_on: bool,
    _environment: *mut c_void,
    _log_message: *const c_void,
) -> Component {
    unsafe { instantiate(instantiation_token, resource_path, false) }
}

/// 3.0 の CS（S231）。`eventModeUsed`・`earlyReturnAllowed` は使わない（事象は刻みの中で回し、途中では返らない）。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fmi3InstantiateCoSimulation(
    _instance_name: *const c_char,
    instantiation_token: *const c_char,
    resource_path: *const c_char,
    _visible: bool,
    _logging_on: bool,
    _event_mode_used: bool,
    _early_return_allowed: bool,
    _required_intermediate_variables: *const c_uint,
    _n_required_intermediate_variables: usize,
    _environment: *mut c_void,
    _log_message: *const c_void,
    _intermediate_update: *const c_void,
) -> Component {
    unsafe { instantiate(instantiation_token, resource_path, true) }
}

/// 3.0 の SE（S251）。クロックの更新・割り込み禁止の callback は使わない（パーティションは 1 つで、`ActivateModelPartition` の中で
/// 完結するので、`fmi3ClockUpdateCallback` を呼ぶ出力クロックも、守る区間も無い）。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fmi3InstantiateScheduledExecution(
    _instance_name: *const c_char,
    instantiation_token: *const c_char,
    resource_path: *const c_char,
    _visible: bool,
    _logging_on: bool,
    _environment: *mut c_void,
    _log_message: *const c_void,
    _clock_update: *const c_void,
    _lock_preemption: *const c_void,
    _unlock_preemption: *const c_void,
) -> Component {
    let c = unsafe { instantiate(instantiation_token, resource_path, true) };
    if let Some(m) = unsafe { inst(c) } {
        m.se = true;
    }
    c
}

/// 入力クロック（パーティション）を `activationTime` で動かす（S251）。前の時刻（初めは開始時刻）から `activationTime` まで進める。
/// 時刻は狭義単調増加（最初の 1 回だけ開始時刻と同じでよい）。モデルの `terminate()` は SE に伝える手段が無いので `fmi3Error`。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fmi3ActivateModelPartition(
    c: Component,
    clock_reference: c_uint,
    activation_time: f64,
) -> Status {
    let Some(m) = (unsafe { inst(c) }) else {
        return ERROR;
    };
    let now = m.inst.time();
    if !m.se
        || clock_reference != clock_ref(&m.inst)
        || activation_time < now
        || (m.activated && activation_time == now)
    {
        eprintln!(
            "globe-mode-fmu: fmi3ActivateModelPartition({clock_reference}, {activation_time}) は受けられない（時刻 {now}）"
        );
        return ERROR;
    }
    m.activated = true;
    match do_step(m, now, activation_time - now) {
        Ok(false) => OK,
        Ok(true) => {
            eprintln!("globe-mode-fmu: モデルが terminate() した（SE では未対応）");
            ERROR
        }
        Err(e) => {
            eprintln!("globe-mode-fmu: {e}");
            ERROR
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fmi3FreeInstance(c: Component) {
    unsafe { fmi2FreeInstance(c) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fmi3EnterInitializationMode(
    c: Component,
    tol_defined: bool,
    tol: f64,
    start: f64,
    stop_defined: bool,
    stop: f64,
) -> Status {
    unsafe { fmi2SetupExperiment(c, tol_defined.into(), tol, start, stop_defined.into(), stop) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fmi3ExitInitializationMode(c: Component) -> Status {
    unsafe { fmi2ExitInitializationMode(c) }
}

/// CS の Step Mode に入る（`eventModeUsed` を使わないので、初期化の直後から Step Mode のまま。何もしない）。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fmi3EnterStepMode(c: Component) -> Status {
    if unsafe { inst(c) }.is_some() {
        OK
    } else {
        ERROR
    }
}

/// 1 歩進める（S231）。事象は刻みの中で回すので `eventHandlingNeeded`・`earlyReturn` はいつも偽。`terminateSimulation` は
/// モデルの `terminate()` のとき真（`lastSuccessfulTime` はその時刻）。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fmi3DoStep(
    c: Component,
    current_communication_point: f64,
    communication_step_size: f64,
    _no_set_fmu_state_prior: bool,
    event_handling_needed: *mut bool,
    terminate_simulation: *mut bool,
    early_return: *mut bool,
    last_successful_time: *mut f64,
) -> Status {
    let (Some(m), Some(e), Some(t), Some(r), Some(l)) = (
        unsafe { inst(c) },
        unsafe { event_handling_needed.as_mut() },
        unsafe { terminate_simulation.as_mut() },
        unsafe { early_return.as_mut() },
        unsafe { last_successful_time.as_mut() },
    ) else {
        return ERROR;
    };
    match do_step(m, current_communication_point, communication_step_size) {
        Ok(terminated) => {
            (*e, *t, *r) = (false, terminated, false);
            *l = m.inst.time();
            OK
        }
        Err(msg) => {
            eprintln!("globe-mode-fmu: {msg}");
            ERROR
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fmi3EnterEventMode(c: Component) -> Status {
    unsafe { fmi2EnterEventMode(c) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fmi3Terminate(c: Component) -> Status {
    unsafe { fmi2Terminate(c) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fmi3Reset(c: Component) -> Status {
    unsafe { fmi2Reset(c) }
}

/// スカラーだけなので `nValues` は `nvr` と同じでなければ `fmi3Error`。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fmi3GetFloat64(
    c: Component,
    vr: *const c_uint,
    nvr: usize,
    values: *mut f64,
    n_values: usize,
) -> Status {
    if n_values != nvr {
        return ERROR;
    }
    unsafe { fmi2GetReal(c, vr, nvr, values) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fmi3SetFloat64(
    c: Component,
    vr: *const c_uint,
    nvr: usize,
    values: *const f64,
    n_values: usize,
) -> Status {
    if n_values != nvr {
        return ERROR;
    }
    unsafe { fmi2SetReal(c, vr, nvr, values) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fmi3GetInt32(
    c: Component,
    vr: *const c_uint,
    nvr: usize,
    values: *mut i32,
    n_values: usize,
) -> Status {
    if n_values != nvr {
        return ERROR;
    }
    unsafe { fmi2GetInteger(c, vr, nvr, values) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fmi3SetInt32(
    c: Component,
    vr: *const c_uint,
    nvr: usize,
    values: *const i32,
    n_values: usize,
) -> Status {
    if n_values != nvr {
        return ERROR;
    }
    unsafe { fmi2SetInteger(c, vr, nvr, values) }
}

/// 3.0 の Enumeration は Int64 の getter・setter で読み書きする。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fmi3GetInt64(
    c: Component,
    vr: *const c_uint,
    nvr: usize,
    values: *mut i64,
    n_values: usize,
) -> Status {
    if n_values != nvr {
        return ERROR;
    }
    unsafe {
        each(c, vr, nvr, values, |m, r, v| {
            integer(m, r as usize).map(|x| *v = x).is_some()
        })
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fmi3SetInt64(
    c: Component,
    vr: *const c_uint,
    nvr: usize,
    values: *const i64,
    n_values: usize,
) -> Status {
    if n_values != nvr {
        return ERROR;
    }
    unsafe {
        each(c, vr, nvr, values.cast_mut(), |m, r, v| {
            set_enum(m, r as usize, *v)
        })
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fmi3GetString(
    c: Component,
    vr: *const c_uint,
    nvr: usize,
    values: *mut *const c_char,
    n_values: usize,
) -> Status {
    if n_values != nvr {
        return ERROR;
    }
    unsafe { fmi2GetString(c, vr, nvr, values) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fmi3SetString(
    c: Component,
    vr: *const c_uint,
    nvr: usize,
    values: *const *const c_char,
    n_values: usize,
) -> Status {
    if n_values != nvr {
        return ERROR;
    }
    unsafe { fmi2SetString(c, vr, nvr, values) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fmi3GetBoolean(
    c: Component,
    vr: *const c_uint,
    nvr: usize,
    values: *mut bool,
    n_values: usize,
) -> Status {
    if n_values != nvr {
        return ERROR;
    }
    unsafe {
        each(c, vr, nvr, values, |m, r, v| {
            variable(m, r as usize, ValueType::Boolean)
                .map(|x| *v = x != 0.0)
                .is_some()
        })
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fmi3UpdateDiscreteStates(
    c: Component,
    discrete_states_need_update: *mut bool,
    terminate_simulation: *mut bool,
    nominals_changed: *mut bool,
    values_changed: *mut bool,
    next_event_time_defined: *mut bool,
    next_event_time: *mut f64,
) -> Status {
    let mut info = EventInfo {
        new_discrete_states_needed: 0,
        terminate_simulation: 0,
        nominals_of_continuous_states_changed: 0,
        values_of_continuous_states_changed: 0,
        next_event_time_defined: 0,
        next_event_time: 0.0,
    };
    let status = unsafe { fmi2NewDiscreteStates(c, &mut info) };
    // SAFETY: 呼び手は有効な（または NULL の）ポインタを渡す
    let outs = unsafe {
        [
            (
                discrete_states_need_update.as_mut(),
                info.new_discrete_states_needed,
            ),
            (terminate_simulation.as_mut(), info.terminate_simulation),
            (
                nominals_changed.as_mut(),
                info.nominals_of_continuous_states_changed,
            ),
            (
                values_changed.as_mut(),
                info.values_of_continuous_states_changed,
            ),
            (
                next_event_time_defined.as_mut(),
                info.next_event_time_defined,
            ),
        ]
    };
    for (p, v) in outs {
        if let Some(p) = p {
            *p = v != 0;
        }
    }
    if let Some(t) = unsafe { next_event_time.as_mut() } {
        *t = info.next_event_time;
    }
    status
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fmi3EnterContinuousTimeMode(c: Component) -> Status {
    unsafe { fmi2EnterContinuousTimeMode(c) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fmi3CompletedIntegratorStep(
    c: Component,
    _no_set_prior: bool,
    enter_event_mode: *mut bool,
    terminate: *mut bool,
) -> Status {
    let (mut e, mut t) = (0, 0);
    let status = unsafe { fmi2CompletedIntegratorStep(c, 0, &mut e, &mut t) };
    match unsafe { (enter_event_mode.as_mut(), terminate.as_mut()) } {
        (Some(pe), Some(pt)) if status == OK => {
            (*pe, *pt) = (e != 0, t != 0);
            OK
        }
        _ => ERROR,
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fmi3SetTime(c: Component, time: f64) -> Status {
    unsafe { fmi2SetTime(c, time) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fmi3SetContinuousStates(c: Component, x: *const f64, nx: usize) -> Status {
    unsafe { fmi2SetContinuousStates(c, x, nx) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fmi3GetContinuousStates(c: Component, x: *mut f64, nx: usize) -> Status {
    unsafe { fmi2GetContinuousStates(c, x, nx) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fmi3GetContinuousStateDerivatives(
    c: Component,
    dx: *mut f64,
    nx: usize,
) -> Status {
    unsafe { fmi2GetDerivatives(c, dx, nx) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fmi3GetNominalsOfContinuousStates(
    c: Component,
    x: *mut f64,
    nx: usize,
) -> Status {
    unsafe { fmi2GetNominalsOfContinuousStates(c, x, nx) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fmi3GetEventIndicators(c: Component, z: *mut f64, ni: usize) -> Status {
    unsafe { fmi2GetEventIndicators(c, z, ni) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fmi3GetNumberOfEventIndicators(c: Component, n: *mut usize) -> Status {
    match unsafe { (inst(c), n.as_mut()) } {
        (Some(m), Some(n)) => {
            *n = m.inst.n_event_indicators();
            OK
        }
        _ => ERROR,
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fmi3GetNumberOfContinuousStates(c: Component, n: *mut usize) -> Status {
    match unsafe { (inst(c), n.as_mut()) } {
        (Some(m), Some(n)) => {
            *n = m.inst.n_states();
            OK
        }
        _ => ERROR,
    }
}

/// 無い関数（ほかの型の値・FMUstate・方向微分・Clock・CS の微分と状態の問い合わせ）。引数を読まずに `fmi3Error` を返す
/// （呼び手が引数を片付ける C の呼び出し規約なので、引数を宣言しなくてよい）。
macro_rules! unsupported {
    ($($name:ident),* $(,)?) => {$(
        #[unsafe(no_mangle)]
        pub extern "C" fn $name() -> Status {
            ERROR
        }
    )*};
}

unsupported!(
    // FMI 2.0 の CS で、無いもの（入力の微分・出力の微分・取り消し・状態の問い合わせ）
    fmi2SetRealInputDerivatives,
    fmi2GetRealOutputDerivatives,
    fmi2CancelStep,
    fmi2GetStatus,
    fmi2GetRealStatus,
    fmi2GetIntegerStatus,
    fmi2GetBooleanStatus,
    fmi2GetStringStatus,
    fmi3GetFloat32,
    fmi3GetInt8,
    fmi3GetUInt8,
    fmi3GetInt16,
    fmi3GetUInt16,
    fmi3GetUInt32,
    fmi3GetUInt64,
    fmi3GetBinary,
    fmi3GetClock,
    fmi3SetFloat32,
    fmi3SetInt8,
    fmi3SetUInt8,
    fmi3SetInt16,
    fmi3SetUInt16,
    fmi3SetUInt32,
    fmi3SetUInt64,
    fmi3SetBoolean,
    fmi3SetBinary,
    fmi3SetClock,
    fmi3GetNumberOfVariableDependencies,
    fmi3GetVariableDependencies,
    fmi3GetFMUState,
    fmi3SetFMUState,
    fmi3FreeFMUState,
    fmi3SerializedFMUStateSize,
    fmi3SerializeFMUState,
    fmi3DeserializeFMUState,
    fmi3GetDirectionalDerivative,
    fmi3GetAdjointDerivative,
    fmi3EnterConfigurationMode,
    fmi3ExitConfigurationMode,
    fmi3GetIntervalDecimal,
    fmi3GetIntervalFraction,
    fmi3GetShiftDecimal,
    fmi3GetShiftFraction,
    fmi3SetIntervalDecimal,
    fmi3SetIntervalFraction,
    fmi3SetShiftDecimal,
    fmi3SetShiftFraction,
    fmi3EvaluateDiscreteStates,
    fmi3GetOutputDerivatives,
);

//! S095・S099・S109・S112・S114: FMU を関数で直接呼ぶ検査と、FMPy で回す検査（FMPy が回せない環境では skip）。
//! S137: FMPy の検査は FMI 2.0 と 3.0 の両方で回す。
//!
//! モデルは `src/Modelica/GlobeModeValidation/Simulation/` の 1 ファイルを `resources/<完全修飾名>.mo` に置く。

use std::ffi::{CString, c_int};
use std::path::{Path, PathBuf};
use std::process::Command;

use globe_mode_fmu::*;

fn source(name: &str) -> (String, String) {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!(
        "../../../Modelica/GlobeModeValidation/Simulation/{name}.mo"
    ));
    (
        format!("GlobeModeValidation.Simulation.{name}"),
        std::fs::read_to_string(path).unwrap(),
    )
}

/// `resources/<完全修飾名>.mo` を置いた一時の置き場（FMU を展開したときの `resources` に当たる）。
/// 試験は並行に走るので、置き場は試験ごとの `tag` で分ける。debug の cdylib は 700 MB あり、tmpfs の /tmp に
/// 置くと溢れるので、target の下（`CARGO_TARGET_TMPDIR`）に置く。
fn resources(tag: &str, model: &str, text: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!(
        "globe_mode_fmu_{}_{tag}/resources",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join(format!("{model}.mo")), text).unwrap();
    dir
}

#[test]
fn model_description_lists_states_derivatives_and_parameters() {
    let (model, text) = source("HarmonicOscillator");
    let md = model_description(&model, &text).unwrap();
    assert!(md.contains(&format!("guid=\"{}\"", guid(&model, &text))));
    for v in [
        r#"name="x" valueReference="0""#,
        r#"name="v" valueReference="1""#,
        r#"name="der(x)" valueReference="2""#,
        r#"name="der(v)" valueReference="3""#,
        r#"name="w" valueReference="4""#,
        r#"<Real start="2.0"/>"#,
        r#"stopTime="1.0" tolerance="1e-8""#,
    ] {
        assert!(md.contains(v), "{v} が無い:\n{md}");
    }
}

/// S137: 3.0 の modelDescription.xml。derivative と ModelStructure は valueReference で、EventIndicator は
/// Float64 の変数 `_eventIndicator1` を指す。
#[test]
fn model_description_3_lists_event_indicators() {
    let (model, text) = source("BouncingBall");
    let md = model_description_3(&model, &text).unwrap();
    let key = "<Float64 name=\"_eventIndicator1\" valueReference=\"";
    let at = md.find(key).expect(&md) + key.len();
    let r = &md[at..at + md[at..].find('"').unwrap()];
    for v in [
        format!("instantiationToken=\"{}\"", guid(&model, &text)),
        r#"<Float64 name="der(h)" valueReference="2" causality="local" variability="continuous" initial="calculated" derivative="0"/>"#.into(),
        r#"<ContinuousStateDerivative valueReference="2"/>"#.into(),
        format!("<EventIndicator valueReference=\"{r}\"/>"),
    ] {
        assert!(md.contains(&v), "{v} が無い:\n{md}");
    }
}

/// S112: 離散の変数（Integer の n、discrete Real の xs）が値参照 `2n+p..` に載る。
#[test]
fn model_description_lists_discrete_variables() {
    let (model, text) = source("SampleCounter");
    let md = model_description(&model, &text).unwrap();
    for v in [
        r#"name="n" valueReference="#,
        r#"causality="local" variability="discrete" initial="calculated">
      <Integer/>"#,
        r#"name="xs" valueReference="#,
        r#"<Real/>"#,
    ] {
        assert!(md.contains(v), "{v} が無い:\n{md}");
    }
}

/// S114: 入力 u は `causality="input"`（start 必須）、出力 y は `ModelStructure` の `Outputs` に載り、
/// `fmi2SetReal` で u を書くと微分と y に効く。
#[test]
fn inputs_and_outputs_through_the_c_abi() {
    let (model, text) = source("InputDecay");
    let md = model_description(&model, &text).unwrap();
    let vr = |name: &str| -> u32 {
        let key = format!("name=\"{name}\" valueReference=\"");
        let at = md.find(&key).unwrap() + key.len();
        md[at..at + md[at..].find('"').unwrap()].parse().unwrap()
    };
    let (u, y) = (vr("u"), vr("y"));
    for v in [
        format!(
            "name=\"u\" valueReference=\"{u}\" causality=\"input\" variability=\"continuous\">\n      <Real start=\"0.0\"/>"
        ),
        format!("name=\"y\" valueReference=\"{y}\" causality=\"output\""),
        format!("<Outputs>\n      <Unknown index=\"{}\"/>", y + 1),
    ] {
        assert!(md.contains(&v), "{v} が無い:\n{md}");
    }
    let dir = resources("inputs", &model, &text);
    let location = CString::new(format!("file://{}", dir.display())).unwrap();
    let guid = CString::new(guid(&model, &text)).unwrap();
    unsafe {
        let c = fmi2Instantiate(
            c"i".as_ptr(),
            0,
            guid.as_ptr(),
            location.as_ptr(),
            std::ptr::null(),
            0,
            0,
        );
        assert!(!c.is_null());
        assert_eq!(fmi2EnterInitializationMode(c), 0);
        assert_eq!(fmi2SetReal(c, [u].as_ptr(), 1, [3.0].as_ptr()), 0);
        assert_eq!(
            fmi2SetReal(c, [y].as_ptr(), 1, [3.0].as_ptr()),
            3,
            "出力は書けない"
        );
        assert_eq!(fmi2ExitInitializationMode(c), 0);
        let mut dx = [0.0];
        assert_eq!(fmi2GetDerivatives(c, dx.as_mut_ptr(), 1), 0);
        assert_eq!(dx[0], 2.0, "der(x) = -x + u = -1 + 3");
        assert_eq!(fmi2SetReal(c, [u].as_ptr(), 1, [-1.0].as_ptr()), 0);
        assert_eq!(fmi2SetContinuousStates(c, [0.5].as_ptr(), 1), 0);
        let mut v = [0.0; 2];
        assert_eq!(fmi2GetReal(c, [u, y].as_ptr(), 2, v.as_mut_ptr()), 0);
        assert_eq!(v, [-1.0, 1.0]);
        assert_eq!(fmi2GetDerivatives(c, dx.as_mut_ptr(), 1), 0);
        assert_eq!(dx[0], -1.5);
        fmi2FreeInstance(c);
    }
    let _ = std::fs::remove_dir_all(dir.parent().unwrap());
}

/// S224: Enumeration（パラメータ・入力・出力）は `declaredType` つきの Integer として読み書きし、String のパラメータは
/// `fmi2GetString` で読む（書くと Error）。2.0 と 3.0 の modelDescription.xml に型の定義が載る。
#[test]
fn enumeration_and_string_through_the_c_abi() {
    let (model, text) = source("EnumString");
    let md = model_description(&model, &text).unwrap();
    let md3 = model_description_3(&model, &text).unwrap();
    for v in [
        r#"<SimpleType name="Speed">"#,
        r#"<Item name="fast" value="2"/>"#,
        r#"<Enumeration declaredType="Speed" start="2"/>"#,
        r#"<Enumeration declaredType="Speed" start="1"/>"#,
        r#"<String start="decay"/>"#,
    ] {
        assert!(md.contains(v), "{v} が無い:\n{md}");
    }
    for v in [
        r#"<EnumerationType name="Speed">"#,
        r#"<Enumeration name="speed" valueReference="#,
        r#"<Start value="decay"/>"#,
    ] {
        assert!(md3.contains(v), "{v} が無い:\n{md3}");
    }
    // 自分の読み手を通して、型の定義と declaredType が戻る
    for (xml, v3) in [(&md, false), (&md3, true)] {
        let d = globe_mode_fmi::parse_model_description(xml).unwrap();
        assert_eq!(d.enum_types.len(), 1, "{xml}");
        assert_eq!(
            d.enum_types[0].items,
            [("slow".to_string(), 1), ("fast".to_string(), 2)]
        );
        let var = |n: &str| d.variables.iter().find(|v| v.name == n).unwrap();
        for n in ["speed", "request", "level"] {
            assert_eq!(
                var(n).ty,
                globe_mode_fmi::VarType::Enumeration,
                "{n} (v3={v3})"
            );
            assert_eq!(var(n).declared_type.as_deref(), Some("Speed"));
        }
        let label = var("label");
        assert_eq!(
            (label.ty, label.start.as_deref()),
            (globe_mode_fmi::VarType::String, Some("decay"))
        );
        assert_eq!(label.variability, globe_mode_fmi::Variability::Fixed);
    }
    let d = globe_mode_fmi::parse_model_description(&md).unwrap();
    let vr = |n: &str| {
        d.variables
            .iter()
            .find(|v| v.name == n)
            .unwrap()
            .value_reference
    };
    let (speed, label, request, level) = (vr("speed"), vr("label"), vr("request"), vr("level"));
    let dir = resources("enum", &model, &text);
    let location = CString::new(format!("file://{}", dir.display())).unwrap();
    let guid = CString::new(guid(&model, &text)).unwrap();
    unsafe {
        let c = fmi2Instantiate(
            c"i".as_ptr(),
            0,
            guid.as_ptr(),
            location.as_ptr(),
            std::ptr::null(),
            0,
            0,
        );
        assert!(!c.is_null());
        let int = |c, r: u32| {
            let mut v = [0 as c_int];
            assert_eq!(
                fmi2GetInteger(c, [r].as_ptr(), 1, v.as_mut_ptr()),
                0,
                "vr {r}"
            );
            v[0]
        };
        // level（出力）は離散の変数。S237 から初期化の事象を回すので、最初から speed と同じ 2（それまでは開始値の 1 のままだった）
        assert_eq!((int(c, speed), int(c, request), int(c, level)), (2, 1, 2));
        let mut s = [std::ptr::null()];
        assert_eq!(fmi2GetString(c, [label].as_ptr(), 1, s.as_mut_ptr()), 0);
        assert_eq!(std::ffi::CStr::from_ptr(s[0]).to_str().unwrap(), "decay");
        assert_eq!(
            fmi2SetString(c, [label].as_ptr(), 1, s.as_ptr()),
            3,
            "String は fixed"
        );
        // 範囲の外と、出力と、Real の変数に Integer は書けない
        assert_eq!(fmi2SetInteger(c, [speed].as_ptr(), 1, [3].as_ptr()), 3);
        assert_eq!(fmi2SetInteger(c, [level].as_ptr(), 1, [1].as_ptr()), 3);
        assert_eq!(fmi2SetInteger(c, [0].as_ptr(), 1, [1].as_ptr()), 3);
        // speed = fast（2）なら der(x) = -2x、slow（1）に変えて初期化し直すと -x
        let mut dx = [0.0];
        assert_eq!(fmi2GetDerivatives(c, dx.as_mut_ptr(), 1), 0);
        assert_eq!(dx[0], -2.0);
        assert_eq!(fmi2SetInteger(c, [speed].as_ptr(), 1, [1].as_ptr()), 0);
        assert_eq!(fmi2SetInteger(c, [request].as_ptr(), 1, [2].as_ptr()), 0);
        assert_eq!(fmi2ExitInitializationMode(c), 0);
        assert_eq!((int(c, speed), int(c, request), int(c, level)), (1, 2, 1));
        assert_eq!(fmi2GetDerivatives(c, dx.as_mut_ptr(), 1), 0);
        assert_eq!(dx[0], -1.0);
        fmi2FreeInstance(c);
    }
    let _ = std::fs::remove_dir_all(dir.parent().unwrap());
}

#[test]
fn explicit_euler_through_the_c_abi() {
    let (model, text) = source("Decay");
    let dir = resources("c_abi", &model, &text);
    let location = CString::new(format!("file://{}", dir.display())).unwrap();
    let guid = CString::new(guid(&model, &text)).unwrap();
    let instantiate = |kind: c_int, g: &CString| unsafe {
        fmi2Instantiate(
            c"d".as_ptr(),
            kind,
            g.as_ptr(),
            location.as_ptr(),
            std::ptr::null(),
            0,
            0,
        )
    };
    unsafe {
        // S231: CS も受ける（fmi2Type は 0 = ME、1 = CS）。それ以外は断る
        let cs = instantiate(1, &guid);
        assert!(!cs.is_null(), "CS は受ける");
        fmi2FreeInstance(cs);
        assert!(instantiate(2, &guid).is_null(), "ME・CS 以外は断る");
        assert!(
            instantiate(0, &CString::new("{x}").unwrap()).is_null(),
            "guid 違いは断る"
        );
        let c = instantiate(0, &guid);
        assert!(!c.is_null());
        assert_eq!(fmi2EnterInitializationMode(c), 0);
        assert_eq!(fmi2SetReal(c, [2].as_ptr(), 1, [2.0].as_ptr()), 0);
        assert_eq!(fmi2ExitInitializationMode(c), 0);
        let (mut x, mut dx) = ([0.0], [0.0]);
        let n = 100_000;
        for _ in 0..n {
            assert_eq!(fmi2GetContinuousStates(c, x.as_mut_ptr(), 1), 0);
            assert_eq!(fmi2GetDerivatives(c, dx.as_mut_ptr(), 1), 0);
            x[0] += dx[0] / f64::from(n);
            assert_eq!(fmi2SetContinuousStates(c, x.as_ptr(), 1), 0);
        }
        assert!((x[0] - (-2.0f64).exp()).abs() < 1e-4, "x(1) = {}", x[0]);
        let mut v = [0.0; 3];
        assert_eq!(fmi2GetReal(c, [0, 1, 2].as_ptr(), 3, v.as_mut_ptr()), 0);
        assert_eq!(v, [x[0], -2.0 * x[0], 2.0]);
        assert_eq!(
            fmi2GetReal(c, [9].as_ptr(), 1, v.as_mut_ptr()),
            3,
            "無い値参照"
        );
        assert_eq!(fmi2GetDerivatives(c, dx.as_mut_ptr(), 2), 3, "状態の数違い");
        let (mut e, mut t): (c_int, c_int) = (1, 1);
        assert_eq!(fmi2CompletedIntegratorStep(c, 1, &mut e, &mut t), 0);
        assert_eq!((e, t), (0, 0));
        assert_eq!(fmi2Reset(c), 0);
        assert_eq!(fmi2GetReal(c, [0, 2].as_ptr(), 2, v.as_mut_ptr()), 0);
        assert_eq!(&v[..2], [1.0, 1.0], "Reset で既定に戻る");
        fmi2FreeInstance(c);
    }
    let _ = std::fs::remove_dir_all(dir.parent().unwrap());
}

/// #40: FMPy の `start_values={'x': 5, 'k': 2}` と同じ順（状態 x を書き、パラメータ k を書き、Exit）。
/// パラメータの変更で作り直しても、x の開始値は戻らない。開始時刻を変えたときも同じ。
#[test]
fn state_start_values_survive_a_rebuild() {
    let (model, text) = source("Decay");
    let dir = resources("start_values", &model, &text);
    let location = CString::new(format!("file://{}", dir.display())).unwrap();
    let guid = CString::new(guid(&model, &text)).unwrap();
    unsafe {
        let c = fmi2Instantiate(
            c"d".as_ptr(),
            0,
            guid.as_ptr(),
            location.as_ptr(),
            std::ptr::null(),
            0,
            0,
        );
        assert!(!c.is_null());
        assert_eq!(fmi2SetupExperiment(c, 0, 0.0, 0.5, 0, 0.0), 0);
        assert_eq!(fmi2EnterInitializationMode(c), 0);
        assert_eq!(fmi2SetReal(c, [0].as_ptr(), 1, [5.0].as_ptr()), 0);
        assert_eq!(fmi2SetReal(c, [2].as_ptr(), 1, [2.0].as_ptr()), 0);
        assert_eq!(fmi2ExitInitializationMode(c), 0);
        let mut v = [0.0; 2];
        assert_eq!(fmi2GetReal(c, [0, 2].as_ptr(), 2, v.as_mut_ptr()), 0);
        assert_eq!(v, [5.0, 2.0], "x の start 値と k の上書きが両方残る");
        fmi2FreeInstance(c);
    }
    let _ = std::fs::remove_dir_all(dir.parent().unwrap());
}

/// cdylib を作り、`model` を FMU に詰めて FMPy の ME で回す。終了時刻での `names` の値と事象の回数
/// （`events`・`reinit`）を `名前=値` で並べた出力を返す。FMPy が使えなければ None。
/// `input` は FMPy の `input=` に渡す入力（`{"u": [[時刻, 値], …]}` の JSON。無ければ空）。
/// `v3` なら FMI 3.0 の FMU にする（S137）。
fn fmpy(name: &str, names: &[&str], input: &str, v3: bool) -> Option<String> {
    fmpy_as(name, names, input, v3, Kind::Me)
}

/// [`fmpy`] と同じで、`kind` が `Cs` なら FMPy の Co-Simulation で回す（S231。`modelDescription.xml` は CS だけ）。
fn fmpy_as(name: &str, names: &[&str], input: &str, v3: bool, kind: Kind) -> Option<String> {
    fmpy_with(source(name), None, names, input, v3, kind)
}

/// [`fmpy_as`] の本体。`libdir` があれば MSL などのライブラリを FMU の `resources/lib` に入れる（S237。`model` がライブラリの
/// クラスなら `text` はコメントだけ）。
fn fmpy_with(
    (model, text): (String, String),
    libdir: Option<&Path>,
    names: &[&str],
    input: &str,
    v3: bool,
    kind: Kind,
) -> Option<String> {
    let name = model.rsplit('.').next().unwrap().to_string();
    let name = name.as_str();
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let tmp = PathBuf::from(env!("CARGO_TARGET_TMPDIR"));
    let uv = |args: &[&std::ffi::OsStr]| {
        Command::new("uv")
            // FMPy が FMU を展開する置き場も target の下にする
            .env("TMPDIR", &tmp)
            .args(["run", "--no-project", "--with", "fmpy", "python", "-I"])
            .args(args)
            .output()
    };
    match uv(&["-c".as_ref(), "import fmpy".as_ref()]) {
        Ok(o) if o.status.success() => {}
        other => {
            eprintln!("skip: FMPy を使えない: {other:?}");
            return None;
        }
    }
    // 共有ライブラリは別 crate（globe-mode-fmu-cdylib、S240）なので、cargo で作らせて置き場を JSON の出力から取る
    let mut cargo = Command::new(env!("CARGO"));
    cargo.args(["build", "--lib", "--message-format=json", "--manifest-path"]);
    cargo.arg(
        manifest
            .with_file_name("globe-mode-fmu-cdylib")
            .join("Cargo.toml"),
    );
    // S234: `cargo test --release` なら cdylib も release（fmu-export.yml の fmu-native が速く回すため）
    if !cfg!(debug_assertions) {
        cargo.arg("--release");
    }
    let build = cargo.output().unwrap();
    assert!(
        build.status.success(),
        "{}",
        String::from_utf8_lossy(&build.stderr)
    );
    // S234: Windows は globe_mode_fmu_cdylib.dll、macOS は libglobe_mode_fmu_cdylib.dylib（JSON の中の `\` は `\\`）
    let lib = if cfg!(windows) {
        "globe_mode_fmu_cdylib.dll"
    } else if cfg!(target_os = "macos") {
        "libglobe_mode_fmu_cdylib.dylib"
    } else {
        "libglobe_mode_fmu_cdylib.so"
    };
    let json = String::from_utf8_lossy(&build.stdout);
    let end = json
        .find(&format!("{lib}\""))
        .expect("cdylib の置き場が出ない")
        + lib.len();
    let so = PathBuf::from(json[json[..end].rfind('"').unwrap() + 1..end].replace("\\\\", "\\"));
    // 試験は並行に走るので、置き場は ME と CS で分ける（S231）
    let dir = resources(&format!("{name}-{kind:?}"), &model, &text);
    let work = dir.parent().unwrap();
    let md = work.join("modelDescription.xml");
    let xml = match libdir {
        Some(l) => model_description_lib(&model, &text, v3, kind, l),
        None => model_description_kind(&model, &text, v3, kind),
    };
    std::fs::write(&md, xml.unwrap()).unwrap();
    let fmu = work.join("model.fmu");
    let out = uv(&[
        manifest.join("tests/fmpy_run.py").as_os_str(),
        so.as_os_str(),
        md.as_os_str(),
        dir.join(format!("{model}.mo")).as_os_str(),
        model.as_ref(),
        fmu.as_os_str(),
        names.join(",").as_ref(),
        if v3 { "3" } else { "2" }.as_ref(),
        input.as_ref(),
        if kind == Kind::Cs {
            "CoSimulation"
        } else {
            "ModelExchange"
        }
        .as_ref(),
        libdir.map_or(std::ffi::OsStr::new(""), |l| l.as_os_str()),
    ])
    .unwrap();
    let _ = std::fs::remove_dir_all(work);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        out.status.success(),
        "FMPy が失敗: {stdout}\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    eprintln!(
        "FMPy {name}（FMI {}）: {}",
        if v3 { 3 } else { 2 },
        stdout.trim()
    );
    Some(stdout.into_owned())
}

/// `globe_mode_exec::run`（bdf の既定）で回し、終了時刻での `names` の値を返す。
fn exec_last(name: &str, names: &[&str]) -> Vec<f64> {
    let (model, text) = source(name);
    let report = globe_mode_exec::run(&compile(&model, &text).unwrap(), &Default::default());
    let run = report.run.expect("exec で回る");
    let last: Vec<f64> = names
        .iter()
        .map(|n| *run.trajectory.variable(n).unwrap().values.last().unwrap())
        .collect();
    eprintln!("exec {name}: {names:?} = {last:?}");
    last
}

fn value(stdout: &str, name: &str) -> f64 {
    stdout
        .split(&format!("{name}="))
        .nth(1)
        .and_then(|s| s.split_whitespace().next())
        .unwrap()
        .parse()
        .unwrap()
}

#[test]
fn fmpy_simulates_decay() {
    for v3 in [false, true] {
        let Some(out) = fmpy("Decay", &["x"], "", v3) else {
            return;
        };
        let x = value(&out, "x");
        assert!((x - (-1.0f64).exp()).abs() < 1e-4, "x(1) = {x}");
    }
}

/// 状態 2 つ・事象なしの HarmonicOscillator を、FMPy と `globe_mode_exec::run`（bdf の既定）で回して x(1)・v(1) を突き合わせる。
#[test]
fn fmpy_matches_exec_on_harmonic_oscillator() {
    for v3 in [false, true] {
        let Some(out) = fmpy("HarmonicOscillator", &["x", "v"], "", v3) else {
            return;
        };
        let exec = exec_last("HarmonicOscillator", &["x", "v"]);
        for (n, e) in ["x", "v"].iter().zip(&exec) {
            let f = value(&out, n);
            assert!((f - e).abs() < 1e-4, "{n}: FMPy {f} と exec {e}");
        }
    }
}

/// S109・S112: 状態事象と reinit。跳ねた回数（FMU の中の n と reinit で状態が変わった事象の数、exec の n）と
/// h(2)・v(2) を突き合わせる。
#[test]
fn fmpy_matches_exec_on_bouncing_ball() {
    for v3 in [false, true] {
        let Some(out) = fmpy("BouncingBall", &["h", "v", "n"], "", v3) else {
            return;
        };
        let exec = exec_last("BouncingBall", &["h", "v", "n"]);
        assert_eq!(value(&out, "n"), exec[2], "FMU の中の n");
        assert_eq!(value(&out, "reinit"), exec[2], "跳ねた回数");
        for (n, e) in ["h", "v"].iter().zip(&exec) {
            let f = value(&out, n);
            assert!((f - e).abs() < 1e-3, "{n}: FMPy {f} と exec {e}");
        }
    }
}

/// S109・S112: 時刻事象（`sample`）。FMU の中の Integer の n と discrete Real の xs（fmi2GetInteger・fmi2GetReal）、
/// x(1) を exec と突き合わせる（n は 10、xs は最後の標本の時刻 0.95 の x）。
#[test]
fn fmpy_matches_exec_on_sample_counter() {
    for v3 in [false, true] {
        let Some(out) = fmpy("SampleCounter", &["x", "n", "xs"], "", v3) else {
            return;
        };
        let exec = exec_last("SampleCounter", &["x", "n", "xs"]);
        assert_eq!(value(&out, "n"), 10.0, "FMU の中の n");
        assert_eq!(value(&out, "n"), exec[1], "n");
        for (n, e) in ["x", "xs"].iter().zip([exec[0], exec[2]]) {
            let f = value(&out, n);
            assert!((f - e).abs() < 1e-6, "{n}: FMPy {f} と exec {e}");
        }
    }
}

/// S112: 代数変数（非線形のループの y・z）を fmi2GetReal で読み、exec の最後の値（解析解 y = 2、z = 4）と突き合わせる。
/// HarmonicOscillator・BouncingBall には代数変数が無いので NonlinearLoop を使う。
#[test]
fn fmpy_matches_exec_on_algebraic_variables() {
    for v3 in [false, true] {
        let Some(out) = fmpy("NonlinearLoop", &["x", "y", "z"], "", v3) else {
            return;
        };
        let exec = exec_last("NonlinearLoop", &["x", "y", "z"]);
        for (n, e) in ["x", "y", "z"].iter().zip(&exec) {
            let f = value(&out, n);
            assert!((f - e).abs() < 1e-4, "{n}: FMPy {f} と exec {e}");
        }
    }
}

/// S224: Enumeration と String を持つ modelDescription.xml が FMPy の validate を通り、ME で回る（speed = fast なので x(1) = e^-2）。
#[test]
fn fmpy_validates_enumeration_and_string() {
    for v3 in [false, true] {
        let Some(out) = fmpy("EnumString", &["x", "level"], "", v3) else {
            return;
        };
        let x = value(&out, "x");
        assert!((x - (-2.0f64).exp()).abs() < 1e-5, "x(1) = {x}");
    }
}

/// S114: 入力 u（`causality="input"`）を FMPy の `input=` で t = 0.5 に 0 → 2 の階段で与え、x(1)・出力 y(1) = 2 x(1) を
/// 解析解 x(1) = 2 + (e^-0.5 - 2) e^-0.5 と突き合わせる（exec の `run` は入力を与える道が無く、駆動されない入力を断る）。
#[test]
fn fmpy_drives_input_step() {
    for v3 in [false, true] {
        let Some(out) = fmpy(
            "InputDecay",
            &["x", "y", "u"],
            r#"{"u": [[0, 0], [0.5, 0], [0.5, 2], [1, 2]]}"#,
            v3,
        ) else {
            return;
        };
        let h = (-0.5f64).exp();
        let x = 2.0 + (h - 2.0) * h;
        assert_eq!(value(&out, "u"), 2.0, "u(1)");
        for (n, e) in [("x", x), ("y", 2.0 * x)] {
            let f = value(&out, n);
            eprintln!("{n}(1): FMPy {f}、解析解 {e}、差 {:.1e}", (f - e).abs());
            assert!((f - e).abs() < 1e-5, "{n}: FMPy {f} と解析解 {e}");
        }
    }
}

/// S231: Co-Simulation。FMPy の CS（`fmi2DoStep` / `fmi3DoStep`）で、Decay の x(1) = e^-1、BouncingBall の h・v・n を exec と
/// （状態事象と reinit を刻みの中で回す）、InputDecay の入力の階段を解析解と突き合わせる。
#[test]
fn fmpy_co_simulation() {
    for v3 in [false, true] {
        let Some(out) = fmpy_as("Decay", &["x"], "", v3, Kind::Cs) else {
            return;
        };
        let x = value(&out, "x");
        assert!((x - (-1.0f64).exp()).abs() < 1e-6, "Decay x(1) = {x}");

        let out = fmpy_as("BouncingBall", &["h", "v", "n"], "", v3, Kind::Cs).unwrap();
        let exec = exec_last("BouncingBall", &["h", "v", "n"]);
        assert_eq!(value(&out, "n"), exec[2], "跳ねた回数");
        for (n, e) in ["h", "v"].iter().zip(&exec) {
            let f = value(&out, n);
            assert!((f - e).abs() < 1e-3, "{n}: FMPy(CS) {f} と exec {e}");
        }

        let input = r#"{"u": [[0, 0], [0.5, 0], [0.5, 2], [1, 2]]}"#;
        let out = fmpy_as("InputDecay", &["x", "y"], input, v3, Kind::Cs).unwrap();
        let h = (-0.5f64).exp();
        let x = 2.0 + (h - 2.0) * h;
        for (n, e) in [("x", x), ("y", 2.0 * x)] {
            let f = value(&out, n);
            assert!((f - e).abs() < 1e-5, "{n}: FMPy(CS) {f} と解析解 {e}");
        }
    }
}

/// S231: CS を含む modelDescription.xml は `canHandleVariableCommunicationStepSize` を書き、既定（ME だけ）は変わらない。
#[test]
fn model_description_kind_writes_co_simulation() {
    let (model, text) = source("Decay");
    for v3 in [false, true] {
        let get = |k| model_description_kind(&model, &text, v3, k).unwrap();
        let cs = r#"<CoSimulation modelIdentifier="globe_mode_fmu" canHandleVariableCommunicationStepSize="true"/>"#;
        let me = r#"<ModelExchange modelIdentifier="globe_mode_fmu"/>"#;
        let (m, c, b) = (get(Kind::Me), get(Kind::Cs), get(Kind::Both));
        assert!(m.contains(me) && !m.contains("CoSimulation"));
        assert!(c.contains(cs) && !c.contains("ModelExchange"));
        assert!(b.contains(cs) && b.contains(me));
        assert_eq!(
            m,
            if v3 {
                model_description_3(&model, &text)
            } else {
                model_description(&model, &text)
            }
            .unwrap()
        );
    }
}

/// S237: MSL の例題を P5b で。`CompareBrakingTorque` の `SignTorque` は `abs(w) >= w0`（noEvent でない関係式）で、
/// 初期化の点（w = 100）から関係式の記憶を決めないと最初から別の枝（`w/w0`）で微分を返し、w が指数で減る
/// （正しくは一定のトルクで 1 s で止まり、phi = 50.005）。`data/msl` が無ければ飛ばす。
#[test]
fn fmpy_runs_an_msl_example_through_resources_lib() {
    let msl = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../../data/msl");
    if !msl.is_dir() {
        eprintln!("skip: data/msl が無い");
        return;
    }
    let model = "Modelica.Mechanics.Rotational.Examples.CompareBrakingTorque";
    for (v3, kind) in [(false, Kind::Me), (true, Kind::Cs)] {
        let src = (model.to_string(), "// library model\n".to_string());
        let Some(out) = fmpy_with(
            src,
            Some(&msl),
            &["inertia1.phi", "inertia1.w"],
            "",
            v3,
            kind,
        ) else {
            return;
        };
        let phi = value(&out, "inertia1.phi");
        assert!((phi - 50.005).abs() < 0.05, "phi(5) = {phi}");
    }
}

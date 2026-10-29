//! シミュレーションの設定と結果（S011、ADR-0013）。実行系（`globe-mode-exec`）と結果の置き場
//! （`globe-mode-sim`）の間で受け渡す中立の型。Rumoca の型は出さない。
//!
//! 値は f64 なので `Eq` にしない（salsa のクエリには載せない。実行は salsa の外のジョブ）。

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// 利用者が指定した実行の設定。None の項目は experiment annotation か既定値で決める
/// （[`ResolvedSettings`]）。
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct SimSettings {
    pub start_time: Option<f64>,
    pub stop_time: Option<f64>,
    /// 相対許容誤差（絶対許容誤差にも同じ値を使う。Rumoca の MSL ハーネスと同じ）
    pub tolerance: Option<f64>,
    /// 出力の間隔
    pub interval: Option<f64>,
    /// GlobeMode のソルバの名前（`bdf`、`dopri45`、`auto`、別名の `dassl` など。doc/architecture.md §3.3）
    pub solver: Option<String>,
    /// 壁時計の予算（秒）。None は無制限
    pub max_wall_seconds: Option<f64>,
    /// 固定刻みのソルバ（`euler`・`heun`・`rk4`・`impeuler`・`trapezoid`）の刻み幅。None なら出力の間隔（S012）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub step: Option<f64>,
    /// パラメータの上書き（名前 → 値）。再コンパイル無しに変えられる（tunable な）パラメータだけ。
    /// 変えられないものは GM-E10006（S012）
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub parameters: BTreeMap<String, f64>,
}

/// 設定の値をどこから決めたか。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SettingOrigin {
    /// 利用者が指定した（CLI の引数など）
    Requested,
    /// モデルの experiment annotation
    Experiment,
    /// 既定値（StartTime 0、StopTime は開始 + 1、Tolerance 1e-6、Interval は区間の 1/500、ソルバ bdf）
    Default,
}

/// GlobeMode のソルバ（実装のあるもの）。名前の登録表は `globe_mode_sim::solvers`。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SolverId {
    /// 可変次数の BDF（diffsol。既定）
    Bdf,
    /// Dormand–Prince 5(4)（陽的、刻み幅の制御あり）
    Dopri45,
    /// 実行系に選ばせる（Rumoca の Auto: 系を見て BDF か陽的 RK を選ぶ。S012 から、BDF が積分で失敗したら
    /// dopri45 で回し直す）
    Auto,
    /// ESDIRK 3(4)（diffsol の SDIRK。陰的、刻み幅の制御あり。S012）
    Esdirk34,
    /// TR-BDF2（diffsol の SDIRK。陰的、刻み幅の制御あり。S012）
    Trbdf2,
    /// Tsitouras 5(4)（diffsol の陽的 RK、刻み幅の制御あり。S012）
    Tsit45,
    /// 陽的 Euler 法（固定刻み。S012）
    Euler,
    /// Heun 法（固定刻み、2 次。S012）
    Heun,
    /// 古典的な 4 段の Runge–Kutta 法（固定刻み。S012）
    Rk4,
    /// 陰的 Euler 法（固定刻み、1 次。S026）
    Impeuler,
    /// 台形公式（固定刻み、2 次。S026）
    Trapezoid,
    /// ESDIRK 2(3)（3 段、2 次の主解と 3 次の埋め込み。diffsol の SDIRK に自前の係数表。S027）
    Esdirk23,
    /// ESDIRK 5(4)（7 段、5 次の主解と 4 次の埋め込み。diffsol の SDIRK に自前の係数表。S027）
    Esdirk45,
    /// Hairer–Wanner の SDIRK4（5 段、4 次の主解と 3 次の埋め込み。diffsol の SDIRK に自前の係数表。S027）
    Sdirk34hw,
    /// DOP853（13 段の FSAL、8 次の主解と 5 次の埋め込み。diffsol の陽的 RK に自前の係数表。S028）
    Dopri853,
    /// CERK 3(2)（4 段の FSAL、3 次の主解と 2 次の埋め込み、3 次の補間。Owren–Zennaro。diffsol の陽的 RK に自前の係数表。S029）
    Cerk23,
    /// CERK 4(3)（6 段の FSAL、4 次の主解と 3 次の埋め込み、4 次の補間。S029）
    Cerk34,
    /// CERK 5(4)（8 段の FSAL、5 次の主解と 4 次の埋め込み、5 次の補間。S029）
    Cerk45,
    /// Radau IIA 3 段 5 次（陰的、刻み幅の制御あり、簡略 Newton 法。自前の積分器。S030）
    Radau5,
    /// 量子化状態法 QSS2（事象ごとに進む、2 次。自前の積分器。S031）
    Qss,
}

impl SolverId {
    /// GlobeMode での名前。
    pub const fn name(self) -> &'static str {
        match self {
            SolverId::Bdf => "bdf",
            SolverId::Dopri45 => "dopri45",
            SolverId::Auto => "auto",
            SolverId::Esdirk34 => "esdirk34",
            SolverId::Trbdf2 => "trbdf2",
            SolverId::Tsit45 => "tsit45",
            SolverId::Euler => "euler",
            SolverId::Heun => "heun",
            SolverId::Rk4 => "rk4",
            SolverId::Impeuler => "impeuler",
            SolverId::Trapezoid => "trapezoid",
            SolverId::Esdirk23 => "esdirk23",
            SolverId::Esdirk45 => "esdirk45",
            SolverId::Sdirk34hw => "sdirk34hw",
            SolverId::Dopri853 => "dopri853",
            SolverId::Cerk23 => "cerk23",
            SolverId::Cerk34 => "cerk34",
            SolverId::Cerk45 => "cerk45",
            SolverId::Radau5 => "radau5",
            SolverId::Qss => "qss",
        }
    }

    /// 固定刻みのソルバか（許容誤差を使わず、刻み幅 [`ResolvedSettings::step`] で進む）。
    pub const fn is_fixed_step(self) -> bool {
        matches!(
            self,
            SolverId::Euler
                | SolverId::Heun
                | SolverId::Rk4
                | SolverId::Impeuler
                | SolverId::Trapezoid
        )
    }
}

/// 決まった実行の設定。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResolvedSettings {
    pub start_time: f64,
    pub stop_time: f64,
    pub tolerance: f64,
    pub interval: f64,
    pub solver: SolverId,
    pub max_wall_seconds: Option<f64>,
    /// 固定刻みの幅。固定刻みのソルバのときだけ Some（指定が無ければ出力の間隔）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub step: Option<f64>,
    /// パラメータの上書き（[`SimSettings::parameters`] をそのまま写す）
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub parameters: BTreeMap<String, f64>,
    /// 各項目の出どころ（start_time / stop_time / tolerance / interval / solver の順）
    pub origins: SettingOrigins,
}

/// [`ResolvedSettings`] の各項目の出どころ。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SettingOrigins {
    pub start_time: SettingOrigin,
    pub stop_time: SettingOrigin,
    pub tolerance: SettingOrigin,
    pub interval: SettingOrigin,
    pub solver: SettingOrigin,
    /// 固定刻みの幅の出どころ（固定刻みのソルバのときだけ Some）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub step: Option<SettingOrigin>,
}

/// 結果の変数の役割。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VariableRole {
    State,
    /// 状態の微分 `der(x)`（実行系が出すときだけ）
    Derivative,
    Algebraic,
    Output,
    Input,
    /// 離散の変数（Real・Integer・Boolean。値は f64 に直す）
    Discrete,
    Parameter,
}

/// 時系列の変数 1 つ。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TrajectoryVariable {
    /// スカラーの名前（例: `C1.v`、`x[2]`、`a[1,2]`）
    pub name: String,
    pub role: VariableRole,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,
    /// 変数の説明（Modelica の description string。S012）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// [`Trajectory::time`] と同じ長さ
    pub values: Vec<f64>,
}

/// 時系列の結果（列ごとに持つ）。イベントの時刻では 2 行続くことがある（左極限と、イベント後の値。Rumoca は
/// 2 行目の時刻を数 ulp だけずらして書くので、時刻は等しいとは限らない）。
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Trajectory {
    pub time: Vec<f64>,
    pub variables: Vec<TrajectoryVariable>,
}

impl Trajectory {
    /// 名前で変数を引く。
    pub fn variable(&self, name: &str) -> Option<&TrajectoryVariable> {
        self.variables.iter().find(|v| v.name == name)
    }

    /// 値に NaN / inf を含む変数の名前。
    pub fn non_finite(&self) -> Vec<&str> {
        self.variables
            .iter()
            .filter(|v| v.values.iter().any(|x| !x.is_finite()))
            .map(|v| v.name.as_str())
            .collect()
    }
}

/// 実行がどう終わったか。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Termination {
    /// 終了時刻まで進んだ
    StopTime,
    /// `terminate()` で終了時刻の前に終わった
    Terminated { time: f64, message: String },
}

/// 実行の統計。
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct SimStats {
    /// 実行系の準備（lowering と組み立て）にかかった壁時計の時間（秒）
    pub build_seconds: f64,
    /// 初期化と積分にかかった壁時計の時間（秒）
    pub run_seconds: f64,
    /// 状態の数
    pub states: usize,
    /// 出力した行の数
    pub points: usize,
}

/// 結果に添えるパラメータの値（MAT v4 の `data_1` などに書く。S012）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResultParameter {
    pub name: String,
    /// 実行に使った値（上書きがあれば上書きの値）
    pub value: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// 再コンパイル無しに変えられる（tunable な）パラメータ（`globe simulate --list-parameters` の 1 行。S012）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TunableParameter {
    pub name: String,
    /// モデルの既定値
    pub default: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// 成功した実行。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SimRun {
    pub model: String,
    pub settings: ResolvedSettings,
    pub trajectory: Trajectory,
    pub termination: Termination,
    pub stats: SimStats,
    /// 実行に使ったパラメータの値（実行系が分かる範囲。無ければ空。S012）
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub parameters: Vec<ResultParameter>,
}

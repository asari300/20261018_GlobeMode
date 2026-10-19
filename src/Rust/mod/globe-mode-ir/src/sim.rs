//! シミュレーションの設定と結果（S011、ADR-0013）。実行系（`globe-mode-exec`）と結果の置き場
//! （`globe-mode-sim`）の間で受け渡す中立の型。Rumoca の型は出さない。
//!
//! 値は f64 なので `Eq` にしない（salsa のクエリには載せない。実行は salsa の外のジョブ）。

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
    /// 実行系に選ばせる（Rumoca の Auto: 系を見て BDF か陽的 RK を選ぶ）
    Auto,
}

impl SolverId {
    /// GlobeMode での名前。
    pub const fn name(self) -> &'static str {
        match self {
            SolverId::Bdf => "bdf",
            SolverId::Dopri45 => "dopri45",
            SolverId::Auto => "auto",
        }
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
    /// [`Trajectory::time`] と同じ長さ
    pub values: Vec<f64>,
}

/// 時系列の結果（列ごとに持つ）。イベントの時刻では同じ時刻が 2 行続くことがある（左極限と、イベント後の値）。
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

/// 成功した実行。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SimRun {
    pub model: String,
    pub settings: ResolvedSettings,
    pub trajectory: Trajectory,
    pub termination: Termination,
    pub stats: SimStats,
}

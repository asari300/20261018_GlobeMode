//! GlobeMode の protocol（S035）。facade `globe-mode` の入出力の型で、CLI の `--json`・wasm・LSP・各言語の
//! 束縛が同じ JSON を使うためのもの。計算は入れない（中身は `globe-mode-core` / `ir` / `analysis` の型）。
//!
//! JSON の形は S034 までの `globe check --json`・`globe simulate --json`・`globe-mode-wasm` の `Response` と
//! 同じ（フィールドの順もそのまま。CLI の snapshot 試験で守る）。応答は Serialize だけ（`Diagnostic` と各段の
//! 報告が Deserialize を持たないため。読み戻しが要るときに core / analysis に足す）。JSON Schema の書き出しは持ち越し
//! （`doc/plan/2026-10-30-release-estimate-and-parallel-plan.md` の WP-0）。

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

pub use globe_mode_analysis::{
    IndexReport, InitReport, LoopReport, PreflightReport, StructuralReport,
};
pub use globe_mode_core::{Diagnostic, Severity, Stage};
pub use globe_mode_ir::{
    BalanceCount, Bits, DaeCounts, Experiment, FlatSummary, InitStructure, ModelParameter, Outline,
    ResolvedSettings, SimSettings, SimStats, Termination, TunableParameter,
};

/// 検査（S0〜S10）の結果（`globe check --json` と同じ形）。
#[derive(Debug, Clone, Serialize)]
pub struct CheckResponse {
    pub model: String,
    /// Error が 1 つも無い
    pub passed: bool,
    /// Error が出た最初の段階
    pub failed_stage: Option<Stage>,
    pub balance: Option<BalanceCount>,
    pub dae: Option<DaeCounts>,
    /// DAE の大きさ（未知数・方程式・導関数の本数）
    pub structure: Option<StructureCounts>,
    pub s6: Option<StructuralReport>,
    pub s7: Option<IndexReport>,
    pub s8: Option<InitReport>,
    pub s9: Option<LoopReport>,
    pub s10: Option<PreflightReport>,
    /// モデルの experiment annotation の数の字面（S142。UI の実行の設定の初期値。annotation が無ければ出ない）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub experiment: Option<ExperimentValues>,
    /// 初期化系の構造（位置は元の本文の中）
    pub init: Option<InitStructure>,
    pub flat: Option<FlatSummary>,
    /// 位置は元の本文の中
    pub diagnostics: Vec<Diagnostic>,
    pub notes: Vec<String>,
    /// Error がすべて「コンパイラが未対応」（`GM-U…`）のときの要約（S173。UI が誤りと分けて見せる。[`unsupported_summary`]）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unsupported: Option<String>,
    /// 強行（`force`）で実行に進める（S173。[`force_helps`]）。出るのは true のときだけ
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub can_force: bool,
    /// 結果を人が読む 1 行（`globe check` の最後の行と同じ。S184。[`CheckResponse::summary_line`]）
    pub summary: String,
}

/// Error がすべて `GM-U…`（コンパイラが未対応）のとき、モデルの誤りと分けて見せる要約（S173）。
pub fn unsupported_summary(diagnostics: &[Diagnostic]) -> Option<String> {
    let mut errors = diagnostics.iter().filter(|d| d.is_error()).peekable();
    (errors.peek().is_some() && errors.all(Diagnostic::is_unsupported)).then(|| {
        "モデルの誤りではなく、コンパイラ（Rumoca）が未対応の機能か既知の制約で止まった。モデルを直しても通らず、強行しても実行できない。原文は下の診断の詳細にある".to_string()
    })
}

/// 検査の Error を無視して実行（`force`）に進めると結果が出る見込みか（S173）。
/// 確かめたのは初期条件が余る `GM-E8001` だけ（`Over` は強行で解ける）。S1〜S5 の Error は DAE にできず、
/// `GM-U` と S6 の構造特異は実行系が止まり、S10 の `GM-E10001`〜`GM-E10006` は実行の設定でも断られる。
/// shortcut: 確かめた 1 種だけ。ほかの Error も強行で通ると分かったら足す。
pub fn force_helps(diagnostics: &[Diagnostic]) -> bool {
    let mut errors = diagnostics.iter().filter(|d| d.is_error()).peekable();
    errors.peek().is_some() && errors.all(|d| d.code == globe_mode_core::codes::INIT_OVERDETERMINED)
}

/// 実行しなかった理由の後ろに足す、最初の error の診断のコードと 1 文（Error が無ければ空）。
fn first_error(diagnostics: &[Diagnostic]) -> String {
    let Some(d) = diagnostics.iter().find(|d| d.is_error()) else {
        return String::new();
    };
    let line = d.message.lines().next().unwrap_or_default();
    let line = match line.char_indices().nth(100) {
        Some((i, _)) => format!("{}…", &line[..i]),
        None => line.to_string(),
    };
    let kind = if d.is_unsupported() {
        "最初の未対応"
    } else {
        "最初の error"
    };
    format!("。{kind}: {} {line}", d.code)
}

/// 実行しなかった・失敗したときの 1 行（`globe simulate` の最後の行と同じ。S184。成功なら無い）。
pub fn refused_summary(model: &str, reason: &str, diagnostics: &[Diagnostic]) -> String {
    format!("{model}: {reason}{}", first_error(diagnostics))
}

impl CheckResponse {
    /// 検査の結果の 1 行（`globe check` の最後の行。通った段階か、止まった段階。S184）。
    pub fn summary_line(&self) -> String {
        let model = &self.model;
        if let Some(stage) = self.failed_stage {
            return format!("{model}: NG（{stage:?} {} で停止）", stage.title());
        }
        let through = if self.s10.is_some() && self.s9.is_some() {
            "S0〜S10 を通過"
        } else if self.s8.is_some() {
            "S0〜S8 を通過"
        } else if self.s7.is_some() {
            "S0〜S7 を通過。S8 は判定できなかった"
        } else if self.s6.is_some() {
            "S0〜S6 を通過。S7 以降は判定できなかった"
        } else {
            "S0〜S5 を通過。S6 以降は判定できなかった"
        };
        format!("{model}: OK（{through}）")
    }

    /// 検査に入る前に失敗した（ライブラリを読めないなど）ときの結果。段階は Error の最初の段階（無ければ S1）。
    pub fn failed(model: impl Into<String>, diagnostics: Vec<Diagnostic>) -> Self {
        let failed_stage = diagnostics
            .iter()
            .filter(|d| d.is_error())
            .map(|d| d.stage)
            .min()
            .unwrap_or(Stage::S1);
        let mut response = CheckResponse {
            model: model.into(),
            passed: false,
            failed_stage: Some(failed_stage),
            balance: None,
            dae: None,
            structure: None,
            s6: None,
            s7: None,
            s8: None,
            s9: None,
            s10: None,
            experiment: None,
            init: None,
            flat: None,
            unsupported: unsupported_summary(&diagnostics),
            can_force: force_helps(&diagnostics),
            diagnostics,
            notes: Vec::new(),
            summary: String::new(),
        };
        response.summary = response.summary_line();
        response
    }
}

/// experiment annotation の値（書かれていない項目と数の字面でない項目は None）。
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct ExperimentValues {
    pub start_time: Option<f64>,
    pub stop_time: Option<f64>,
    pub tolerance: Option<f64>,
    pub interval: Option<f64>,
}

impl From<&Experiment> for ExperimentValues {
    fn from(e: &Experiment) -> Self {
        let get = |b: Option<Bits>| b.map(Bits::get);
        ExperimentValues {
            start_time: get(e.start_time),
            stop_time: get(e.stop_time),
            tolerance: get(e.tolerance),
            interval: get(e.interval),
        }
    }
}

/// DAE の大きさ。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct StructureCounts {
    pub unknowns: usize,
    pub equations: usize,
    pub derivatives: usize,
}

/// 実行の依頼（wasm の `Request`、`globe simulate` の引数と同じ）。
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SimulateRequest {
    /// 実行の前にワークスペースに足す（差し替える）ファイル（名前 → 本文）。ライブラリを文書として入れてもよい
    #[serde(default)]
    pub files: BTreeMap<String, String>,
    /// モデルの完全修飾名
    pub model: String,
    /// 実行の設定（None の項目は experiment annotation か既定値）
    #[serde(default)]
    pub settings: SimSettings,
    /// S0〜S10 に Error があっても実行する
    #[serde(default)]
    pub force: bool,
    /// 結果の CSV を返さない（大きな結果で JSON を小さくするため。最後の値は `final` に入る）
    #[serde(default)]
    pub no_csv: bool,
}

/// 実行の結果（wasm の `Response` と同じ形）。
#[derive(Debug, Clone, Serialize)]
pub struct SimulateResponse {
    pub model: String,
    /// 結果があり、Error の診断が無い
    pub ok: bool,
    /// 実行しなかった・失敗した理由（成功なら None）
    pub refused: Option<String>,
    pub diagnostics: Vec<Diagnostic>,
    pub settings: Option<ResolvedSettings>,
    pub termination: Option<Termination>,
    pub stats: Option<SimStats>,
    /// 最後の時刻の値（変数名 → 値）
    #[serde(rename = "final")]
    pub final_values: Option<BTreeMap<String, f64>>,
    /// 結果の CSV（`globe_mode_sim::store::write_csv` と同じ形。`globe simulate -o` と同じ）
    pub csv: Option<String>,
    /// 段階ごとの壁時計（ミリ秒）: load / check / compile / run
    pub timings_ms: BTreeMap<String, f64>,
    /// 返す時点の wasm の線形メモリ（バイト。ネイティブでは 0）
    pub memory_bytes: f64,
    /// [`CheckResponse::unsupported`] と同じ（この実行の診断について）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unsupported: Option<String>,
    /// 検査の Error で実行しなかった（`refused`）が、強行なら進める見込み（[`force_helps`]）。出るのは true のときだけ
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub can_force: bool,
    /// 実行しなかった・失敗した理由の 1 行（`globe simulate` の最後の行と同じ。S184。成功なら出ない。[`refused_summary`]）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
}

/// 列ごとの結果の並び（wasm の `simulate_columns`。S051）。時系列を CSV の文字列でなく、列ごとの `Float64Array` で渡すときの
/// 見出し。`names[0]` は `time`、残りは変数で、列 `i` は `points` 個の値を持つ（CSV の行数 - 1 と同じ）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ColumnsLayout {
    pub names: Vec<String>,
    pub points: usize,
    /// プロットに既定で描く信号の名前（`time` を除く。最大 3 本。状態、足りなければ出力。規則は
    /// `Trajectory::default_plot`。S190）。無ければ（古い応答）UI は先頭の 3 本を使う。
    #[serde(default)]
    pub defaults: Vec<String>,
}

/// tunable なパラメータの一覧（`globe simulate --list-parameters --json` と同じ形）。
#[derive(Debug, Clone, Serialize)]
pub struct ParametersResponse {
    pub model: String,
    /// lowering に失敗したら None（理由は `diagnostics`）
    pub parameters: Option<Vec<TunableParameter>>,
    pub diagnostics: Vec<Diagnostic>,
}

/// モデルのパラメータ宣言の一覧（S248。`globe/parameters`・wasm の `parameters`）。
#[derive(Debug, Clone, Serialize)]
pub struct ModelParametersResponse {
    pub model: String,
    pub parameters: Vec<ModelParameter>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_response_keys_follow_the_cli_json() {
        let v = serde_json::to_value(CheckResponse::failed("M", Vec::new())).unwrap();
        let keys: Vec<&str> = v.as_object().unwrap().keys().map(String::as_str).collect();
        assert_eq!(
            keys,
            [
                "model",
                "passed",
                "failed_stage",
                "balance",
                "dae",
                "structure",
                "s6",
                "s7",
                "s8",
                "s9",
                "s10",
                "init",
                "flat",
                "diagnostics",
                "notes",
                "summary"
            ]
        );
        assert_eq!(v["failed_stage"], serde_json::to_value(Stage::S1).unwrap());
    }

    #[test]
    fn simulate_request_rejects_unknown_fields() {
        let r: SimulateRequest = serde_json::from_str(r#"{"model":"M"}"#).unwrap();
        assert!(!r.force && r.files.is_empty());
        assert!(
            serde_json::from_str::<SimulateRequest>(r#"{"model":"M","solver":"bdf"}"#).is_err()
        );
        // #83: 入れ子の settings も、綴りの誤りを黙って捨てない
        for bad in [
            r#"{"model":"M","settings":{"stoptime":10}}"#,
            r#"{"model":"M","settings":{"parameter":{"k":2}}}"#,
        ] {
            assert!(
                serde_json::from_str::<SimulateRequest>(bad).is_err(),
                "{bad}"
            );
        }
        assert!(
            serde_json::from_str::<SimulateRequest>(
                r#"{"model":"M","settings":{"stop_time":10,"parameters":{"k":2}}}"#
            )
            .is_ok()
        );
    }
}

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
    BalanceCount, DaeCounts, FlatSummary, InitStructure, Outline, ResolvedSettings, SimSettings,
    SimStats, Termination, TunableParameter,
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
    /// 初期化系の構造（位置は元の本文の中）
    pub init: Option<InitStructure>,
    pub flat: Option<FlatSummary>,
    /// 位置は元の本文の中
    pub diagnostics: Vec<Diagnostic>,
    pub notes: Vec<String>,
}

impl CheckResponse {
    /// 検査に入る前に失敗した（ライブラリを読めないなど）ときの結果。段階は Error の最初の段階（無ければ S1）。
    pub fn failed(model: impl Into<String>, diagnostics: Vec<Diagnostic>) -> Self {
        let failed_stage = diagnostics
            .iter()
            .filter(|d| d.is_error())
            .map(|d| d.stage)
            .min()
            .unwrap_or(Stage::S1);
        CheckResponse {
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
            init: None,
            flat: None,
            diagnostics,
            notes: Vec::new(),
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
}

/// 列ごとの結果の並び（wasm の `simulate_columns`。S051）。時系列を CSV の文字列でなく、列ごとの `Float64Array` で渡すときの
/// 見出し。`names[0]` は `time`、残りは変数で、列 `i` は `points` 個の値を持つ（CSV の行数 - 1 と同じ）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ColumnsLayout {
    pub names: Vec<String>,
    pub points: usize,
}

/// tunable なパラメータの一覧（`globe simulate --list-parameters --json` と同じ形）。
#[derive(Debug, Clone, Serialize)]
pub struct ParametersResponse {
    pub model: String,
    /// lowering に失敗したら None（理由は `diagnostics`）
    pub parameters: Option<Vec<TunableParameter>>,
    pub diagnostics: Vec<Diagnostic>,
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
                "notes"
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
    }
}

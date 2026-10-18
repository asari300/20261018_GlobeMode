//! `trait Frontend` と、その実装（Phase A は Rumoca のアダプタ）。
//!
//! フロントエンドの型を外に出さず、`globe-mode-ir` の中立 IR と
//! `globe-mode-core` の診断に変換して返す（ADR-0002）。

use globe_mode_core::{Diagnostic, Stage};
use globe_mode_ir::{BalanceCount, Outline};

#[cfg(feature = "rumoca")]
mod rumoca;
#[cfg(feature = "rumoca")]
pub use rumoca::RumocaFrontend;

/// 1 つのソースファイル（名前と本文）。
#[derive(Debug, Clone, Copy)]
pub struct Source<'a> {
    pub name: &'a str,
    pub text: &'a str,
}

/// モデルの検査結果（S0〜S5。S6 以降は P2 で `globe-mode-analysis` が足す）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckReport {
    pub model: String,
    /// DAE まで変換できたときの方程式数と変数数。
    pub balance: Option<BalanceCount>,
    pub diagnostics: Vec<Diagnostic>,
}

impl CheckReport {
    /// Error が 1 つも無ければ true。
    pub fn passed(&self) -> bool {
        !self.diagnostics.iter().any(Diagnostic::is_error)
    }

    /// Error が出た最初の段階。
    pub fn failed_stage(&self) -> Option<Stage> {
        self.diagnostics
            .iter()
            .filter(|d| d.is_error())
            .map(|d| d.stage)
            .min()
    }
}

/// Modelica のフロントエンド。Phase A は [`RumocaFrontend`]、Track B は自前コア（feature `native`）。
pub trait Frontend {
    /// 1 ファイルを構文解析してアウトラインを返す。構文の誤りは S0 の診断として返す。
    fn outline(&self, source: Source<'_>) -> Result<Outline, Vec<Diagnostic>>;

    /// `sources` を読み込み、`model` を DAE まで変換して検査する。
    fn check(&self, sources: &[Source<'_>], model: &str) -> CheckReport;
}

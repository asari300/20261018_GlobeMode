//! `trait Frontend` と、その実装（Phase A は Rumoca のアダプタ）。
//!
//! フロントエンドの型を外に出さず、`globe-mode-ir` の中立 IR と
//! `globe-mode-core` の診断に変換して返す（ADR-0002）。

use std::path::Path;

use globe_mode_core::{Diagnostic, Stage};
use globe_mode_ir::{
    BalanceCount, DaeCounts, DaeStructure, FlatSummary, InitStructure, Outline, PreflightStructure,
};

#[cfg(feature = "rumoca")]
mod init;
#[cfg(feature = "rumoca")]
mod preflight;
#[cfg(feature = "rumoca")]
mod rumoca;
#[cfg(feature = "rumoca")]
mod split_library;
#[cfg(feature = "rumoca")]
mod structure;
#[cfg(feature = "rumoca")]
pub mod unsupported;
#[cfg(feature = "rumoca")]
pub use rumoca::{Checker, CompiledModel, ReferenceStructure, RumocaFrontend};

/// 1 つのソースファイル（名前と本文）。
#[derive(Debug, Clone, Copy)]
pub struct Source<'a> {
    pub name: &'a str,
    pub text: &'a str,
}

/// モデルの検査結果（S0〜S5）。S6 は `globe-mode-analysis` が [`CheckReport::structure`] を読んで足す。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CheckReport {
    pub model: String,
    /// DAE まで変換できたときの方程式数と変数数（配列はスカラーに展開）。
    pub balance: Option<BalanceCount>,
    /// DAE まで変換できたときの変数の数（配列は 1 個）。
    pub dae: Option<DaeCounts>,
    /// DAE まで変換できたときの、方程式 × 未知数の構造（S6 の入力）。位置はコンパイルに渡した本文の中。
    /// 作れなかったときは None で、理由は [`CheckReport::notes`] にある。
    pub structure: Option<DaeStructure>,
    /// DAE まで変換できたときの、初期化系の構造（S8 の入力、S007）。位置はコンパイルに渡した本文の中。
    /// 作れなかったときは None で、理由は [`CheckReport::notes`] にある。
    pub init: Option<InitStructure>,
    /// DAE まで変換できたときの、プリフライトの入力（S10、S010）。位置はコンパイルに渡した本文の中。
    /// 作れなかったときは None で、理由は [`CheckReport::notes`] にある。
    pub preflight: Option<PreflightStructure>,
    /// flat モデルの要約。DAE への変換で失敗したとき（S5）も、flatten まで通れば入る。
    pub flat: Option<FlatSummary>,
    pub diagnostics: Vec<Diagnostic>,
    /// 検査を止めない補足（例: S6 の入力を作れなかった理由）。
    pub notes: Vec<String>,
}

impl CheckReport {
    /// 空の結果。
    pub fn new(model: impl Into<String>) -> Self {
        Self {
            model: model.into(),
            ..Self::default()
        }
    }

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

    /// `libraries`（MSL などのライブラリのディレクトリ）と `sources` を読み込み、
    /// `model` を DAE まで変換して検査する。モデルはライブラリの中にあってもよい。
    fn check(&self, sources: &[Source<'_>], libraries: &[&Path], model: &str) -> CheckReport;
}

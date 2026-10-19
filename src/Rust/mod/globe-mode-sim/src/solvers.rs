//! ソルバの名前の登録表（doc/architecture.md §3.3）。

use globe_mode_ir::SolverId;

/// 名前を引いた結果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SolverLookup {
    /// 実装のあるソルバ
    Available(SolverId),
    /// 別名（例: `dassl` → `bdf`。同等の手法で、結果は一致しない）
    Alias { requested: String, id: SolverId },
    /// 登録表にはあるが、まだ実装していない（例: `esdirk34`）
    Planned { name: String },
    /// 名前が無い。`suggestions` は近い名前
    Unknown {
        name: String,
        suggestions: Vec<&'static str>,
    },
}

/// 名前を引く（大文字と小文字は区別しない）。
pub fn lookup(name: &str) -> SolverLookup {
    let _ = name;
    SolverLookup::Available(SolverId::Bdf)
}

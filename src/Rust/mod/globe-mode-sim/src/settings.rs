//! 実行の設定を決める（利用者の指定 > experiment annotation > 既定値）。

use globe_mode_core::Diagnostic;
use globe_mode_ir::{Experiment, ResolvedSettings, SimSettings};

/// 設定を決める。成功なら設定と、止めない診断（GM-I10003 の別名など）を返す。
/// 決められない（ソルバが無い、区間が空など）なら Error の診断を返す。
pub fn resolve(
    requested: &SimSettings,
    experiment: Option<&Experiment>,
) -> Result<(ResolvedSettings, Vec<Diagnostic>), Vec<Diagnostic>> {
    let _ = (requested, experiment);
    Err(Vec::new())
}

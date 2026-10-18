//! DAE の構造解析（S6: 構造特異、S7: インデックス、S8: 初期化）。フロントエンドに依存せず、
//! `globe-mode-ir` の [`DaeStructure`] / `InitStructure` だけを読む。doc/architecture.md §3.2
//!
//! - 二部グラフ（方程式 × 未知数）の最大マッチング（[`hopcroft_karp`]、O(E√V)）
//! - 粗い Dulmage–Mendelsohn 分解（[`dulmage_mendelsohn`]: 過剰な方程式 / 不足している未知数 / 適定の部分）
//! - S6 の結果は [`StructuralReport`]（[`analyze`]）。診断は `GM-E6001`
//! - S7 の結果は [`IndexReport`]（[`analyze_index`]、Pantelides 法）。診断は `GM-I7001` / `GM-I7002` / `GM-E7001`
//! - S8 の結果は [`InitReport`]（[`analyze_init`]）
//!
//! S6 では二部グラフを 2 つ見る。
//!
//! - **まとめた形**（`incidence ∪ state_refs`）: 状態 `x` と微分 `der(x)` を 1 つの列にまとめる。
//!   ここで完全マッチングが無ければ構造特異（Pantelides 法が失敗するのと同じ条件）で、モデルの誤り（`GM-E6001`）。
//! - **der の形**（`incidence` だけ）: 状態を既知とみなす。まとめた形では完全でも、ここで完全マッチングが
//!   無ければ、微分インデックスが 1 より大きい見込み（[`StructuralReport::index_reduction_needed`]。
//!   モデルの誤りではない。S7 の情報は [`IndexReport`] が出す）。
//!   Rumoca の構造解析（`phase_structural::analyze`）はこちらを数えるので、突き合わせにはこちらを使う。
//!
//! wasm32 でも動くように、依存は core と ir と serde だけにする。

mod bipartite;
mod dm;
pub mod init;
mod matching;
pub mod pantelides;
#[cfg(test)]
mod testutil;

use globe_mode_core::{Diagnostic, Stage, TextRange, codes};
use globe_mode_ir::{DaeStructure, UnknownKind};
use serde::Serialize;

use crate::bipartite::Bipartite;
pub use crate::dm::{Decomposition, dulmage_mendelsohn};
pub use crate::init::{InitReport, analyze_init};
pub use crate::matching::{Matching, hopcroft_karp};
pub use crate::pantelides::{IndexReport, analyze_index};

/// 診断の補足に並べる項目の上限。超えた分は「ほか k 件」にまとめる。
pub const NOTE_LIST_LIMIT: usize = 10;

/// S6 の結果。添字は [`DaeStructure`] の `equations` / `unknowns` のもの（すべて昇順）。
///
/// 位置を持たないので、[`DaeStructure::without_ranges`] を渡しても同じ値になる（防火壁 2、ADR-0005）。
/// 位置は [`StructuralReport::diagnostics`] が `DaeStructure` から引く。
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize)]
pub struct StructuralReport {
    pub n_equations: usize,
    pub n_unknowns: usize,
    /// der の形（`incidence` だけ）の最大マッチングの大きさ。Rumoca の構造解析と突き合わせる値
    pub matching: usize,
    /// まとめた形（`incidence ∪ state_refs`）の最大マッチングの大きさ。構造特異の判定に使う
    pub structural_matching: usize,
    /// まとめた形では完全マッチングがあるが、der の形では無い（微分インデックスが 1 より大きい見込み）
    pub index_reduction_needed: bool,
    /// まとめた形でマッチしなかった方程式（添字）
    pub unmatched_equations: Vec<usize>,
    /// まとめた形でマッチしなかった未知数（添字）
    pub unmatched_unknowns: Vec<usize>,
    /// der の形でマッチしなかった方程式（S7 の情報の補足に使う）
    pub der_unmatched_equations: Vec<usize>,
    /// 過剰な部分（まとめた形の DM 分解で、マッチしなかった方程式から交互路で届く方程式）
    pub over_equations: Vec<usize>,
    /// 過剰な部分の未知数（過剰な方程式が含む未知数）
    pub over_unknowns: Vec<usize>,
    /// 不足している部分の方程式（不足している未知数を含む方程式）
    pub under_equations: Vec<usize>,
    /// 不足している部分（マッチしなかった未知数から交互路で届く未知数）
    pub under_unknowns: Vec<usize>,
}

impl StructuralReport {
    /// 構造特異でない（方程式数 = 未知数で、まとめた形に完全マッチングがある）。
    ///
    /// true でも [`Self::index_reduction_needed`] なら、S7（[`analyze_index`]）で方程式を微分する
    /// （[`IndexReport::index`] が 2 以上になる）。
    pub fn is_regular(&self) -> bool {
        self.n_equations == self.n_unknowns && self.structural_matching == self.n_equations
    }

    /// 診断（S6）。
    ///
    /// - 構造特異なら、すべて `GM-E6001`（S6、Error）で、次の順に並べる。
    ///   1. 方程式数と未知数が違えば、その要約（位置なし）
    ///   2. マッチしなかった方程式ごとに 1 件（方程式の位置。補足に過剰な部分の未知数と方程式）
    ///   3. マッチしなかった未知数ごとに 1 件（宣言の位置。補足に不足している部分の方程式と未知数）
    /// - 構造特異でなければ空。der の形でだけ完全マッチングが無くても、ここでは出さない
    ///   （S006 から、S7 の情報 `GM-I7001` は [`IndexReport::diagnostics`] が出す）。
    ///
    /// `structure` は、この結果を作ったときと同じ（位置のある）`DaeStructure` を渡す。
    /// 添字が範囲外なら、名前の代わりに `#添字` と書く。
    pub fn diagnostics(&self, structure: &DaeStructure) -> Vec<Diagnostic> {
        let equation = |i: usize| {
            structure
                .equations
                .get(i)
                .map(|e| e.label.as_str())
                .filter(|s| !s.is_empty())
                .map_or_else(|| format!("#{i}"), str::to_string)
        };
        let unknown = |i: usize| {
            structure
                .unknowns
                .get(i)
                .map(|u| u.name.as_str())
                .filter(|s| !s.is_empty())
                .map_or_else(|| format!("#{i}"), str::to_string)
        };

        if self.is_regular() {
            return Vec::new();
        }

        let mut out = Vec::new();
        if self.n_equations != self.n_unknowns {
            out.push(
                Diagnostic::error(
                    codes::SINGULAR,
                    Stage::S6,
                    format!(
                        "方程式 {} / 未知数 {} で数が合わず、方程式と未知数を 1 対 1 に対応づけられない",
                        self.n_equations, self.n_unknowns
                    ),
                )
                .note(format!(
                    "最大マッチング {}: 対応のない方程式 {} 本、対応のない未知数 {} 個",
                    self.structural_matching,
                    self.unmatched_equations.len(),
                    self.unmatched_unknowns.len()
                )),
            );
        }

        if !self.unmatched_equations.is_empty() {
            let unknowns_note = format!(
                "過剰な部分の未知数（{} 個）: {}",
                self.over_unknowns.len(),
                list(&self.over_unknowns, unknown)
            );
            let equations_note = format!(
                "過剰な部分の方程式（{} 本）: {}",
                self.over_equations.len(),
                list(&self.over_equations, equation)
            );
            for &e in &self.unmatched_equations {
                let row = structure.equations.get(e);
                let d = Diagnostic::error(
                    codes::SINGULAR,
                    Stage::S6,
                    format!(
                        "方程式 `{}` はどの未知数とも対応づけられない（過剰な方程式）",
                        equation(e)
                    ),
                );
                out.push(
                    locate(
                        d,
                        row.and_then(|r| r.file.as_deref()),
                        row.and_then(|r| r.range),
                    )
                    .note(unknowns_note.clone())
                    .note(equations_note.clone()),
                );
            }
        }

        if !self.unmatched_unknowns.is_empty() {
            // どの方程式にも現れない未知数は、補足でそう言う（宣言だけして式を書き忘れた、のよくある形）。
            // 状態 x だけが現れる方程式も「現れる」に数える（まとめた形で見る）
            let mut appears = vec![false; structure.unknowns.len()];
            let n = structure.equations.len();
            for rows in [&structure.incidence, &structure.state_refs] {
                for &u in rows.iter().take(n).flatten() {
                    if let Some(a) = appears.get_mut(u as usize) {
                        *a = true;
                    }
                }
            }
            let equations_note = (!self.under_equations.is_empty()).then(|| {
                format!(
                    "不足している部分の方程式（{} 本）: {}",
                    self.under_equations.len(),
                    list(&self.under_equations, equation)
                )
            });
            let unknowns_note = format!(
                "不足している部分の未知数（{} 個）: {}",
                self.under_unknowns.len(),
                list(&self.under_unknowns, unknown)
            );
            for &u in &self.unmatched_unknowns {
                let decl = structure.unknowns.get(u);
                let mut d = locate(
                    Diagnostic::error(
                        codes::SINGULAR,
                        Stage::S6,
                        format!("未知数 `{}` を決める方程式が足りない", unknown(u)),
                    ),
                    decl.and_then(|x| x.file.as_deref()),
                    decl.and_then(|x| x.range),
                );
                if !appears.get(u).copied().unwrap_or(false) {
                    d = d.note("この未知数はどの方程式にも現れない");
                }
                if decl.is_some_and(|x| x.kind == UnknownKind::Derivative) {
                    d = d.note("状態とその微分（x と der(x)）は 1 つの未知数として数える");
                }
                if let Some(n) = &equations_note {
                    d = d.note(n.clone());
                }
                out.push(d.note(unknowns_note.clone()));
            }
        }
        out
    }
}

/// [`DaeStructure`] の S6 の解析（Hopcroft–Karp と粗い DM 分解）。
///
/// まとめた形（`incidence ∪ state_refs`）で構造特異を判定し、DM 分解もまとめた形で作る。
/// der の形（`incidence` だけ）のマッチングは、微分インデックスの見込み（S7）と Rumoca との突き合わせに使う。
///
/// 位置は見ないので、結果は `structure.without_ranges()` を渡しても同じ。
/// incidence と state_refs の範囲外の添字や、足りない行は捨てる（フロントエンドの不具合で止めない）。
pub fn analyze(structure: &DaeStructure) -> StructuralReport {
    let n = structure.equations.len();
    let m = structure.unknowns.len();
    let der_form = matching::maximum_matching(&Bipartite::new(n, m, &structure.incidence));
    // DaeStructure::merged_incidence と同じ和だが、incidence の行が足りない方程式の state_refs も拾う
    let merged = Bipartite::union(n, m, &[&structure.incidence, &structure.state_refs]);
    let structural = matching::maximum_matching(&merged);
    let dm = dm::decompose(&merged, &structural);
    let mut report = StructuralReport {
        n_equations: n,
        n_unknowns: m,
        matching: der_form.size,
        structural_matching: structural.size,
        index_reduction_needed: false,
        unmatched_equations: structural.unmatched_equations(),
        unmatched_unknowns: structural.unmatched_unknowns(),
        der_unmatched_equations: der_form.unmatched_equations(),
        over_equations: dm.over_equations,
        over_unknowns: dm.over_unknowns,
        under_equations: dm.under_equations,
        under_unknowns: dm.under_unknowns,
    };
    report.index_reduction_needed = report.is_regular() && report.matching < n;
    report
}

/// 位置を付ける。ファイルが分からなくても範囲だけは残す。
pub(crate) fn locate(
    mut d: Diagnostic,
    file: Option<&str>,
    range: Option<TextRange>,
) -> Diagnostic {
    match file {
        Some(f) => d.at(f, range),
        None => {
            d.range = range;
            d
        }
    }
}

/// 名前を `` `a`, `b` `` の形に並べる。[`NOTE_LIST_LIMIT`] を超えた分は「ほか k 件」にまとめる。
pub(crate) fn list(items: &[usize], name: impl Fn(usize) -> String) -> String {
    list_by(items, |i| format!("`{}`", name(i)))
}

/// 項目を `item` で書いて `, ` で並べる（[`list`] の、書き方を決められる版）。
pub(crate) fn list_by(items: &[usize], item: impl Fn(usize) -> String) -> String {
    if items.is_empty() {
        return "なし".to_string();
    }
    let mut s = items
        .iter()
        .take(NOTE_LIST_LIMIT)
        .map(|&i| item(i))
        .collect::<Vec<_>>()
        .join(", ");
    if items.len() > NOTE_LIST_LIMIT {
        s.push_str(&format!(" ほか {} 件", items.len() - NOTE_LIST_LIMIT));
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::{FILE, XorShift, equation, structure, unknown, with_state_refs};
    use globe_mode_core::Severity;

    /// HighIndexDuplicate.mo と同じ形: C * der(vC) = i; V = vR + vC; vR = V - vC;
    fn kvl_written_twice() -> DaeStructure {
        with_state_refs(
            structure(
                &["der(vC)", "vR", "i"],
                &[
                    ("C * der(vC) = i", &[0, 2]),
                    ("V = vR + vC", &[1]),
                    ("vR = V - vC", &[1]),
                ],
            ),
            &[&[], &[0], &[0]],
        )
    }

    #[test]
    fn chain_has_a_perfect_matching() {
        // x + y = 1; y + z = 2; z = 3;
        let s = structure(
            &["x", "y", "z"],
            &[
                ("x + y = 1", &[0, 1]),
                ("y + z = 2", &[1, 2]),
                ("z = 3", &[2]),
            ],
        );
        let r = analyze(&s);
        assert!(r.is_regular() && !r.index_reduction_needed);
        assert_eq!((r.n_equations, r.n_unknowns), (3, 3));
        assert_eq!((r.matching, r.structural_matching), (3, 3));
        assert!(r.unmatched_equations.is_empty() && r.unmatched_unknowns.is_empty());
        assert!(r.der_unmatched_equations.is_empty());
        assert!(r.over_equations.is_empty() && r.under_unknowns.is_empty());
        assert!(r.diagnostics(&s).is_empty());
    }

    #[test]
    fn empty_structure_is_regular() {
        let s = DaeStructure::default();
        let r = analyze(&s);
        assert!(r.is_regular());
        assert_eq!(r, StructuralReport::default());
        assert!(r.diagnostics(&s).is_empty());
    }

    #[test]
    fn square_but_singular() {
        // x = 1; x = 2;（未知数 x, y）: 数は合うが、y を決める方程式が無い
        let s = structure(&["x", "y"], &[("x = 1", &[0]), ("x = 2", &[0])]);
        let r = analyze(&s);
        assert!(s.is_square());
        assert!(!r.is_regular() && !r.index_reduction_needed);
        assert_eq!((r.matching, r.structural_matching), (1, 1));
        assert_eq!(r.unmatched_equations.len(), 1);
        assert_eq!(r.unmatched_unknowns, [1]);
        // 対応のない方程式がどちらでも、交互路で両方に届く
        assert_eq!(r.over_equations, [0, 1]);
        assert_eq!(r.over_unknowns, [0]);
        assert_eq!(r.under_unknowns, [1]);
        assert!(r.under_equations.is_empty());

        let ds = r.diagnostics(&s);
        assert_eq!(ds.len(), 2, "{ds:#?}");
        assert!(ds.iter().all(|d| d.code == codes::SINGULAR
            && d.stage == Stage::S6
            && d.severity == Severity::Error));
        // 1 件目: 対応のない方程式（位置付き）
        let e = r.unmatched_equations[0];
        let label = &s.equations[e].label;
        assert_eq!(
            ds[0].message,
            format!("方程式 `{label}` はどの未知数とも対応づけられない（過剰な方程式）")
        );
        assert_eq!(ds[0].file.as_deref(), Some(FILE));
        assert_eq!(ds[0].range, s.equations[e].range);
        assert_eq!(
            ds[0].notes,
            [
                "過剰な部分の未知数（1 個）: `x`",
                "過剰な部分の方程式（2 本）: `x = 1`, `x = 2`"
            ]
        );
        // 2 件目: 決まらない未知数（宣言の位置付き）
        assert_eq!(ds[1].message, "未知数 `y` を決める方程式が足りない");
        assert_eq!(ds[1].range, s.unknowns[1].range);
        assert_eq!(
            ds[1].notes,
            [
                "この未知数はどの方程式にも現れない",
                "不足している部分の未知数（1 個）: `y`"
            ]
        );
    }

    #[test]
    fn index_two_is_information_not_an_error() {
        // der(x) = y; x = sin(time);（未知数 der(x), y）。2 本目は状態 x だけを含む
        let s = with_state_refs(
            structure(
                &["der(x)", "y"],
                &[("der(x) = y", &[0, 1]), ("x = sin(time)", &[])],
            ),
            &[&[], &[0]],
        );
        let r = analyze(&s);
        assert!(r.is_regular());
        assert_eq!((r.matching, r.structural_matching), (1, 2));
        assert!(r.index_reduction_needed);
        assert_eq!(r.der_unmatched_equations, [1]);
        // まとめた形では完全なので、過剰・不足は無い
        assert!(r.unmatched_equations.is_empty() && r.unmatched_unknowns.is_empty());
        assert!(r.over_equations.is_empty() && r.under_unknowns.is_empty());

        // S6 の診断は出さない。S7 の情報（GM-I7001）は IndexReport が出す
        assert!(r.diagnostics(&s).is_empty());
        let index = analyze_index(&s);
        assert_eq!(index.index, 2);
        let ds = index.diagnostics(&s);
        assert!(ds.iter().all(|d| !d.is_error() && d.stage == Stage::S7));
        let d = ds
            .iter()
            .find(|d| d.code == codes::HIGH_INDEX)
            .expect("GM-I7001");
        assert_eq!(d.severity, Severity::Info);
        assert_eq!(d.range, s.equations[1].range);
    }

    #[test]
    fn singular_with_a_state() {
        // x = 1; x = 2; y + z = der(z);（未知数 x, y, der(z)）。z を含んでも、まとめた形で特異
        let s = with_state_refs(
            structure(
                &["x", "y", "der(z)"],
                &[
                    ("x = 1", &[0]),
                    ("x = 2", &[0]),
                    ("y + z = der(z)", &[1, 2]),
                ],
            ),
            &[&[], &[], &[2]],
        );
        let r = analyze(&s);
        assert!(s.is_square() && !r.is_regular() && !r.index_reduction_needed);
        assert_eq!((r.matching, r.structural_matching), (2, 2));
        assert_eq!(r.over_equations, [0, 1]);
        assert_eq!(r.over_unknowns, [0]);
        assert_eq!(r.under_equations, [2]);
        assert_eq!(r.under_unknowns, [1, 2]);

        let ds = r.diagnostics(&s);
        assert_eq!(ds.len(), 2, "{ds:#?}");
        assert!(ds.iter().all(|d| d.code == codes::SINGULAR && d.is_error()));
        assert!(!ds.iter().any(|d| d.code == codes::HIGH_INDEX));
        let e = r.unmatched_equations[0];
        assert!(e == 0 || e == 1);
        assert!(ds[0].message.contains("過剰な方程式"));
        assert_eq!(ds[0].range, s.equations[e].range);
        let u = r.unmatched_unknowns[0];
        assert!(u == 1 || u == 2);
        assert_eq!(
            ds[1].message,
            format!("未知数 `{}` を決める方程式が足りない", s.unknowns[u].name)
        );
        assert!(
            ds[1]
                .notes
                .contains(&"不足している部分の方程式（1 本）: `y + z = der(z)`".to_string())
        );
    }

    #[test]
    fn over_determined_non_square() {
        // 方程式 3 本 × 未知数 2 個: x = 1; y = 2; x + y = 3;
        let s = structure(
            &["x", "y"],
            &[("x = 1", &[0]), ("y = 2", &[1]), ("x + y = 3", &[0, 1])],
        );
        let r = analyze(&s);
        assert!(!s.is_square() && !r.is_regular() && !r.index_reduction_needed);
        assert_eq!(r.structural_matching, 2);
        assert_eq!(r.unmatched_equations, [2]);
        assert!(r.unmatched_unknowns.is_empty());
        assert_eq!(r.over_equations, [0, 1, 2]);
        assert_eq!(r.over_unknowns, [0, 1]);
        assert!(r.under_unknowns.is_empty() && r.under_equations.is_empty());

        let ds = r.diagnostics(&s);
        assert_eq!(ds.len(), 2);
        // 数が違うときは、要約が先頭（位置なし）
        assert_eq!(
            ds[0].message,
            "方程式 3 / 未知数 2 で数が合わず、方程式と未知数を 1 対 1 に対応づけられない"
        );
        assert_eq!(ds[0].file, None);
        assert_eq!(
            ds[0].notes,
            ["最大マッチング 2: 対応のない方程式 1 本、対応のない未知数 0 個"]
        );
        assert!(ds[1].message.contains("`x + y = 3`"));
        assert_eq!(ds[1].range, s.equations[2].range);
    }

    #[test]
    fn under_determined_non_square() {
        // 方程式 1 本 × 未知数 3 個: x + y = 1;（z はどこにも現れない）
        let s = structure(&["x", "y", "z"], &[("x + y = 1", &[0, 1])]);
        let r = analyze(&s);
        assert_eq!(r.structural_matching, 1);
        assert!(r.unmatched_equations.is_empty());
        assert_eq!(r.unmatched_unknowns.len(), 2);
        assert!(r.unmatched_unknowns.contains(&2));
        assert!(r.over_equations.is_empty() && r.over_unknowns.is_empty());
        assert_eq!(r.under_equations, [0]);
        assert_eq!(r.under_unknowns, [0, 1, 2]);

        let ds = r.diagnostics(&s);
        assert_eq!(ds.len(), 3);
        assert!(ds[0].message.starts_with("方程式 1 / 未知数 3"));
        // 未知数の診断は添字の順
        let names: Vec<&str> = ds[1..].iter().map(|d| d.message.as_str()).collect();
        let expected: Vec<String> = r
            .unmatched_unknowns
            .iter()
            .map(|&u| format!("未知数 `{}` を決める方程式が足りない", s.unknowns[u].name))
            .collect();
        assert_eq!(names, expected);
        let z = ds.last().unwrap();
        assert_eq!(z.notes[0], "この未知数はどの方程式にも現れない");
        assert!(
            z.notes
                .contains(&"不足している部分の方程式（1 本）: `x + y = 1`".to_string())
        );
        // x / y のどちらかは方程式に現れるので、「現れない」とは言わない
        assert_ne!(ds[1].notes[0], "この未知数はどの方程式にも現れない");
    }

    #[test]
    fn state_only_reference_counts_as_appearing() {
        // x + y = 0; だけ（未知数 y, der(x)）。der(x) は der の形では現れないが、状態 x として現れる。
        // まとめた形の行は {y, der(x)} で、y が先に対応づくので der(x) が残る
        let s = with_state_refs(
            structure(&["y", "der(x)", "der(z)"], &[("x + y = 0", &[0])]),
            &[&[1]],
        );
        let r = analyze(&s);
        assert!(!r.is_regular());
        assert_eq!(r.unmatched_unknowns, [1, 2]);
        let ds = r.diagnostics(&s);
        let find = |name: &str| {
            ds.iter()
                .find(|d| d.message == format!("未知数 `{name}` を決める方程式が足りない"))
                .unwrap_or_else(|| panic!("{name} の診断が無い: {ds:#?}"))
        };
        let absent = "この未知数はどの方程式にも現れない".to_string();
        let merged = "状態とその微分（x と der(x)）は 1 つの未知数として数える".to_string();
        // der(x) は状態として方程式に現れるので「現れない」とは言わない。der(z) はどこにも無い
        assert!(!find("der(x)").notes.contains(&absent));
        assert!(find("der(x)").notes.contains(&merged));
        assert!(find("der(z)").notes.contains(&absent));
    }

    #[test]
    fn regular_but_high_index_duplicate_kvl() {
        // src/Modelica/GlobeModeValidation/Invalid/HighIndexDuplicate.mo と同じ形。
        // まとめた形では完全（S6 は通る）が、der の形では vR だけの方程式が 2 本で詰まる（S7 の情報）
        let s = kvl_written_twice();
        let r = analyze(&s);
        assert!(s.is_square() && r.is_regular());
        assert_eq!((r.matching, r.structural_matching), (2, 3));
        assert!(r.index_reduction_needed);
        assert_eq!(r.der_unmatched_equations.len(), 1);
        assert!(r.over_equations.is_empty() && r.under_unknowns.is_empty());
        // S6 は通る。S7 の情報は IndexReport が出す
        assert!(r.diagnostics(&s).is_empty());
        assert_eq!(analyze_index(&s).index, 2);
        // state_refs が無ければ（状態を見なければ）構造特異と誤判定する: S005 の MSL ファネルで見つけた誤り
        let mut no_refs = s.clone();
        no_refs.state_refs.clear();
        assert!(!analyze(&no_refs).is_regular());
    }

    #[test]
    fn notes_are_capped() {
        // 15 本の方程式が同じ 1 個の未知数だけを含む
        let labels: Vec<String> = (0..15).map(|k| format!("x = {k}")).collect();
        let rows: Vec<(&str, &[u32])> = labels.iter().map(|l| (l.as_str(), &[0u32][..])).collect();
        let s = structure(&["x"], &rows);
        let r = analyze(&s);
        assert_eq!(r.unmatched_equations.len(), 14);
        let ds = r.diagnostics(&s);
        assert_eq!(ds.len(), 1 + 14);
        let note = &ds[1].notes[1];
        assert!(
            note.starts_with("過剰な部分の方程式（15 本）: `x = 0`, "),
            "{note}"
        );
        assert!(note.ends_with("`x = 9` ほか 5 件"), "{note}");
        assert_eq!(note.matches('`').count(), 2 * NOTE_LIST_LIMIT);
    }

    #[test]
    fn states_only_constraints_are_regular_but_not_der_matched() {
        // x_j = sin(time)（j = 0..12）: どの方程式も状態 x_j だけを含む。
        // まとめた形は完全（方程式 j ↔ der(x_j)）、der の形では 12 本すべてが対応を持たない
        let names: Vec<String> = (0..12).map(|j| format!("der(x{j})")).collect();
        let labels: Vec<String> = (0..12).map(|j| format!("x{j} = sin(time)")).collect();
        let s = DaeStructure {
            unknowns: names
                .iter()
                .enumerate()
                .map(|(i, n)| unknown(i, n))
                .collect(),
            equations: labels
                .iter()
                .enumerate()
                .map(|(i, l)| equation(i, l))
                .collect(),
            incidence: vec![Vec::new(); 12],
            state_refs: (0..12u32).map(|j| vec![j]).collect(),
        };
        let r = analyze(&s);
        assert!(r.is_regular() && r.index_reduction_needed);
        assert_eq!((r.matching, r.structural_matching), (0, 12));
        assert_eq!(r.der_unmatched_equations.len(), 12);
        assert!(r.diagnostics(&s).is_empty());
    }

    #[test]
    fn report_does_not_depend_on_positions() {
        for s in [
            kvl_written_twice(),
            with_state_refs(
                structure(
                    &["x", "y", "der(z)"],
                    &[
                        ("x = 1", &[0]),
                        ("x = 2", &[0]),
                        ("y + z = der(z)", &[1, 2]),
                    ],
                ),
                &[&[], &[], &[2]],
            ),
        ] {
            let stripped = s.without_ranges();
            assert_ne!(s, stripped);
            assert_eq!(analyze(&s), analyze(&stripped));
            // 位置を落とした構造からの診断は、位置だけが無い
            let r = analyze(&s);
            let with = r.diagnostics(&s);
            let without = r.diagnostics(&stripped);
            assert_eq!(with.len(), without.len());
            for (a, b) in with.iter().zip(&without) {
                assert_eq!(
                    (&a.message, &a.notes, &a.file),
                    (&b.message, &b.notes, &b.file)
                );
                assert_eq!(a.range.is_some(), a.code == codes::SINGULAR);
                assert!(b.range.is_none());
            }
        }
    }

    #[test]
    fn malformed_structure_does_not_panic() {
        // incidence の行が足りない・範囲外の添字・報告と構造の食い違いでも panic しない
        let mut s = structure(&["x", "y"], &[("x = 1", &[0, 7]), ("", &[])]);
        s.incidence.truncate(1);
        let r = analyze(&s);
        assert_eq!(r.structural_matching, 1);
        assert_eq!(r.unmatched_equations, [1]);
        let ds = r.diagnostics(&s);
        // ラベルが空なら添字で書く
        assert!(ds.iter().any(|d| d.message.contains("`#1`")));
        // 別の（小さな）構造を渡しても panic せず、名前は添字になる
        let ds = r.diagnostics(&DaeStructure::default());
        assert!(ds.iter().any(|d| d.message.contains("`#1`")));
        assert!(ds.iter().all(|d| d.file.is_none() && d.range.is_none()));
    }

    #[test]
    fn malformed_state_refs_do_not_panic() {
        // state_refs が方程式より短い・長い・範囲外の添字を含む
        let base = structure(
            &["der(x)", "y"],
            &[("der(x) = y", &[0, 1]), ("x = sin(time)", &[])],
        );
        for refs in [
            vec![],
            vec![vec![]],
            vec![vec![], vec![0, 0, 99, u32::MAX]],
            vec![vec![], vec![0], vec![1], vec![5]],
        ] {
            let mut s = base.clone();
            s.state_refs = refs;
            let r = analyze(&s);
            let _ = r.diagnostics(&s);
            assert_eq!(analyze(&s.without_ranges()), r);
        }
        // 範囲外を捨てたうえで state_refs が効く
        let mut s = base.clone();
        s.state_refs = vec![vec![], vec![0, 0, 99, u32::MAX]];
        assert!(analyze(&s).index_reduction_needed);
        // incidence の行が足りない方程式でも、state_refs の行は拾う
        let mut s = base.clone();
        s.incidence.truncate(1);
        s.state_refs = vec![vec![], vec![0]];
        let r = analyze(&s);
        assert!(r.is_regular() && r.index_reduction_needed);
    }

    #[test]
    fn agrees_with_matchings_on_random_structures() {
        let mut rng = XorShift::new(0x57A7_E5ED);
        let mut high_index = 0;
        for _ in 0..1000 {
            let (n, m, incidence) = rng.bipartite(7, 7);
            let (_, _, state_refs) = rng.bipartite_sized(n, m);
            let s = DaeStructure {
                unknowns: (0..m).map(|i| unknown(i, &format!("u{i}"))).collect(),
                equations: (0..n).map(|i| equation(i, &format!("e{i}"))).collect(),
                incidence,
                state_refs,
            };
            let r = analyze(&s);
            let der_form = hopcroft_karp(n, m, &s.incidence);
            let merged = hopcroft_karp(n, m, &s.merged_incidence());
            assert_eq!(r.matching, der_form.size);
            assert_eq!(r.structural_matching, merged.size);
            // 列をまとめると辺が増えるだけなので、マッチングは小さくならない
            assert!(r.structural_matching >= r.matching);
            assert_eq!(r.der_unmatched_equations.len(), n - r.matching);
            let dm = dulmage_mendelsohn(&s.merged_incidence(), &merged);
            assert_eq!(r.over_equations, dm.over_equations);
            assert_eq!(r.over_unknowns, dm.over_unknowns);
            assert_eq!(r.under_equations, dm.under_equations);
            assert_eq!(r.under_unknowns, dm.under_unknowns);
            assert_eq!(r.index_reduction_needed, r.is_regular() && r.matching < n);
            assert_eq!(analyze(&s.without_ranges()), r);
            // 診断: 特異なら GM-E6001 だけ、そうでなければ空（S7 の情報は IndexReport が出す）
            let ds = r.diagnostics(&s);
            if !r.is_regular() {
                assert!(!ds.is_empty() && ds.iter().all(|d| d.code == codes::SINGULAR));
                let expected =
                    r.unmatched_equations.len() + r.unmatched_unknowns.len() + usize::from(n != m);
                assert_eq!(ds.len(), expected);
            } else {
                high_index += usize::from(r.index_reduction_needed);
                assert!(ds.is_empty());
            }
        }
        assert!(
            high_index > 20,
            "高インデックスの例が {high_index} 件しかない"
        );
    }

    #[test]
    fn report_serializes() {
        let s = structure(&["x", "y"], &[("x = 1", &[0]), ("x = 2", &[0])]);
        let r = analyze(&s);
        // serde の derive が付いていること（JSON の出力は CLI が担う）
        fn is_serialize<T: Serialize>(_: &T) {}
        is_serialize(&r);
        is_serialize(&hopcroft_karp(1, 1, &[vec![0]]));
        is_serialize(&dulmage_mendelsohn(
            &[vec![0]],
            &hopcroft_karp(1, 1, &[vec![0]]),
        ));
    }
}

//! S8: 初期化系の検査（S007）。
//!
//! 入力は [`InitStructure`]（状態・パラメータ・離散変数・代数変数の `fixed` / `start` / 束縛と、初期方程式の数）。
//! 判定は数と属性だけで行う（どの初期方程式がどの変数を決めるかは見ない。S008 以降の候補）。
//!
//! - 要る初期条件の数 = 動的な状態の数（S7。無ければ状態の数）+ 離散変数の数 + `fixed = false` で束縛の無い
//!   パラメータの数
//! - 与えられた初期条件の数 = 初期方程式の数 + `fixed = true` の状態・離散変数・代数変数の数
//! - 多ければ `GM-E8001`（Error）。ただし離散変数が 1 つでもあるか、インデックスの低減で動的な状態が
//!   状態の数より少ない（余った fixed が矛盾しないかは数値で決まる。MSL の `GenerationOfFMUs` など）なら、
//!   数が確定しないので `GM-W8004`（Warning）
//! - 少なければ `GM-W8001`（Warning。不足分は start 値を推定値として使う見込み）。ただし不足の数が
//!   `fixed = false` の離散変数の数以下なら、「足りない見込み」の柔らかい警告にする（下の約束）
//! - 足りないと確定するときだけ、start の無い状態（`fixed = false`）を `GM-W8002` で示す
//! - 値の無いパラメータ（`fixed = true` で、束縛も start も無い）は `GM-W8003`。数が合っていても出す
//!
//! 数え方の約束:
//!
//! - インデックスが 2 以上のモデルでは、状態の一部（dummy derivative）は微分した拘束で決まり、初期条件が要らない。
//!   要る数には S7 の動的な状態の数（`IndexReport::dynamic_states`）を使う（[`analyze_init_with_index`]）。
//!   S7 の結果が無いとき（[`analyze_init`]）は、状態の数を使う。
//! - `fixed = true` の状態の start は、初期方程式とは別に 1 つと数える。Rumoca v0.10.2 の DAE への変換は
//!   それを初期方程式に変えない（`globe-mode-frontend` の `init.rs` のテストで確かめた）ので、2 重には数えない。
//! - 離散変数は、`pre(d)` が初期化の未知数になるので 1 つずつ要る数に入れる（MLS §8.6）。`fixed = true` なら
//!   `pre(d) = start` が 1 つ与えられる。初期方程式（`initial equation d = …`）や初期アルゴリズムの代入も
//!   フロントエンドが `initial_equations` に入れる。
//!   ただし離散変数が when の外の式で決まる（例 `y = x > 0`）か、when の中で決まるかは [`InitStructure`] では
//!   分からない。前者は MLS §8.6 の暗黙の規則で初期条件が要らない（`pre(d)` は `d` から決まる）。数が揺れるので:
//!   - 離散変数があるとき、またはインデックスの低減で状態が減るときの過剰は確定させない（`GM-E8001` ではなく `GM-W8004`）。
//!   - 不足の数が `fixed = false` の離散変数の数以下なら、離散変数の `pre` の初期値に start を使えば足りる
//!     見込みなので、`GM-W8001` を「足りない見込み」の柔らかい警告にし、`GM-W8002` は出さない
//!     （状態の初期条件が足りないとは確定しないため）。不足がそれより多ければ、今までどおり。
//! - `fixed = true` の代数変数は `v = start` が 1 つ与えられる（要る数は増えない）。
//! - `fixed = false` のパラメータでも、束縛（`parameter Real p(fixed = false) = …`。初期アルゴリズムで
//!   決めるものも Rumoca は束縛にする）があれば、その束縛が自分を決めるので、要る数にも与えられた数にも入れない。
//!
//! Rumoca に依存しない。wasm32 でも動く。

use globe_mode_core::{Diagnostic, Stage, TextRange, codes};
use globe_mode_ir::{InitStructure, InitVariable};
use serde::Serialize;

use crate::NOTE_LIST_LIMIT;

/// S8 の結果。添字は [`InitStructure`] の `states` / `parameters` のもの（昇順）。
///
/// 位置を持たないので、[`InitStructure::without_ranges`] を渡しても同じ値になる。
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize)]
pub struct InitReport {
    /// 要る初期条件の数（動的な状態（S7。無ければ状態）+ 離散変数 + fixed = false で束縛の無いパラメータ）
    pub required: usize,
    /// 与えられた初期条件の数（初期方程式 + fixed = true の状態・離散変数・代数変数）
    pub provided: usize,
    /// `fixed = true` の状態（`states` の添字）
    pub fixed_states: Vec<usize>,
    /// `fixed = false` で start の無い状態（`states` の添字）
    pub states_without_start: Vec<usize>,
    /// `fixed = false` で束縛の無いパラメータ（`parameters` の添字）。初期化の未知数になる
    pub free_parameters: Vec<usize>,
    /// 束縛も start も無い `fixed = true` のパラメータ（`parameters` の添字）
    pub parameters_without_value: Vec<usize>,
}

impl InitReport {
    /// 初期条件の数が合っている。
    pub fn is_balanced(&self) -> bool {
        self.required == self.provided
    }

    /// 診断（S8）。
    ///
    /// - 与えられた数が多ければ 1 件。離散変数が無く、動的な状態 = 状態の数なら `GM-E8001`（Error）、そうでなければ `GM-W8004`（Warning。
    ///   数が確定しない）。位置は最初の初期方程式（無ければ最初の `fixed = true` の状態・離散変数・代数変数の
    ///   宣言）。補足に初期方程式と `fixed = true` の変数の一覧。
    /// - 少なければ `GM-W8001`（Warning）を 1 件（位置なし）。補足に初期条件の無い状態と、
    ///   初期化で決めるパラメータの一覧。さらに start の無い状態ごとに `GM-W8002`（宣言の位置）。
    ///   不足の数が `fixed = false` の離散変数の数以下なら、`GM-W8001` は「足りない見込み」とし、
    ///   `GM-W8002` は出さない（モジュールの doc の約束）。
    /// - 値の無いパラメータごとに `GM-W8003`（宣言の位置）。
    /// - どれも無ければ空。
    ///
    /// `init` は、この結果を作ったときと同じ（位置のある）`InitStructure` を渡す。
    /// 添字が範囲外なら、名前の代わりに `#添字` と書く。
    pub fn diagnostics(&self, init: &InitStructure) -> Vec<Diagnostic> {
        let state = |i: usize| name(&init.states, i);
        let parameter = |i: usize| name(&init.parameters, i);
        let equation = |i: usize| {
            init.initial_equations
                .get(i)
                .map(|e| e.label.as_str())
                .filter(|s| !s.is_empty())
                .map_or_else(|| format!("#{i}"), str::to_string)
        };
        let n_discretes = init.discretes.len();
        let fixed_discretes = fixed(&init.discretes);
        let fixed_algebraics = fixed(&init.algebraics);
        // 要る数のうち状態の分。S7 の動的な状態の数を使ったときは、状態の数と違う
        let dynamic = self
            .required
            .saturating_sub(self.free_parameters.len() + n_discretes);
        let mut required_parts = vec![if dynamic == init.states.len() {
            format!("状態 {dynamic}")
        } else {
            format!("動的な状態 {dynamic}（状態 {} のうち）", init.states.len())
        }];
        if n_discretes > 0 {
            required_parts.push(format!("離散変数 {n_discretes}"));
        }
        required_parts.push(format!(
            "初期化で決めるパラメータ {}",
            self.free_parameters.len()
        ));
        let mut provided_parts = vec![
            format!("初期方程式 {}", init.initial_equations.len()),
            format!("fixed = true の状態 {}", self.fixed_states.len()),
        ];
        if n_discretes > 0 {
            provided_parts.push(format!("fixed = true の離散変数 {}", fixed_discretes.len()));
        }
        if !fixed_algebraics.is_empty() {
            provided_parts.push(format!(
                "fixed = true の代数変数 {}",
                fixed_algebraics.len()
            ));
        }
        let counts = format!(
            "要る数 {}（{}）、与えられた数 {}（{}）",
            self.required,
            required_parts.join(" + "),
            self.provided,
            provided_parts.join(" + ")
        );
        let index_note = (dynamic != init.states.len()).then(|| {
            format!(
                "インデックスの低減（S7）で、状態 {} 個のうち {} 個は微分した拘束で決まる見込み。初期条件が要るのは動的な状態 {dynamic} 個",
                init.states.len(),
                init.states.len().saturating_sub(dynamic)
            )
        });
        let discretes_note = (n_discretes > 0).then(|| {
            let all: Vec<usize> = (0..n_discretes).collect();
            format!(
                "離散変数（{n_discretes} 個、うち fixed = true {} 個。pre(d) が初期化の未知数になる）: {}",
                fixed_discretes.len(),
                list(&all, |i| name(&init.discretes, i))
            )
        });

        let mut out = Vec::new();
        if self.provided > self.required {
            let extra = self.provided - self.required;
            let equations: Vec<usize> = (0..init.initial_equations.len()).collect();
            // 確定できない理由: 離散変数（MLS 8.6 の暗黙の初期方程式で数が揺れる）、
            // インデックスの低減（余った fixed が拘束と矛盾しないかは数値で決まる。MSL の GenerationOfFMUs など）
            let mut uncertain = Vec::new();
            if n_discretes > 0 {
                uncertain.push("離散変数を含む");
            }
            if dynamic != init.states.len() {
                uncertain.push("インデックスの低減で状態が減る");
            }
            let d = if !uncertain.is_empty() {
                Diagnostic::warning(
                    codes::INIT_OVERDETERMINED_UNCERTAIN,
                    Stage::S8,
                    format!(
                        "初期条件が {extra} 個多い見込み（{}ため確定できない）: {counts}",
                        uncertain.join("・")
                    ),
                )
            } else {
                Diagnostic::error(
                    codes::INIT_OVERDETERMINED,
                    Stage::S8,
                    format!("初期条件が {extra} 個多い: {counts}"),
                )
            };
            // 位置: 最初の初期方程式。無ければ最初の fixed = true の変数（状態・離散・代数の順）の宣言
            let first_fixed = self
                .fixed_states
                .first()
                .and_then(|&i| init.states.get(i))
                .or_else(|| fixed_discretes.first().and_then(|&i| init.discretes.get(i)))
                .or_else(|| {
                    fixed_algebraics
                        .first()
                        .and_then(|&i| init.algebraics.get(i))
                });
            let (file, range) = match init.initial_equations.first() {
                Some(e) => (e.file.as_deref(), e.range),
                None => first_fixed.map_or((None, None), |v| (v.file.as_deref(), v.range)),
            };
            let mut d = locate(d, file, range);
            if let Some(n) = &index_note {
                d = d.note(n.clone());
            }
            d = d
                .note(format!(
                    "初期方程式（{} 本）: {}",
                    equations.len(),
                    list(&equations, equation)
                ))
                .note(format!(
                    "fixed = true の状態（{} 個）: {}",
                    self.fixed_states.len(),
                    list(&self.fixed_states, state)
                ));
            if !fixed_algebraics.is_empty() {
                d = d.note(format!(
                    "fixed = true の代数変数（{} 個。v = start が初期条件になる）: {}",
                    fixed_algebraics.len(),
                    list(&fixed_algebraics, |i| name(&init.algebraics, i))
                ));
            }
            if let Some(n) = &discretes_note {
                d = d
                    .note(n.clone())
                    .note("離散変数は when の外の pre(d) = d の暗黙の式などで数が揺れるので、過剰とは確定しない");
            }
            out.push(d.note(if index_note.is_some() {
                "fixed = true と初期方程式を、動的な状態の数（と初期化で決めるパラメータの数）までに減らす"
            } else {
                "fixed = true の変数に初期方程式も書いていないか確かめる"
            }));
        } else if self.provided < self.required {
            let missing = self.required - self.provided;
            let unfixed: Vec<usize> = (0..init.states.len())
                .filter(|i| self.fixed_states.binary_search(i).is_err())
                .collect();
            // 不足が fixed = false の離散変数の分に収まるなら、確定させない（柔らかい警告）
            let open_discretes = n_discretes - fixed_discretes.len();
            let uncertain = missing <= open_discretes;
            let message = if uncertain {
                format!(
                    "初期条件が {missing} 個足りない見込み（離散変数の pre の初期値に start を使えば足りる。MLS 8.6）: {counts}"
                )
            } else {
                format!(
                    "初期条件が {missing} 個足りない（不足分は start 値を推定値として使う見込み）: {counts}"
                )
            };
            let mut d =
                Diagnostic::warning(codes::INIT_UNDERDETERMINED, Stage::S8, message).note(format!(
                    "fixed = false の状態（{} 個）: {}",
                    unfixed.len(),
                    list(&unfixed, state)
                ));
            if let Some(n) = &index_note {
                d = d.note(n.clone());
            }
            if let Some(n) = &discretes_note {
                d = d.note(n.clone());
            }
            if !self.free_parameters.is_empty() {
                d = d.note(format!(
                    "初期化で決めるパラメータ（fixed = false、{} 個）: {}",
                    self.free_parameters.len(),
                    list(&self.free_parameters, parameter)
                ));
            }
            if uncertain {
                out.push(d.note(format!(
                    "不足の {missing} 個は fixed = false の離散変数（{open_discretes} 個）の分に収まる。when の外の式で決まる離散変数には初期条件が要らないので、確定しない"
                )));
            } else {
                out.push(d.note("状態に fixed = true を付けるか、initial equation を足す"));
                // 状態の初期条件が足りないと確定するときだけ、start の無い状態を示す
                for &i in &self.states_without_start {
                    let s = init.states.get(i);
                    out.push(
                        locate(
                            Diagnostic::warning(
                                codes::STATE_WITHOUT_START,
                                Stage::S8,
                                format!(
                                    "状態 `{}` に start 値が無い（初期条件が足りないので、既定の値から推定する見込み）",
                                    state(i)
                                ),
                            ),
                            s.and_then(|v| v.file.as_deref()),
                            s.and_then(|v| v.range),
                        )
                        .note("start = … を書くか、fixed = true にして初期値を決める"),
                    );
                }
            }
        }

        self.push_parameters_without_value(out, init)
    }

    /// 値の無いパラメータごとの `GM-W8003` を `out` に足して返す。
    fn push_parameters_without_value(
        &self,
        mut out: Vec<Diagnostic>,
        init: &InitStructure,
    ) -> Vec<Diagnostic> {
        for &i in &self.parameters_without_value {
            let p = init.parameters.get(i);
            out.push(
                locate(
                    Diagnostic::warning(
                        codes::PARAMETER_WITHOUT_VALUE,
                        Stage::S8,
                        format!(
                            "パラメータ `{}` に値が無い（束縛も start も無い）",
                            name(&init.parameters, i)
                        ),
                    ),
                    p.and_then(|v| v.file.as_deref()),
                    p.and_then(|v| v.range),
                )
                .note("宣言に = 値 を書くか、fixed = false にして initial equation で決める"),
            );
        }
        out
    }
}

/// 初期化系を調べる（S7 の結果を使わない。要る数には状態の数を使う）。
/// `analyze_init_with_index(init, None)` と同じ。
pub fn analyze_init(init: &InitStructure) -> InitReport {
    analyze_init_with_index(init, None)
}

/// 初期化系を調べる。`dynamic_states` には S7 の `IndexReport::dynamic_states` を渡す
/// （S7 が打ち切ったときや、S7 を通っていないときは None）。
///
/// - `Some(n)` なら、要る数の状態の分は `n`（インデックスの低減で、残りの状態は微分した拘束で
///   決まる見込みなので、初期条件が要らない）。
/// - `None` なら、要る数の状態の分は状態の数。
///
/// どちらも、離散変数の数と初期化で決めるパラメータの数を足す。
/// 位置は見ないので、結果は `init.without_ranges()` を渡しても同じ。
pub fn analyze_init_with_index(init: &InitStructure, dynamic_states: Option<usize>) -> InitReport {
    let fixed_states = fixed(&init.states);
    let states_without_start = indices(&init.states, |s| !s.fixed && !s.has_start);
    let free_parameters = indices(&init.parameters, |p| !p.fixed && !p.has_binding);
    let parameters_without_value = indices(&init.parameters, |p| {
        p.fixed && !p.has_binding && !p.has_start
    });
    let required =
        dynamic_states.unwrap_or(init.states.len()) + init.discretes.len() + free_parameters.len();
    let provided = init.initial_equations.len()
        + fixed_states.len()
        + fixed(&init.discretes).len()
        + fixed(&init.algebraics).len();
    InitReport {
        required,
        provided,
        fixed_states,
        states_without_start,
        free_parameters,
        parameters_without_value,
    }
}

/// 条件に合う変数の添字（昇順）。
fn indices(vars: &[InitVariable], keep: fn(&InitVariable) -> bool) -> Vec<usize> {
    vars.iter()
        .enumerate()
        .filter(|(_, v)| keep(v))
        .map(|(i, _)| i)
        .collect()
}

/// `fixed = true` の変数の添字（昇順）。
fn fixed(vars: &[InitVariable]) -> Vec<usize> {
    indices(vars, |v| v.fixed)
}

/// 変数の表示名。添字が範囲外か名前が空なら `#添字`。
fn name(vars: &[InitVariable], i: usize) -> String {
    vars.get(i)
        .map(|v| v.name.as_str())
        .filter(|s| !s.is_empty())
        .map_or_else(|| format!("#{i}"), str::to_string)
}

/// 位置を付ける。ファイルが分からなくても範囲だけは残す。
fn locate(mut d: Diagnostic, file: Option<&str>, range: Option<TextRange>) -> Diagnostic {
    match file {
        Some(f) => d.at(f, range),
        None => {
            d.range = range;
            d
        }
    }
}

/// 名前を `` `a`, `b` `` の形に並べる。[`NOTE_LIST_LIMIT`] を超えた分は「ほか k 件」にまとめる。
fn list(items: &[usize], name: impl Fn(usize) -> String) -> String {
    if items.is_empty() {
        return "なし".to_string();
    }
    let mut s = items
        .iter()
        .take(NOTE_LIST_LIMIT)
        .map(|&i| format!("`{}`", name(i)))
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
    use globe_mode_core::Severity;
    use globe_mode_ir::{EquationOrigin, EquationRow};

    const FILE: &str = "M.mo";

    /// 状態（fixed, has_start）。位置は添字から作る
    fn state(name: &str, fixed: bool, has_start: bool, at: u32) -> InitVariable {
        InitVariable {
            name: name.into(),
            fixed,
            has_start,
            has_binding: false,
            file: Some(FILE.into()),
            range: Some(TextRange::new(at, at + 6)),
        }
    }

    /// パラメータ（fixed, has_start, has_binding）
    fn param(name: &str, fixed: bool, has_start: bool, has_binding: bool, at: u32) -> InitVariable {
        InitVariable {
            name: name.into(),
            fixed,
            has_start,
            has_binding,
            file: Some(FILE.into()),
            range: Some(TextRange::new(at, at + 16)),
        }
    }

    fn equation(label: &str, at: u32) -> EquationRow {
        EquationRow {
            label: label.into(),
            origin: EquationOrigin::Source,
            file: Some(FILE.into()),
            range: Some(TextRange::new(at, at + 5)),
        }
    }

    fn codes_of(ds: &[Diagnostic]) -> Vec<&'static str> {
        ds.iter().map(|d| d.code).collect()
    }

    #[test]
    fn balanced_with_fixed_state_and_initial_equation() {
        // x は fixed = true、y は initial equation で決める
        let init = InitStructure {
            states: vec![state("x", true, true, 10), state("y", false, false, 30)],
            initial_equations: vec![equation("y = 2", 80)],
            ..Default::default()
        };
        let r = analyze_init(&init);
        assert_eq!((r.required, r.provided), (2, 2));
        assert!(r.is_balanced());
        assert_eq!(r.fixed_states, [0]);
        // start が無くても、数が合えば W8002 は出さない
        assert_eq!(r.states_without_start, [1]);
        assert!(r.diagnostics(&init).is_empty());
    }

    #[test]
    fn empty_structure_is_balanced() {
        let init = InitStructure::default();
        let r = analyze_init(&init);
        assert_eq!(r, InitReport::default());
        assert!(r.is_balanced());
        assert!(r.diagnostics(&init).is_empty());
    }

    #[test]
    fn fixed_state_with_initial_equation_is_overdetermined() {
        let init = InitStructure {
            states: vec![state("x", true, true, 10)],
            initial_equations: vec![equation("x = 2", 80)],
            ..Default::default()
        };
        let r = analyze_init(&init);
        assert_eq!((r.required, r.provided), (1, 2));
        let ds = r.diagnostics(&init);
        assert_eq!(codes_of(&ds), [codes::INIT_OVERDETERMINED]);
        let d = &ds[0];
        assert_eq!(d.severity, Severity::Error);
        assert_eq!(d.stage, Stage::S8);
        assert!(d.message.contains("1 個多い"), "{}", d.message);
        assert!(d.message.contains("要る数 1"), "{}", d.message);
        assert!(d.message.contains("与えられた数 2"), "{}", d.message);
        // 位置は最初の初期方程式
        assert_eq!(
            (d.file.as_deref(), d.range),
            (Some(FILE), init.initial_equations[0].range)
        );
        assert!(
            d.notes.iter().any(|n| n.contains("`x = 2`")),
            "{:?}",
            d.notes
        );
        assert!(
            d.notes
                .iter()
                .any(|n| n.starts_with("fixed = true の状態（1 個）") && n.contains("`x`")),
            "{:?}",
            d.notes
        );
    }

    #[test]
    fn missing_initial_conditions_warn_and_point_at_states_without_start() {
        // x は start あり、y は start なし。どちらも fixed でなく、初期方程式も無い
        let init = InitStructure {
            states: vec![state("x", false, true, 10), state("y", false, false, 30)],
            ..Default::default()
        };
        let r = analyze_init(&init);
        assert_eq!((r.required, r.provided), (2, 0));
        assert_eq!(r.states_without_start, [1]);
        let ds = r.diagnostics(&init);
        assert_eq!(
            codes_of(&ds),
            [codes::INIT_UNDERDETERMINED, codes::STATE_WITHOUT_START]
        );
        assert!(ds.iter().all(|d| d.severity == Severity::Warning));
        assert!(ds.iter().all(|d| d.stage == Stage::S8));
        assert!(ds[0].message.contains("2 個足りない"), "{}", ds[0].message);
        assert!(ds[0].message.contains("start 値を推定値として使う見込み"));
        assert_eq!((ds[0].file.as_deref(), ds[0].range), (None, None));
        assert!(
            ds[0].notes.iter().any(|n| n.contains("`x`, `y`")),
            "{:?}",
            ds[0].notes
        );
        // W8002 は y の宣言を指す
        assert!(ds[1].message.contains("`y`"));
        assert_eq!(
            (ds[1].file.as_deref(), ds[1].range),
            (Some(FILE), init.states[1].range)
        );
        assert!(
            r.diagnostics(&init)
                .iter()
                .all(|d| d.code != codes::INIT_OVERDETERMINED)
        );
    }

    #[test]
    fn parameter_without_value_warns_even_when_balanced() {
        let init = InitStructure {
            states: vec![state("x", true, true, 10)],
            parameters: vec![
                param("a", true, false, false, 100), // 値なし → W8003
                param("b", true, true, false, 120),  // start がある
                param("c", true, true, true, 140),   // 束縛がある
            ],
            ..Default::default()
        };
        let r = analyze_init(&init);
        assert!(r.is_balanced());
        assert_eq!(r.parameters_without_value, [0]);
        assert!(r.free_parameters.is_empty());
        let ds = r.diagnostics(&init);
        assert_eq!(codes_of(&ds), [codes::PARAMETER_WITHOUT_VALUE]);
        assert_eq!(ds[0].severity, Severity::Warning);
        assert!(ds[0].message.contains("`a`"));
        assert_eq!(
            (ds[0].file.as_deref(), ds[0].range),
            (Some(FILE), init.parameters[0].range)
        );
    }

    #[test]
    fn free_parameter_is_determined_by_initial_equation() {
        // parameter Real p(fixed = false); initial equation der(x) = 0;（x は fixed = true）
        let init = InitStructure {
            states: vec![state("x", true, true, 10)],
            parameters: vec![param("p", false, false, false, 100)],
            initial_equations: vec![equation("der(x) = 0", 80)],
            ..Default::default()
        };
        let r = analyze_init(&init);
        assert_eq!(r.free_parameters, [0]);
        assert_eq!((r.required, r.provided), (2, 2));
        // fixed = false のパラメータは、start が無くても値なしとは言わない
        assert!(r.parameters_without_value.is_empty());
        assert!(r.diagnostics(&init).is_empty());
    }

    #[test]
    fn free_parameter_without_initial_equation_is_underdetermined() {
        let init = InitStructure {
            states: vec![state("x", true, true, 10)],
            parameters: vec![param("p", false, true, false, 100)],
            ..Default::default()
        };
        let r = analyze_init(&init);
        assert_eq!((r.required, r.provided), (2, 1));
        let ds = r.diagnostics(&init);
        // 状態はすべて start があるので W8002 は無い。補足に p を挙げる
        assert_eq!(codes_of(&ds), [codes::INIT_UNDERDETERMINED]);
        assert!(
            ds[0].notes.iter().any(|n| n.contains("`p`")),
            "{:?}",
            ds[0].notes
        );
    }

    #[test]
    fn free_parameter_with_binding_is_not_counted() {
        // parameter Real p(fixed = false) = 2 * q;（束縛が p を決める。初期アルゴリズムで決めるものも同じ）
        let init = InitStructure {
            states: vec![state("x", true, true, 10)],
            parameters: vec![param("p", false, true, true, 100)],
            ..Default::default()
        };
        let r = analyze_init(&init);
        assert!(r.free_parameters.is_empty());
        assert!(r.is_balanced());
        assert!(r.diagnostics(&init).is_empty());
    }

    #[test]
    fn dynamic_states_from_index_reduction_set_the_required_count() {
        // 振り子（状態 x, y, vx, vy。動的な状態 2）に fixed = true が x と vx の 2 つ → 合う
        let init = InitStructure {
            states: vec![
                state("x", true, true, 10),
                state("y", false, false, 20),
                state("vx", true, true, 30),
                state("vy", false, false, 40),
            ],
            ..Default::default()
        };
        let r = analyze_init_with_index(&init, Some(2));
        assert_eq!((r.required, r.provided), (2, 2));
        assert!(r.diagnostics(&init).is_empty());
        // S7 の結果が無ければ、状態の数で数える（2 個足りない）
        let plain = analyze_init(&init);
        assert_eq!(plain, analyze_init_with_index(&init, None));
        assert_eq!((plain.required, plain.provided), (4, 2));
        assert_eq!(
            codes_of(&plain.diagnostics(&init)),
            [
                codes::INIT_UNDERDETERMINED,
                codes::STATE_WITHOUT_START,
                codes::STATE_WITHOUT_START
            ]
        );
    }

    #[test]
    fn fixed_state_without_dynamic_freedom_is_overdetermined_but_uncertain() {
        // 状態 1 つ（fixed = true）だが、インデックスの低減で動的な状態は 0 → 1 個多い。
        // 余った fixed が拘束と矛盾しないかは数値で決まるので、Error ではなく GM-W8004
        let init = InitStructure {
            states: vec![state("vC", true, true, 10)],
            ..Default::default()
        };
        let r = analyze_init_with_index(&init, Some(0));
        assert_eq!((r.required, r.provided), (0, 1));
        let ds = r.diagnostics(&init);
        assert_eq!(codes_of(&ds), [codes::INIT_OVERDETERMINED_UNCERTAIN]);
        assert_eq!(ds[0].severity, Severity::Warning);
        assert!(
            ds[0].message.contains("動的な状態 0（状態 1 のうち）"),
            "{}",
            ds[0].message
        );
        assert!(
            ds[0].notes.iter().any(|n| n.contains("インデックスの低減")),
            "{:?}",
            ds[0].notes
        );
        // 初期方程式が無いので、位置は fixed = true の状態の宣言
        assert_eq!(ds[0].range, init.states[0].range);
    }

    #[test]
    fn dynamic_states_with_free_parameter() {
        // 動的な状態 1 + fixed = false のパラメータ 1 = 要る数 2。fixed 1 + 初期方程式 1 で合う
        let init = InitStructure {
            states: vec![state("v1", true, true, 10), state("v2", false, false, 20)],
            parameters: vec![param("p", false, false, false, 100)],
            initial_equations: vec![equation("der(v1) = 0", 80)],
            ..Default::default()
        };
        let r = analyze_init_with_index(&init, Some(1));
        assert_eq!((r.required, r.provided), (2, 2));
        assert!(r.diagnostics(&init).is_empty());
        // 足りないときの補足にも、動的な状態の数を書く
        let short = InitStructure {
            initial_equations: vec![],
            ..init
        };
        let r = analyze_init_with_index(&short, Some(1));
        let ds = r.diagnostics(&short);
        assert_eq!(
            codes_of(&ds),
            [codes::INIT_UNDERDETERMINED, codes::STATE_WITHOUT_START]
        );
        assert!(ds[0].message.contains("動的な状態 1（状態 2 のうち）"));
        assert!(ds[0].notes.iter().any(|n| n.contains("インデックスの低減")));
    }

    #[test]
    fn discretes_need_an_initial_condition_each() {
        // 離散変数 d（fixed なし）と k（fixed = true）。pre(d) と pre(k) が初期化の未知数になる
        let init = InitStructure {
            states: vec![state("x", true, true, 10)],
            discretes: vec![state("d", false, false, 50), state("k", true, true, 60)],
            ..Default::default()
        };
        let r = analyze_init(&init);
        assert_eq!((r.required, r.provided), (3, 2));
        let ds = r.diagnostics(&init);
        // 状態はすべて fixed なので W8002 は無い
        assert_eq!(codes_of(&ds), [codes::INIT_UNDERDETERMINED]);
        assert!(ds[0].message.contains("離散変数 2"), "{}", ds[0].message);
        assert!(
            ds[0].message.contains("fixed = true の離散変数 1"),
            "{}",
            ds[0].message
        );
        assert!(
            ds[0].notes.iter().any(|n| n.contains("`d`, `k`")),
            "{:?}",
            ds[0].notes
        );
        // d を初期方程式で決めれば合う
        let with_equation = InitStructure {
            initial_equations: vec![equation("d = 0", 80)],
            ..init
        };
        let r = analyze_init(&with_equation);
        assert_eq!((r.required, r.provided), (3, 3));
        assert!(r.diagnostics(&with_equation).is_empty());
    }

    #[test]
    fn shortfall_within_open_discretes_is_a_soft_warning() {
        // PID_Controller 相当: 状態 10（動的な状態 6）、初期方程式 3、fixed の状態 2、fixed の代数変数 1、
        // fixed でない離散変数 3。連続の分は合っていて、不足の 3 個は離散変数の分に収まる
        let states: Vec<InitVariable> = (0..10)
            .map(|i| state(&format!("s{i}"), i < 2, false, 10 * i as u32))
            .collect();
        let init = InitStructure {
            states,
            discretes: (0..3)
                .map(|i| state(&format!("b{i}"), false, true, 200 + i))
                .collect(),
            algebraics: vec![state("y", true, true, 300)],
            initial_equations: (0..3)
                .map(|i| equation(&format!("e{i}"), 400 + i))
                .collect(),
            ..Default::default()
        };
        let r = analyze_init_with_index(&init, Some(6));
        assert_eq!((r.required, r.provided), (9, 6));
        assert_eq!(r.states_without_start.len(), 8);
        let ds = r.diagnostics(&init);
        // 柔らかい W8001 だけ。W8002 は出さない
        assert_eq!(codes_of(&ds), [codes::INIT_UNDERDETERMINED]);
        assert_eq!(ds[0].severity, Severity::Warning);
        assert!(
            ds[0].message.contains(
                "3 個足りない見込み（離散変数の pre の初期値に start を使えば足りる。MLS 8.6）"
            ),
            "{}",
            ds[0].message
        );
        assert!(
            ds[0].notes.iter().any(|n| n.contains("分に収まる")),
            "{:?}",
            ds[0].notes
        );

        // BooleanSignals 相当: 状態 0、離散変数 4（fixed なし）→ 柔らかい W8001 だけ
        let booleans = InitStructure {
            discretes: (0..4)
                .map(|i| state(&format!("b{i}"), false, false, 10 * i))
                .collect(),
            ..Default::default()
        };
        let r = analyze_init(&booleans);
        assert_eq!((r.required, r.provided), (4, 0));
        let ds = r.diagnostics(&booleans);
        assert_eq!(codes_of(&ds), [codes::INIT_UNDERDETERMINED]);
        assert!(
            ds[0].message.contains("足りない見込み"),
            "{}",
            ds[0].message
        );
    }

    #[test]
    fn shortfall_beyond_open_discretes_is_certain() {
        // 不足 2 に対して fixed でない離散変数は 1 → 状態の初期条件が足りないと確定する
        let init = InitStructure {
            states: vec![state("x", false, false, 10), state("y", false, true, 20)],
            discretes: vec![state("b", false, false, 30)],
            initial_equations: vec![equation("b = false", 80)],
            ..Default::default()
        };
        let r = analyze_init(&init);
        assert_eq!((r.required, r.provided), (3, 1));
        let ds = r.diagnostics(&init);
        assert_eq!(
            codes_of(&ds),
            [codes::INIT_UNDERDETERMINED, codes::STATE_WITHOUT_START]
        );
        assert!(
            ds[0].message.contains("2 個足りない（"),
            "{}",
            ds[0].message
        );
        assert!(ds[1].message.contains("`x`"));
        // fixed = true の離散変数は「収まる」の数に入れない
        let fixed_discrete = InitStructure {
            states: vec![state("x", false, false, 10)],
            discretes: vec![state("b", true, true, 30)],
            ..Default::default()
        };
        let ds = analyze_init(&fixed_discrete).diagnostics(&fixed_discrete);
        assert_eq!(
            codes_of(&ds),
            [codes::INIT_UNDERDETERMINED, codes::STATE_WITHOUT_START]
        );
    }

    #[test]
    fn excess_with_discretes_is_only_a_warning() {
        // fixed = true の離散変数 k に、初期方程式 k = 0 も書いた。離散変数があるので確定させない
        let init = InitStructure {
            states: vec![state("x", true, true, 10)],
            discretes: vec![state("k", true, true, 60)],
            initial_equations: vec![equation("k = 0", 80)],
            ..Default::default()
        };
        let r = analyze_init(&init);
        assert_eq!((r.required, r.provided), (2, 3));
        let ds = r.diagnostics(&init);
        assert_eq!(codes_of(&ds), [codes::INIT_OVERDETERMINED_UNCERTAIN]);
        let d = &ds[0];
        assert_eq!(d.severity, Severity::Warning);
        assert_eq!(d.stage, Stage::S8);
        assert!(
            d.message
                .contains("1 個多い見込み（離散変数を含むため確定できない）"),
            "{}",
            d.message
        );
        assert_eq!(d.range, init.initial_equations[0].range);
        assert!(
            d.notes.iter().any(|n| n.contains("pre(d) = d")),
            "{:?}",
            d.notes
        );
        // 初期方程式が無くても、位置は fixed = true の変数の宣言（状態が無ければ離散変数）
        let only_discretes = InitStructure {
            discretes: vec![state("k", true, true, 60), state("j", true, true, 70)],
            initial_equations: vec![],
            ..Default::default()
        };
        let mut r = analyze_init(&only_discretes);
        r.required = 1;
        let ds = r.diagnostics(&only_discretes);
        assert_eq!(codes_of(&ds), [codes::INIT_OVERDETERMINED_UNCERTAIN]);
        assert_eq!(ds[0].range, only_discretes.discretes[0].range);
    }

    #[test]
    fn fixed_algebraic_adds_an_initial_condition() {
        // 代数変数 y に fixed = true（v = start）。状態 x も fixed = true なら 1 個多い（離散変数なし → Error）
        let init = InitStructure {
            states: vec![state("x", true, true, 10)],
            algebraics: vec![state("y", true, true, 40), state("z", false, false, 45)],
            ..Default::default()
        };
        let r = analyze_init(&init);
        assert_eq!((r.required, r.provided), (1, 2));
        let ds = r.diagnostics(&init);
        assert_eq!(codes_of(&ds), [codes::INIT_OVERDETERMINED]);
        assert!(
            ds[0].message.contains("fixed = true の代数変数 1"),
            "{}",
            ds[0].message
        );
        assert!(
            ds[0]
                .notes
                .iter()
                .any(|n| n.starts_with("fixed = true の代数変数（1 個") && n.contains("`y`")),
            "{:?}",
            ds[0].notes
        );
        // x の fixed を外せば、y の fixed = true で合う（代数変数は要る数を増やさない）
        let balanced = InitStructure {
            states: vec![state("x", false, true, 10)],
            ..init
        };
        let r = analyze_init(&balanced);
        assert_eq!((r.required, r.provided), (1, 1));
        assert!(r.diagnostics(&balanced).is_empty());
    }

    #[test]
    fn report_ignores_ranges() {
        let init = InitStructure {
            states: vec![state("x", false, false, 10), state("y", true, true, 30)],
            parameters: vec![param("a", true, false, false, 100)],
            initial_equations: vec![equation("x = 1", 80), equation("y = 1", 90)],
            ..Default::default()
        };
        let stripped = init.without_ranges();
        let r = analyze_init(&init);
        assert_eq!(analyze_init(&stripped), r);
        let ds = r.diagnostics(&stripped);
        assert_eq!(
            codes_of(&ds),
            [codes::INIT_OVERDETERMINED, codes::PARAMETER_WITHOUT_VALUE]
        );
        assert!(ds.iter().all(|d| d.range.is_none()));
    }

    #[test]
    fn long_lists_are_cut() {
        let n = NOTE_LIST_LIMIT + 3;
        let init = InitStructure {
            states: (0..n)
                .map(|i| state(&format!("x{i}"), false, true, 10 * i as u32))
                .collect(),
            ..Default::default()
        };
        let r = analyze_init(&init);
        let ds = r.diagnostics(&init);
        assert_eq!(codes_of(&ds), [codes::INIT_UNDERDETERMINED]);
        let note = &ds[0].notes[0];
        assert!(note.ends_with(" ほか 3 件"), "{note}");
        assert_eq!(note.matches('`').count(), 2 * NOTE_LIST_LIMIT);
    }

    #[test]
    fn out_of_range_indices_are_named_by_number() {
        let init = InitStructure::default();
        let r = InitReport {
            required: 1,
            provided: 0,
            states_without_start: vec![4],
            parameters_without_value: vec![7],
            ..Default::default()
        };
        let ds = r.diagnostics(&init);
        assert_eq!(
            codes_of(&ds),
            [
                codes::INIT_UNDERDETERMINED,
                codes::STATE_WITHOUT_START,
                codes::PARAMETER_WITHOUT_VALUE
            ]
        );
        assert!(ds[1].message.contains("`#4`"));
        assert!(ds[2].message.contains("`#7`"));
        assert!(ds.iter().all(|d| d.file.is_none() && d.range.is_none()));
    }
}

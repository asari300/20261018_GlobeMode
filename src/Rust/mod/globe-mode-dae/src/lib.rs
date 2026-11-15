//! Track B の B4 の最初の一切れ（S290）: 平らにしたモデル（`globe-mode-flat`）から DAE の構造
//! （`globe-mode-ir` の [`DaeStructure`]）と釣り合いの数を作る。
//!
//! - 変数を、状態（`der(x)` が現れる連続変数）・代数・離散（`discrete`・Integer/Boolean/列挙・`when` で代入する Real）・
//!   既知（parameter・constant・トップレベルの input）に分ける。未知数の約束は Rumoca の構造解析と同じ
//!   （状態は `der(x)` のスカラー、代数と出力はスカラー）。
//! - incidence は、方程式の印字を `globe-mode-syntax` で読み直した木の `ComponentRef` から取る（`der(x)` は `der(x)` の未知数、
//!   状態そのものの参照は `state_refs`、`pre()` の中は無視）。
//! - 数え方は Rumoca の釣り合いと同じ分け方: 方程式 = 連続の行 + 離散の定義（`when` の中の代入・Integer/Boolean の定義）、
//!   未知数 = 状態 + 代数 + 離散。`DaeStructure` には連続の行と未知数だけを入れる。
//!
//! shortcut（広めの incidence。釣り合いの数は変わらず、構造の正則は甘く出る方に傾く）:
//! - `if`・`when`・`algorithm` は 1 本のまとまりとして、枝の中の未知数の和集合を行の incidence にする。
//! - 配列は、添字が定数ならその要素、添字なしで大きさが行の数と同じなら行 k に要素 k、それ以外は全要素。
//! - 非線形の判定はしない（`nonlinear` は空）。位置は付けない。
//!
//! 既存のフロントエンド（Rumoca）・LSP にはつながない（doc/adr/0039-native-cst-parser.md の付記）。

use globe_mode_flat::{Causality, FlatModel, Unsupported, Var, VarKind};
use globe_mode_ir::{
    BalanceCount, DaeStructure, EquationOrigin, EquationRow, Unknown, UnknownKind,
};
use globe_mode_syntax::ast::{self, Expr};
use globe_mode_syntax::{SyntaxKind as K, SyntaxNode, parse};
use std::collections::{BTreeSet, HashMap, HashSet};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    /// parameter・constant・トップレベルの input・String
    Known,
    State,
    Algebraic,
    Discrete,
}

#[derive(Clone, Debug)]
pub struct Dae {
    /// 連続の行と未知数（S6 の入力）
    pub structure: DaeStructure,
    /// 釣り合いの数（Rumoca の `BalanceCount` と同じ分け方。`outputs` は代数に含めて 0）
    pub counts: BalanceCount,
    /// 離散の方程式の数（`when` の中の代入と Integer/Boolean の定義）
    pub discrete_equations: usize,
    /// 変数ごとの役割（`FlatModel::vars` と同じ順）
    pub roles: Vec<Role>,
}

impl Dae {
    /// 方程式の総数 = 未知数の総数（S5）
    pub fn is_balanced(&self) -> bool {
        self.counts.is_balanced()
    }
    /// 連続の部分が構造的に正則（行と未知数が同数で、完全マッチングがある。S6 のまとめた形）
    pub fn is_regular(&self) -> bool {
        globe_mode_analysis::analyze(&self.structure).is_regular()
    }
}

#[derive(Clone, Copy, Debug)]
enum Sel {
    All,
    One(usize),
}

#[derive(Clone, Copy, Debug)]
struct Ref {
    var: usize,
    sel: Sel,
    der: bool,
}

fn bad<T>(kind: &'static str, detail: String) -> Result<T, Unsupported> {
    Err(Unsupported { kind, detail })
}

/// `text` を `model M <head> text; end M;` として読み、`head` の節の木を返す
fn parse_in_model(head: &str, text: &str) -> Result<SyntaxNode, Unsupported> {
    // 単純な方程式の印字は `;` が無く、`if`・`when` の印字は `;` で終わる
    let semi = if head == "equation" && !text.trim_end().ends_with(';') {
        ";"
    } else {
        ""
    };
    let src = format!("model M {head} {text}{semi} end M;");
    let p = parse(&src);
    if let Some(e) = p.errors.first() {
        return bad("dae-parse", format!("{}: {text}", e.message));
    }
    Ok(p.syntax())
}

struct Ctx<'a> {
    index: HashMap<&'a str, usize>,
    vars: &'a [Var],
}

impl Ctx<'_> {
    fn n(&self, v: usize) -> usize {
        self.vars[v].dims.iter().product::<usize>().max(1)
    }

    /// 方程式の左辺がレコードの部品（変数ではなく、`a.re`・`a.im` の持ち主）なら、欄のスカラーの数の和。
    /// 平らにした方程式は 1 本と数えるが、DAE では欄ごとの方程式になる（Rumoca の record equation。`y = Complex(a, b)` は 2 本）
    fn record_scalars(&self, eq: &SyntaxNode) -> Option<usize> {
        let Some(Expr::ComponentRef(c)) = eq.children().find_map(Expr::cast) else {
            return None;
        };
        let t: String = c.syntax().text().to_string().split_whitespace().collect();
        if self.index.contains_key(t.as_str()) {
            return None;
        }
        let pre = format!("{t}.");
        let n: usize = self
            .index
            .iter()
            .filter(|(k, _)| k.starts_with(&pre))
            .map(|(_, &v)| self.n(v))
            .sum();
        (n > 0).then_some(n)
    }

    /// `a.b[2].x[1]` を変数と選択に解く。変数でなければ None
    fn resolve(&self, text: &str) -> Option<(usize, Sel)> {
        let t: String = text.chars().filter(|c| !c.is_whitespace()).collect();
        if let Some(&v) = self.index.get(t.as_str()) {
            return Some((v, Sel::All));
        }
        let inner = t.strip_suffix(']')?;
        // 最後の添字の `[` を、対応で探す
        let mut depth = 0;
        let open = inner.char_indices().rev().find_map(|(i, c)| {
            match c {
                ']' => depth += 1,
                '[' if depth == 0 => return Some(i),
                '[' => depth -= 1,
                _ => {}
            }
            None
        })?;
        let v = *self.index.get(&t[..open])?;
        let dims = &self.vars[v].dims;
        let subs: Vec<Option<usize>> = inner[open + 1..]
            .split(',')
            .map(|s| s.parse::<usize>().ok().filter(|&i| i >= 1))
            .collect();
        if subs.len() != dims.len() || subs.iter().any(Option::is_none) {
            return Some((v, Sel::All));
        }
        let mut k = 0;
        for (s, d) in subs.iter().zip(dims) {
            let s = s.unwrap_or(1) - 1;
            if s >= *d {
                return Some((v, Sel::All));
            }
            k = k * d + s;
        }
        Some((v, Sel::One(k)))
    }

    /// 木の中の変数の参照。`pre()` の中と関数名は除く
    fn refs(&self, node: &SyntaxNode) -> Vec<Ref> {
        let mut out = Vec::new();
        for n in node.descendants() {
            let Some(c) = ast::ComponentRef::cast(n.clone()) else {
                continue;
            };
            if n.parent().is_some_and(|p| p.kind() == K::CallExpr) {
                continue; // 呼ぶ関数の名前
            }
            let mut der = false;
            let mut skip = false;
            for a in n.ancestors().skip(1) {
                if let Some(call) = ast::CallExpr::cast(a) {
                    match call.callee().as_deref() {
                        Some("der") => der = true,
                        Some("pre" | "previous") => skip = true,
                        _ => {}
                    }
                }
            }
            if skip {
                continue;
            }
            if let Some((var, sel)) = self.resolve(&c.syntax().text().to_string()) {
                out.push(Ref { var, sel, der });
            }
        }
        out
    }

    /// 代入の左辺（`x = …`・`x := …`）の変数
    fn lhs(&self, node: &SyntaxNode) -> Vec<(usize, Sel)> {
        let first = node.children().find_map(Expr::cast);
        let items = match first {
            Some(Expr::Paren(p)) => p.items(),
            Some(e) => vec![e],
            None => Vec::new(),
        };
        items
            .iter()
            .filter_map(|e| match e {
                Expr::ComponentRef(c) => self.resolve(&c.syntax().text().to_string()),
                _ => None,
            })
            .collect()
    }
}

fn kw_tokens(node: &SyntaxNode) -> impl Iterator<Item = K> {
    node.children_with_tokens()
        .filter_map(|e| e.into_token())
        .map(|t| t.kind())
}

/// `if`・`when` の最初の枝の入れ子の方程式（文）
fn first_branch(node: &SyntaxNode, kind: K) -> Vec<SyntaxNode> {
    let mut out = Vec::new();
    let mut seen_then = false;
    for e in node.children_with_tokens() {
        if let Some(t) = e.as_token() {
            if matches!(t.kind(), K::KwElseif | K::KwElsewhen | K::KwElse) && seen_then {
                break;
            }
            seen_then |= t.kind() == K::KwThen;
        } else if let Some(n) = e.into_node()
            && n.kind() == kind
            && seen_then
        {
            out.push(n);
        }
    }
    out
}

/// `Equation` の節（`if`・`when` は最初の枝）の左辺が離散の変数か（Rumoca と同じ。右辺が離散でも左辺が代数なら連続の行）
fn defines_discrete(cx: &Ctx, roles: &[Role], eq: &SyntaxNode) -> bool {
    if matches!(kw_tokens(eq).next(), Some(K::KwIf | K::KwWhen | K::KwFor)) {
        return first_branch(eq, K::Equation)
            .iter()
            .any(|e| defines_discrete(cx, roles, e));
    }
    let es: Vec<Expr> = eq.children().filter_map(Expr::cast).collect();
    es.iter().take(1).any(|e| match e {
        Expr::ComponentRef(c) => cx
            .resolve(&c.syntax().text().to_string())
            .is_some_and(|(v, _)| roles[v] == Role::Discrete),
        _ => false,
    })
}

/// 辺の 1 つが Clock 型の変数そのもの（`c = Clock(…)`・`y = c`）。`z = sample(u, c)` のように式の中で使うだけなら数える
fn is_clock_equation(cx: &Ctx, eq: &SyntaxNode) -> bool {
    eq.children()
        .filter_map(Expr::cast)
        .take(2)
        .any(|e| match e {
            Expr::ComponentRef(c) => cx
                .resolve(&c.syntax().text().to_string())
                .is_some_and(|(v, _)| cx.vars[v].ty == "Clock"),
            _ => false,
        })
}

struct Row {
    label: String,
    origin: EquationOrigin,
    count: usize,
    refs: Vec<Ref>,
    /// 左辺（束縛・algorithm の代入先）として必ず入れる未知数
    own: Vec<(usize, Sel)>,
}

fn label(text: &str) -> String {
    text.chars().take(60).collect()
}

/// 平らなモデルから DAE の構造を作る
pub fn build(m: &FlatModel) -> Result<Dae, Unsupported> {
    let cx = Ctx {
        index: m
            .vars
            .iter()
            .enumerate()
            .map(|(i, v)| (v.name.as_str(), i))
            .collect(),
        vars: &m.vars,
    };

    // 方程式の木（連続側）。`when` と algorithm は別に持つ
    let mut eqs: Vec<(SyntaxNode, usize, String)> = Vec::new();
    for e in &m.equations {
        if e.count > 0 {
            let node = first_equation(&parse_in_model("equation", &e.text)?)?;
            // Rumoca は Clock 型の変数とその式を数えない（`c = Clock(…)`・`y = c`）
            if is_clock_equation(&cx, &node) {
                continue;
            }
            let count = cx
                .record_scalars(&node)
                .filter(|_| e.count == 1)
                .unwrap_or(e.count);
            eqs.push((node, count, e.text.clone()));
        }
    }
    let mut whens: Vec<(SyntaxNode, usize)> = Vec::new();
    for e in &m.when_equations {
        whens.push((
            first_equation(&parse_in_model("equation", &e.text)?)?,
            e.count,
        ));
    }
    let mut algs: Vec<SyntaxNode> = Vec::new();
    for a in &m.algorithms {
        // 初期の algorithm は釣り合いに数えない（Rumoca と同じ）
        if a.trim_start().starts_with("initial") {
            continue;
        }
        let body = a.trim_start().strip_prefix("algorithm").unwrap_or(a);
        algs.push(parse_in_model("algorithm", body)?);
    }
    let mut binds: Vec<(usize, SyntaxNode)> = Vec::new();
    for (i, v) in m.vars.iter().enumerate() {
        if let Some(b) = &v.binding
            && matches!(v.kind, VarKind::Continuous | VarKind::Discrete)
            && !is_top_input(v)
            && v.ty != "Clock"
        {
            binds.push((
                i,
                first_equation(&parse_in_model("equation", &format!("0 = ({b})"))?)?,
            ));
        }
    }

    // 役割: `der` の引数は状態、`when`・algorithm の `when` で代入する Real は離散
    let mut has_der = HashSet::new();
    for node in eqs
        .iter()
        .map(|e| &e.0)
        .chain(whens.iter().map(|w| &w.0))
        .chain(algs.iter())
    {
        has_der.extend(cx.refs(node).iter().filter(|r| r.der).map(|r| r.var));
    }
    let mut when_assigned = HashSet::new();
    for (w, _) in &whens {
        for n in w.descendants().filter(|n| n.kind() == K::Equation) {
            when_assigned.extend(cx.lhs(&n).into_iter().map(|l| l.0));
        }
    }
    for a in &algs {
        for n in a.descendants().filter(|n| {
            n.kind() == K::Statement
                && n.ancestors()
                    .skip(1)
                    .any(|p| p.kind() == K::Statement && kw_tokens(&p).next() == Some(K::KwWhen))
        }) {
            when_assigned.extend(cx.lhs(&n).into_iter().map(|l| l.0));
        }
    }
    let roles: Vec<Role> = m
        .vars
        .iter()
        .enumerate()
        .map(|(i, v)| {
            if matches!(v.kind, VarKind::Constant | VarKind::Parameter)
                || v.ty == "String"
                || v.ty == "Clock"
                || is_top_input(v)
            {
                Role::Known
            } else if v.kind == VarKind::Discrete || when_assigned.contains(&i) {
                Role::Discrete
            } else if has_der.contains(&i) {
                Role::State
            } else {
                Role::Algebraic
            }
        })
        .collect();

    // 未知数（変数の順。状態は der、代数はそのまま）
    let mut unknowns = Vec::new();
    let mut off = vec![0usize; m.vars.len()];
    let (mut states, mut algebraics, mut discrete) = (0, 0, 0);
    for (i, v) in m.vars.iter().enumerate() {
        let n = cx.n(i);
        let name = |k: usize| {
            if v.dims.is_empty() {
                v.name.clone()
            } else {
                format!("{}[{}]", v.name, k + 1)
            }
        };
        match roles[i] {
            Role::State | Role::Algebraic => {
                off[i] = unknowns.len();
                let (state, kind) = if roles[i] == Role::State {
                    states += n;
                    (true, UnknownKind::Derivative)
                } else {
                    algebraics += n;
                    (
                        false,
                        if v.causality == Causality::Output && !v.name.contains('.') {
                            UnknownKind::Output
                        } else {
                            UnknownKind::Algebraic
                        },
                    )
                };
                for k in 0..n {
                    unknowns.push(Unknown {
                        name: if state {
                            format!("der({})", name(k))
                        } else {
                            name(k)
                        },
                        kind,
                        file: None,
                        range: None,
                    });
                }
            }
            Role::Discrete => discrete += n,
            Role::Known => {}
        }
    }

    // 行
    let mut rows: Vec<Row> = Vec::new();
    let mut discrete_equations = 0;
    for (node, count, text) in &eqs {
        if defines_discrete(&cx, &roles, node) {
            discrete_equations += count;
        } else {
            rows.push(Row {
                label: label(text),
                origin: EquationOrigin::Source,
                count: *count,
                refs: cx.refs(node),
                own: Vec::new(),
            });
        }
    }
    discrete_equations += whens.iter().map(|w| w.1).sum::<usize>();
    for (i, node) in &binds {
        let n = cx.n(*i);
        if roles[*i] == Role::Discrete {
            discrete_equations += n;
        } else if roles[*i] != Role::Known {
            rows.push(Row {
                label: label(&format!("{} = …", m.vars[*i].name)),
                origin: EquationOrigin::Binding,
                count: n,
                refs: cx.refs(node),
                own: vec![(*i, Sel::All)],
            });
        }
    }
    for a in &algs {
        // 代入先のスカラーごとに 1 本。行は algorithm 全体の参照の和集合（shortcut）
        let refs = cx.refs(a);
        let mut seen: BTreeSet<(usize, usize)> = BTreeSet::new();
        for s in a.descendants().filter(|n| n.kind() == K::Statement) {
            if !s.children_with_tokens().any(|e| e.kind() == K::Assign) {
                continue;
            }
            for (v, sel) in cx.lhs(&s) {
                match sel {
                    Sel::One(k) => seen.insert((v, k)),
                    Sel::All => (0..cx.n(v)).fold(false, |_, k| seen.insert((v, k))),
                };
            }
        }
        for (v, k) in seen {
            match roles[v] {
                Role::Discrete => discrete_equations += 1,
                Role::Known => {}
                _ => rows.push(Row {
                    label: label(&format!("algorithm {}", m.vars[v].name)),
                    origin: EquationOrigin::Source,
                    count: 1,
                    refs: refs.clone(),
                    own: vec![(v, Sel::One(k))],
                }),
            }
        }
    }

    let mut s = DaeStructure {
        unknowns,
        ..DaeStructure::default()
    };
    for r in &rows {
        for k in 0..r.count {
            let (mut inc, mut st) = (BTreeSet::new(), BTreeSet::new());
            let pick = |sel: Sel, v: usize| -> Vec<usize> {
                match sel {
                    Sel::One(j) => vec![j],
                    Sel::All if r.count > 1 && cx.n(v) == r.count => vec![k],
                    Sel::All => (0..cx.n(v)).collect(),
                }
            };
            let own = r.own.iter().map(|&(v, sel)| Ref {
                var: v,
                sel,
                der: false,
            });
            for x in r.refs.iter().copied().chain(own) {
                for j in pick(x.sel, x.var) {
                    let u = (off[x.var] + j) as u32;
                    match roles[x.var] {
                        Role::State if x.der => inc.insert(u),
                        Role::State => st.insert(u),
                        Role::Algebraic => inc.insert(u),
                        _ => false,
                    };
                }
            }
            s.equations.push(EquationRow {
                label: r.label.clone(),
                origin: r.origin,
                file: None,
                range: None,
            });
            s.incidence.push(inc.into_iter().collect());
            s.state_refs.push(st.into_iter().collect());
        }
    }
    let equations = s.equations.len() + discrete_equations;
    Ok(Dae {
        counts: BalanceCount {
            equations,
            unknowns: s.unknowns.len() + discrete,
            states,
            algebraics,
            outputs: 0,
            discrete,
        },
        structure: s,
        discrete_equations,
        roles,
    })
}

fn is_top_input(v: &Var) -> bool {
    v.causality == Causality::Input && !v.name.contains('.')
}

fn first_equation(root: &SyntaxNode) -> Result<SyntaxNode, Unsupported> {
    match root.descendants().find(|n| n.kind() == K::Equation) {
        Some(n) => Ok(n),
        None => bad("dae-parse", root.text().to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use globe_mode_flat::Project;

    fn dae(src: &str) -> Dae {
        let p = Project::from_sources([("t.mo", src)]);
        build(&p.flatten("M").unwrap()).unwrap()
    }

    #[test]
    fn one_state_is_balanced() {
        let d = dae("model M Real x(start=1); equation der(x) = -x; end M;");
        assert_eq!(
            (d.counts.states, d.counts.unknowns, d.counts.equations),
            (1, 1, 1)
        );
        assert!(d.is_balanced() && d.is_regular());
        assert_eq!(d.structure.incidence, [[0]]);
        assert_eq!(d.structure.state_refs, [[0]]);
    }

    #[test]
    fn algebraic_loop_is_regular() {
        let d = dae("model M Real x; Real y; equation x + y = 1; x - y = 0; end M;");
        assert!(d.is_balanced() && d.is_regular());
        assert_eq!(d.structure.incidence, [[0, 1], [0, 1]]);
    }

    #[test]
    fn discrete_variables_count_on_both_sides() {
        let d = dae("model M Real x(start=0); Integer n(start=0); Boolean b; \
             equation der(x) = 1; b = x > 1; when b then n = pre(n) + 1; end when; end M;");
        assert_eq!(d.counts.discrete, 2);
        assert_eq!(d.discrete_equations, 2);
        assert!(d.is_balanced() && d.is_regular());
        assert_eq!(d.structure.equations.len(), 1);
    }

    #[test]
    fn unbalanced_model_is_reported() {
        let d = dae("model M Real x; Real y; equation x = 1; end M;");
        assert!(!d.is_balanced() && !d.is_regular());
    }

    #[test]
    fn structurally_singular_but_balanced_counts() {
        let d = dae("model M Real x; Real y; equation x = 1; x = 2; end M;");
        assert!(d.is_balanced() && !d.is_regular());
    }
}

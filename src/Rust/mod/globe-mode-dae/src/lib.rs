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
//! - 非線形の判定はしない（`nonlinear` は空）。位置は平らなモデルの由来（`Origin`、S311）をそのまま付ける（`algorithm` の行は付けない）。
//!
//! 既存のフロントエンド（Rumoca）・LSP にはつながない（doc/adr/0039-native-cst-parser.md の付記）。

use globe_mode_core::TextRange;
use globe_mode_flat::{Causality, FlatModel, Origin, Unsupported, Var, VarKind};
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
    /// 部品の配列を通した欄（`a.pin.v`）の j 番目の要素の変数。配列の行 k は k == j のときだけ触れる
    Part(usize),
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
        self.vars[v].dims.iter().product::<usize>()
    }

    /// 方程式の左辺がレコードの部品（変数ではなく、`a.re`・`a.im` の持ち主）なら、その欄の変数（`BTreeMap` の順）。
    /// 平らにした方程式は 1 本と数えるが、DAE では欄ごとの方程式になる（Rumoca の record equation。`y = Complex(a, b)` は 2 本）
    /// 要素ごとに展開されたレコードの配列（`Complex aw[3]` の `aw = {…}`。flat の数は要素の数 3）は、全要素の欄を数える（S351）。
    /// 欄の変数の数は Rumoca の数え方と同じ（要素 × 欄のスカラー）
    fn record_leaves(&self, eq: &SyntaxNode, count: usize) -> Vec<usize> {
        let Some(Expr::ComponentRef(c)) = eq.children().find_map(Expr::cast) else {
            return Vec::new();
        };
        let text = c.syntax().text().to_string();
        let v = if count == 1 {
            self.leaves_of(&text)
        } else {
            Vec::new()
        };
        if v.is_empty() {
            self.array_leaves(&text)
        } else {
            v
        }
    }

    /// 要素ごとに展開されたレコードの配列 `t` の、欄の変数（`t[1].re`・`t[1].im`・`t[2].re` …。`BTreeMap` の順）
    fn array_leaves(&self, text: &str) -> Vec<usize> {
        let t: String = text.split_whitespace().collect();
        if t.ends_with(']') || self.index.contains_key(t.as_str()) {
            return Vec::new();
        }
        let pre = format!("{t}[");
        let mut v: Vec<usize> = self
            .index
            .iter()
            .filter(|(k, _)| k.strip_prefix(&pre).is_some_and(|r| r.contains("].")))
            .map(|(_, &v)| v)
            .collect();
        v.sort_unstable();
        v
    }

    /// 変数でないレコードの参照 `a.R` の、欄の変数（`a.R.T`・`a.R.w` …。`BTreeMap` の順）
    fn leaves_of(&self, text: &str) -> Vec<usize> {
        let t: String = text.split_whitespace().collect();
        if self.index.contains_key(t.as_str()) {
            return Vec::new();
        }
        let pre = format!("{t}.");
        let mut v: Vec<usize> = self
            .index
            .iter()
            .filter(|(k, _)| k.starts_with(&pre))
            .map(|(_, &v)| v)
            .collect();
        v.sort_unstable();
        v
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
            let text = c.syntax().text().to_string();
            if let Some((var, sel)) = self.resolve(&text) {
                out.push(Ref { var, sel, der });
            } else {
                let parts = self.parts(&text);
                if parts.is_empty() {
                    // レコードそのものを関数に渡す `angularVelocity2(frame_a.R)` は、欄すべてに触れるとみなす
                    // （Rumoca は関数の本体を見て欄を絞る。S334）
                    for var in self.leaves_of(&text) {
                        out.push(Ref {
                            var,
                            sel: Sel::All,
                            der,
                        });
                    }
                }
                for (j, var) in parts {
                    out.push(Ref {
                        var,
                        sel: Sel::Part(j),
                        der,
                    });
                }
            }
        }
        out
    }

    /// 部品の配列を通した欄の参照 `a.pin.v`（変数は `a.pin[1].v` …）の、要素ごとの（添字 0 から、変数）
    fn parts(&self, text: &str) -> Vec<(usize, usize)> {
        let t: String = text.chars().filter(|c| !c.is_whitespace()).collect();
        t.match_indices('.')
            .map(|(p, _)| {
                (1..)
                    .map_while(|i| {
                        let name = format!("{}[{i}]{}", &t[..p], &t[p..]);
                        self.index.get(name.as_str()).map(|&v| (i - 1, v))
                    })
                    .collect::<Vec<_>>()
            })
            .find(|v| !v.is_empty())
            .unwrap_or_default()
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

/// 第 1 引数が別のクロックの区画に属する演算子（MLS 16.3）
const CLOCK_OPS: [&str; 7] = [
    "sample",
    "hold",
    "subSample",
    "superSample",
    "shiftSample",
    "backSample",
    "noClock",
];
/// 結果がクロックの区画に入る演算子（`hold`・`noClock` の結果は連続の区画）
const CLOCK_RESULT_OPS: [&str; 5] = [
    "sample",
    "subSample",
    "superSample",
    "shiftSample",
    "backSample",
];

impl Ctx<'_> {
    /// 区画（MLS 16.5.1）の入力: 式に現れる既知でない変数（`CLOCK_OPS` の第 1 引数は別の区画なので除く）と、
    /// その式がクロックの区画に属する印（クロックの変換の結果、`previous`）
    fn partition(&self, roles: &[Role], node: &SyntaxNode) -> (Vec<usize>, bool) {
        let member = |c: &ast::ComponentRef| {
            self.resolve(&c.syntax().text().to_string())
                .map(|(v, _)| v)
                .filter(|&v| roles[v] != Role::Known)
        };
        let (mut vars, mut seed) = (Vec::new(), false);
        for n in node.descendants() {
            if let Some(call) = ast::CallExpr::cast(n.clone()) {
                match call.callee().as_deref() {
                    Some("previous") => seed = true,
                    Some(f) if CLOCK_RESULT_OPS.contains(&f) => {
                        // `sample(0, 0.1)`（イベント）のように第 1 引数が変数でなければ印にしない
                        seed |= call.args().first().is_some_and(|a| {
                            a.syntax()
                                .descendants()
                                .filter_map(ast::ComponentRef::cast)
                                .any(|c| member(&c).is_some())
                        });
                    }
                    _ => {}
                }
                continue;
            }
            let Some(c) = ast::ComponentRef::cast(n.clone()) else {
                continue;
            };
            if n.parent().is_some_and(|p| p.kind() == K::CallExpr) {
                continue; // 呼ぶ関数の名前
            }
            let r = n.text_range();
            let other = n
                .ancestors()
                .skip(1)
                .filter_map(ast::CallExpr::cast)
                .any(|call| {
                    call.callee()
                        .is_some_and(|f| CLOCK_OPS.contains(&f.as_str()))
                        && call
                            .args()
                            .first()
                            .is_some_and(|a| a.syntax().text_range().contains_range(r))
                });
            if !other && let Some(v) = member(&c) {
                vars.push(v);
            }
        }
        (vars, seed)
    }

    /// `when Clock() then`・`when <Clock 型の変数> then`（クロックの区画を宣言する `when`）
    fn is_clocked_when(&self, w: &SyntaxNode) -> bool {
        match w.children().find_map(Expr::cast) {
            Some(Expr::Call(c)) => c.callee().as_deref() == Some("Clock"),
            Some(Expr::ComponentRef(c)) => self
                .resolve(&c.syntax().text().to_string())
                .is_some_and(|(v, _)| self.vars[v].ty == "Clock"),
            _ => false,
        }
    }
}

/// 変数の区画（素集合）。クロックの区画に入る変数の根の集合を返す
struct Partitions(Vec<usize>);

impl Partitions {
    fn find(&mut self, mut x: usize) -> usize {
        while self.0[x] != x {
            self.0[x] = self.0[self.0[x]];
            x = self.0[x];
        }
        x
    }
    fn join(&mut self, vars: &[usize]) {
        if let Some((&a, rest)) = vars.split_first() {
            for &b in rest {
                let (ra, rb) = (self.find(a), self.find(b));
                self.0[ra] = rb;
            }
        }
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

/// 最初の枝が単純な代入だけの `if` 方程式の、代入先の離散のスカラー数と連続の変数。離散と連続が混ざり、
/// 代入先の大きさの和が方程式の数と一致するときだけ（それ以外は None）
fn split_if(
    cx: &Ctx,
    roles: &[Role],
    eq: &SyntaxNode,
    count: usize,
) -> Option<(usize, Vec<usize>)> {
    if kw_tokens(eq).next() != Some(K::KwIf) {
        return None;
    }
    let targets: Vec<usize> = first_branch(eq, K::Equation)
        .iter()
        .map(|e| match cx.lhs(e).as_slice() {
            [(v, _)] => Some(*v),
            _ => None,
        })
        .collect::<Option<_>>()?;
    let (disc, cont): (Vec<usize>, Vec<usize>) =
        targets.iter().partition(|&&v| roles[v] == Role::Discrete);
    let total: usize = targets.iter().map(|&v| cx.n(v)).sum();
    (total == count && !disc.is_empty() && !cont.is_empty())
        .then(|| (disc.iter().map(|&v| cx.n(v)).sum(), cont))
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

/// 連続側の方程式: 木・スカラーの数・印字・由来・（レコードの方程式なら）欄の変数
type EqRow = (SyntaxNode, usize, String, Option<Origin>, Vec<usize>);

struct Row {
    label: String,
    origin: EquationOrigin,
    at: Option<Origin>,
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
    let file = |o: &Origin| o.file.to_string();
    let span = |o: &Origin| TextRange::new(o.start, o.end);
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
    let mut eqs: Vec<EqRow> = Vec::new();
    for e in &m.equations {
        if e.count > 0 {
            let node = first_equation(&parse_in_model("equation", &e.text)?)?;
            // Rumoca は Clock 型の変数とその式を数えない（`c = Clock(…)`・`y = c`）
            if is_clock_equation(&cx, &node) {
                continue;
            }
            let leaves = cx.record_leaves(&node, e.count);
            let count = if leaves.is_empty() {
                e.count
            } else {
                leaves.iter().map(|&v| cx.n(v)).sum()
            };
            eqs.push((node, count, e.text.clone(), e.origin.clone(), leaves));
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
        // 束縛の中の `der`（`Real w = der(phi)`）も状態を作る
        .chain(binds.iter().map(|b| &b.1))
    {
        has_der.extend(cx.refs(node).iter().filter(|r| r.der).map(|r| r.var));
    }
    // 初期方程式の `der(x) = 0` も `x` を状態にする（Rumoca と同じ。MLS 8.6 は決めていない。S337）。
    // shortcut: 読めない初期方程式は黙って飛ばす（釣り合いには数えないので、状態の数だけが変わる）
    for e in &m.initial_equations {
        if let Ok(root) = parse_in_model("equation", &e.text) {
            has_der.extend(cx.refs(&root).iter().filter(|r| r.der).map(|r| r.var));
        }
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
    let mut roles: Vec<Role> = m
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

    // クロックの区画（MLS 16.5.1）: 方程式でつながる変数を 1 つの区画にし、`sample` などの結果・`previous`・
    // `when Clock()` を含む区画の Real を離散にする（Rumoca の DAE と同じ。クロックの値は持たず、区画の持ち主の整合は見ない）
    let mut part = Partitions((0..m.vars.len()).collect());
    let mut seeds: Vec<usize> = Vec::new();
    let mut eq_parts: Vec<Vec<usize>> = Vec::new();
    for e in &eqs {
        let (vars, seed) = cx.partition(&roles, &e.0);
        part.join(&vars);
        if seed {
            seeds.extend(vars.first());
        }
        eq_parts.push(vars);
    }
    let clocked_whens = whens
        .iter()
        .filter(|w| cx.is_clocked_when(&w.0))
        .map(|w| &w.0);
    for node in clocked_whens.chain(&algs) {
        let (vars, seed) = cx.partition(&roles, node);
        part.join(&vars);
        // `when Clock()` は必ず、algorithm は印があるときだけ区画を決める
        if seed || node.kind() == K::Equation {
            seeds.extend(vars.first());
        }
    }
    for (i, node) in &binds {
        let (mut vars, seed) = cx.partition(&roles, node);
        vars.push(*i);
        part.join(&vars);
        if seed {
            seeds.extend(vars.first());
        }
    }
    let owned: HashSet<usize> = seeds.iter().map(|&v| part.find(v)).collect();
    let in_owned: Vec<bool> = (0..roles.len())
        .map(|v| owned.contains(&part.find(v)))
        .collect();
    for (r, c) in roles.iter_mut().zip(in_owned) {
        if *r == Role::Algebraic && c {
            *r = Role::Discrete;
        }
    }
    let eq_clocked: Vec<bool> = eq_parts
        .iter()
        .map(|vars| vars.first().is_some_and(|&v| owned.contains(&part.find(v))))
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
                        file: v.origin.as_ref().map(file),
                        range: v.origin.as_ref().map(span),
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
    for (k, (node, count, text, at, leaves)) in eqs.iter().enumerate() {
        // `if` の枝が離散と連続の代入を混ぜるときは、代入先ごとに分ける（Rumoca は代入ごとの方程式）
        if !eq_clocked[k]
            && let Some((disc, cont)) = split_if(&cx, &roles, node, *count)
        {
            discrete_equations += disc;
            let refs = cx.refs(node);
            for v in cont {
                rows.push(Row {
                    label: label(text),
                    origin: EquationOrigin::Source,
                    at: at.clone(),
                    count: cx.n(v),
                    refs: refs.clone(),
                    own: vec![(v, Sel::All)],
                });
            }
        } else if defines_discrete(&cx, &roles, node) || eq_clocked[k] {
            discrete_equations += count;
        } else {
            // レコードの方程式の離散の欄（`phase` など）は離散の方程式に数える（Rumoca は欄ごとの方程式）
            let disc: usize = leaves
                .iter()
                .filter(|&&v| roles[v] == Role::Discrete)
                .map(|&v| cx.n(v))
                .sum();
            discrete_equations += disc;
            if disc == *count {
                continue;
            }
            rows.push(Row {
                label: label(text),
                origin: EquationOrigin::Source,
                at: at.clone(),
                count: *count - disc,
                refs: cx.refs(node),
                // レコードの方程式は、どの行も欄の変数すべてに触れる（shortcut: 欄ごとの対応は付けない。正則は甘く出る方に傾く）
                own: leaves.iter().map(|&v| (v, Sel::All)).collect(),
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
                at: m.vars[*i].origin.clone(),
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
                    Sel::Part(_) => false,
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
                    at: None,
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
                    Sel::Part(j) if r.count > 1 => (k == j).then_some(0).into_iter().collect(),
                    Sel::Part(_) => vec![0],
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
                file: r.at.as_ref().map(file),
                range: r.at.as_ref().map(span),
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
    fn der_in_initial_equation_makes_state() {
        // y は代数変数だが、初期方程式で `der(y)` を使うので Rumoca と同じく状態に数える
        let d = dae(
            "model M Real x(start=1); Real y; equation der(x) = -x; y = 2 * x; \
             initial equation der(y) = 0; end M;",
        );
        assert_eq!(
            (d.counts.states, d.counts.unknowns, d.counts.equations),
            (2, 2, 2)
        );
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

    #[test]
    fn record_equation_counts_per_field() {
        // `y = C(a, b)` は欄ごとの 2 本（flat は 1 本）。平らにした方程式の数は 1
        let src = "record C Real re; Real im; end C;
            model M C y; Real a = time; Real b = 2 * time; equation y = C(a, b); end M;";
        let m = Project::from_sources([("t.mo", src)]).flatten("M").unwrap();
        assert_eq!(m.equation_count(), 1);
        let d = build(&m).unwrap();
        assert_eq!((d.counts.equations, d.counts.unknowns), (4, 4));
        assert!(d.is_regular());
    }

    /// S351（#133）: レコードの配列への方程式は、要素ごとの欄に展開して数える（for 文の形・配列の形・配列どうしで同じ）
    #[test]
    fn record_array_equation_counts_per_field() {
        let rec = "operator record Complex Real re; Real im; end Complex;\n";
        for body in [
            "for i in 1:3 loop aw[i] = Complex(i, 0); end for; s = aw[1];",
            "aw = {Complex(i, 0) for i in 1:3}; s = aw[1];",
            "aw = {Complex(1, 0), Complex(2, 0), Complex(3, 0)}; s = aw[1];",
        ] {
            let d = dae(&format!(
                "{rec}model M Complex aw[3]; Complex s; equation {body} end M;"
            ));
            assert_eq!((d.counts.equations, d.counts.unknowns), (8, 8), "{body}");
            assert!(d.is_balanced(), "{body}");
        }
        let d = dae(&format!(
            "{rec}model M Complex aw[3]; Complex bw[3]; equation aw = bw; \
             bw = {{Complex(1, 0), Complex(2, 0), Complex(3, 0)}}; end M;"
        ));
        assert_eq!((d.counts.equations, d.counts.unknowns), (12, 12));
        // 長さ 1 の配列も、要素の欄 2 本
        let d = dae(&format!(
            "{rec}model M Complex aw[1]; equation aw = {{Complex(1, 0)}}; end M;"
        ));
        assert_eq!((d.counts.equations, d.counts.unknowns), (2, 2));
    }

    #[test]
    fn der_in_a_binding_makes_a_state_and_empty_arrays_have_no_unknowns() {
        let d =
            dae("model M Real x(start = 0); Real w = der(x); Real e[0]; equation x = time; end M;");
        assert_eq!(d.counts.states, 1);
        // x（状態）の 1 本の方程式、w の束縛の 1 本。未知数は x の状態と w。e[0] は数えない
        assert_eq!((d.counts.equations, d.counts.unknowns), (2, 2));
    }

    #[test]
    fn clocked_partition_is_discrete() {
        // y = sample(x, c) とそこからつながる v は連続の行に入らない。hold の結果 w は連続
        let d = dae(
            "model M Real x(start = 0); Real y; Real v; Real w; Clock c = Clock(0.1); \
             equation der(x) = 1; y = sample(x, c); v = 2 * y; w = hold(v) + x; end M;",
        );
        assert_eq!(d.counts.discrete, 2);
        assert_eq!(d.discrete_equations, 2);
        // der(x) と w の 2 本。w は x の状態にも触れる
        assert_eq!(d.structure.equations.len(), 2);
        assert!(d.is_balanced() && d.is_regular());
    }

    #[test]
    fn if_equation_splits_discrete_and_continuous_targets() {
        let d = dae("model M Real x; Boolean b; Real y; equation \
             if time > 1 then x = 1; b = true; else x = 2; b = false; end if; y = x; end M;");
        // x は連続の 1 行、b は離散、y = x は連続の 1 行（if 全体を離散にしない）
        assert_eq!((d.structure.equations.len(), d.discrete_equations), (2, 1));
        assert!(d.is_balanced() && d.is_regular());
    }

    #[test]
    fn record_equation_discrete_field_is_discrete_equation() {
        let src = "record S Real p; Integer k; end S;
            model M S s; equation s = S(time, 1); end M;";
        let d = build(&Project::from_sources([("t.mo", src)]).flatten("M").unwrap()).unwrap();
        assert_eq!((d.structure.equations.len(), d.discrete_equations), (1, 1));
        assert!(d.is_balanced());
    }

    #[test]
    fn field_through_a_component_array_touches_each_element() {
        // 宣言より前の束縛 `x = p.pin.v + 1` は `p.pin.v` のまま残る。行 k は `p.pin[k].v` に触れる
        let src = "model Pin Real v; end Pin;
            model Plug Pin pin[2]; end Plug;
            model M Real x[2] = p.pin.v + 1; Plug p; equation p.pin[1].v = 1; p.pin[2].v = 2; end M;";
        let m = Project::from_sources([("t.mo", src)]).flatten("M").unwrap();
        let d = build(&m).unwrap();
        assert!(d.is_balanced() && d.is_regular());
        // 行 0・1 は pin[k].v = k、行 2・3 が x の束縛。未知数は pin[1].v・pin[2].v・x[1]・x[2]
        assert_eq!(d.structure.incidence[2], [0, 2]);
        assert_eq!(d.structure.incidence[3], [1, 3]);
    }

    #[test]
    fn whole_record_argument_touches_every_field() {
        // `y = f(r)` の `r`（レコードそのもの）は、欄すべてを含む（Rumoca は関数の本体を見て絞る。S334）
        let d = dae("record R Real p; Real q; end R;
            function f input R r; output Real y; algorithm y := r.p; end f;
            model M R r; Real y; equation r.p = 1; r.q = 2; y = f(r); end M;");
        assert_eq!(d.structure.incidence[2], [0, 1, 2]);
    }

    #[test]
    fn field_after_a_call_of_a_record_function_is_read() {
        // 欄ごとの束縛 `r.T = f(time).T`（呼び出しのあとの欄）を読み直せる
        let d = dae("record R Real T; Real p; end R;
            function f input Real x; output R y; algorithm y.T := x; y.p := 2 * x; end f;
            model M R r = f(time); end M;");
        assert_eq!((d.counts.equations, d.counts.unknowns), (2, 2));
    }
}

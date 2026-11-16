//! 関数の評価器（S370）。定数の評価（`eval`）の中で、入力が定数に決まる Modelica の関数を実行して出力を返す。
//!
//! - 実行できるのは `function` の `algorithm` と、出力・保護変数の束縛 `= e`。値は整数・実数・論理のスカラーと 1 次元の配列。
//!   文は代入（`x := e`・`x[i] := e`）・`if`・`for`（添字 1 つ）・`while`・`return`・`break`・`assert`（条件が偽なら分からないとする）。
//! - `external` の関数・`impure`・`extends` で入力や出力を受け継ぐ関数・文字列やレコードの変数・`print` などの副作用は評価しない（分からない）。
//! - 暴走を防ぐ: 仕事量の上限は 1 回の flat 全体で 1 つ（`MAX_ITERS`。呼び出し 1 回は `CALL_COST`、文・反復は 1、配列の確保・複製・走査は要素の数なので、確保する配列の要素の総数もこれに収まる）。
//!   同じ関数・同じスカラーの引数の結果は覚える（`fn_memo`）。再帰の深さの上限は `MAX_CALL_DEPTH`、文・式の入れ子の上限は `MAX_NEST`。
//!
//! shortcut: 配列は 1 次元・要素はスカラーだけ（配列どうしの演算・内包・`zeros` などは分からない）。必要になったら足す。

use super::{Ctx, F, FN, MAX_ITERS, Ty, Val, binop, ord, single, tokens};
use globe_mode_resolve::{ClassId, Resolution};
use globe_mode_syntax::ast::{self, Element, Expr};
use globe_mode_syntax::{SyntaxKind as K, SyntaxNode};
use std::collections::HashMap;
use std::rc::Rc;

/// 関数の呼び出しの入れ子の上限（MSL の再帰は `numberOfSymmetricBaseSystems` の log2 段）と、1 つの呼び出しの中の文・式の入れ子の上限。
/// 積がスタックの最大の使い方になる。release で 1 段の入れ子が約 0.5 KB なので、16 × 32 で約 0.25 MB（wasm の 1 MiB に収まる。32 × 48 は約 0.75 MB だった）
const MAX_CALL_DEPTH: u32 = 16;
const MAX_NEST: u32 = 32;
/// 関数の呼び出し 1 回の重さ（Frame・文脈を作る分。S374）。仕事量の上限は 1 回の flat で `MAX_ITERS`（S370 は呼び出しごとに戻していた）。
/// S370 の FFT の 4 本（`realFFTsamplePoints`）は 1300 足らずで通る
const CALL_COST: usize = 64;

/// 式の名前だけで決まるスカラーの組み込み関数（`eval` と共有）
pub(super) const BUILTINS: &[&str] = &[
    "abs", "sqrt", "floor", "ceil", "integer", "div", "mod", "rem", "sign", "sin", "cos", "tan",
    "asin", "acos", "atan", "atan2", "sinh", "cosh", "tanh", "exp", "log", "log10",
];

pub(super) fn builtin(name: &str, a: &[Val]) -> Option<Val> {
    Some(match (name, a) {
        ("abs", [Val::I(i)]) => Val::I(i.checked_abs()?),
        ("abs", [x]) => Val::R(x.f().abs()),
        // 整数はそのまま（f64 を通すと 2^53 を超えて値が変わる）。`i64` に収まらない `floor`・`ceil` は Real のまま、`integer` は決まらない
        ("floor" | "ceil" | "integer", [Val::I(i)]) => Val::I(*i),
        ("floor", [x]) => int_or_real(x.f().floor()),
        ("ceil", [x]) => int_or_real(x.f().ceil()),
        ("integer", [x]) => Val::I(int_or_real(x.f().floor()).int()?),
        // `i64::MIN / -1` は溢れるので `checked_div`（`mod`・`rem` の余りは `wrapping_rem` で 0）
        ("div", [Val::I(a), Val::I(b)]) => Val::I(a.checked_div(*b)?),
        ("div", [x, y]) if y.f() != 0.0 => int_or_real((x.f() / y.f()).trunc()),
        ("rem", [Val::I(a), Val::I(b)]) if *b != 0 => Val::I(a.wrapping_rem(*b)),
        ("mod", [Val::I(a), Val::I(b)]) if *b != 0 => {
            let r = a.wrapping_rem(*b);
            Val::I(if r != 0 && (r < 0) != (*b < 0) {
                r + b
            } else {
                r
            })
        }
        ("mod", [x, y]) if y.f() != 0.0 => Val::R(x.f() - (x.f() / y.f()).floor() * y.f()),
        ("rem", [x, y]) if y.f() != 0.0 => Val::R(x.f() - (x.f() / y.f()).trunc() * y.f()),
        ("sign", [x]) => Val::I(if x.f() > 0.0 {
            1
        } else if x.f() < 0.0 {
            -1
        } else {
            0
        }),
        ("atan2", [y, x]) => Val::R(y.f().atan2(x.f())),
        (n, [x]) => Val::R(match n {
            "sqrt" => x.f().sqrt(),
            "sin" => x.f().sin(),
            "cos" => x.f().cos(),
            "tan" => x.f().tan(),
            "asin" => x.f().asin(),
            "acos" => x.f().acos(),
            "atan" => x.f().atan(),
            "sinh" => x.f().sinh(),
            "cosh" => x.f().cosh(),
            "tanh" => x.f().tanh(),
            "exp" => x.f().exp(),
            "log" => x.f().ln(),
            "log10" => x.f().log10(),
            _ => return None,
        }),
        _ => return None,
    })
}

/// `i64` に収まる整数値の実数は `Val::I`、収まらない（NaN・無限大・`1e30`）なら `Val::R` のまま
fn int_or_real(x: f64) -> Val {
    if (i64::MIN as f64..-(i64::MIN as f64)).contains(&x) {
        Val::I(x as i64)
    } else {
        Val::R(x)
    }
}

/// 要素の最大・最小（空は `None`。型は要素のまま。整数どうしは整数のまま比べ、決まらない大小（NaN・文字列）は `None`）
pub(super) fn extremum(xs: &[Val], is_max: bool) -> Option<Val> {
    let (first, rest) = xs.split_first()?;
    rest.iter().try_fold(*first, |a, &b| {
        Some(if ord(a, b)?.is_ge() == is_max { a } else { b })
    })
}

pub(super) fn unop(op: K, v: Val) -> Option<Val> {
    match (op, v) {
        (K::Minus, Val::I(i)) => i.checked_neg().map(Val::I),
        (K::Minus, v) => Some(Val::R(-v.f())),
        (K::Plus, v) => Some(v),
        (K::KwNot, Val::B(b)) => Some(Val::B(!b)),
        _ => None,
    }
}

/// 呼び出しの結果を覚える鍵: 関数・`heir`・引数（スカラーだけ）の `Debug`
pub(super) type MemoKey = (ClassId, Option<ClassId>, String);

/// 配列は `Rc`（読み・引数の受け渡しで複製しない。書き込みは `Rc::make_mut`）
#[derive(Clone, Debug)]
pub(super) enum V {
    S(Val),
    A(Rc<Vec<Val>>),
}

/// 関数の変数。`ty` は宣言の型（`R` 実数・`I` 整数・`B` 論理）で、代入のときに値を合わせる
struct Slot {
    ty: char,
    v: V,
}

struct Frame {
    file: u32,
    /// 関数の中の定数・パッケージの定数を引く文脈（`pfx` は `FN`）
    ctx: Rc<Ctx>,
    vars: HashMap<String, Slot>,
}

#[derive(PartialEq)]
enum Flow {
    Next,
    Break,
    Return,
}

/// 型 `ty` の値にする（整数は整数の実数も受け、実数は整数も受ける）
fn coerce(ty: char, v: Val) -> Option<Val> {
    match (ty, v) {
        ('R', Val::I(_) | Val::R(_)) => Some(Val::R(v.f())),
        ('I', v) if !matches!(v, Val::B(_)) => v.int().map(Val::I),
        ('B', Val::B(_)) => Some(v),
        _ => None,
    }
}

fn zero(ty: char) -> Val {
    match ty {
        'R' => Val::R(0.0),
        'I' => Val::I(0),
        _ => Val::B(false),
    }
}

/// 宣言の大きさ `dims`（`None` は `[:]`）に合う値にする
fn conform(ty: char, dims: &[Option<usize>], v: V) -> Option<V> {
    match (dims, v) {
        ([], V::S(x)) => coerce(ty, x).map(V::S),
        ([n], V::A(a)) if n.is_none_or(|n| n == a.len()) => a
            .iter()
            .map(|x| coerce(ty, *x))
            .collect::<Option<_>>()
            .map(|v| V::A(Rc::new(v))),
        _ => None,
    }
}

fn scalars(vs: Vec<V>) -> Option<Vec<Val>> {
    vs.into_iter()
        .map(|v| match v {
            V::S(x) => Some(x),
            V::A(_) => None,
        })
        .collect()
}

impl F<'_> {
    /// `eval` から: 関数の呼び出し（引数は定数に評価できるスカラーと `{…}`）。値がスカラーのときだけ返す
    pub(super) fn call_fn(
        &self,
        c: &ast::CallExpr,
        file: u32,
        ctx: &Rc<Ctx>,
        d: u32,
    ) -> Option<Val> {
        let r = c.syntax().children().find_map(ast::ComponentRef::cast)?;
        let Some(Resolution::Class(rid)) = self.p.res_of(file, r.range()) else {
            return None;
        };
        let arg = |e: &Expr| {
            (self.eval(e.syntax(), file, ctx, d).map(V::S))
                .or_else(|| (self.eval_items(e.syntax(), file, ctx, d)).map(|v| V::A(Rc::new(v))))
        };
        let pos = c.args().iter().map(arg).collect::<Option<Vec<_>>>()?;
        let named = (c.named_args().into_iter())
            .map(|(k, e)| Some((k, arg(&e)?)))
            .collect::<Option<Vec<_>>>()?;
        match self.run_fn(rid, pos, named, ctx)? {
            V::S(v) => Some(v),
            V::A(_) => None,
        }
    }

    /// 仕事量を `n` 数える（上限は 1 回の flat 全体で `MAX_ITERS`）
    fn tick_n(&self, n: usize) -> Option<()> {
        let t = self.fn_steps.get().saturating_add(n);
        self.fn_steps.set(t);
        (t <= MAX_ITERS).then_some(())
    }

    fn tick(&self) -> Option<()> {
        self.tick_n(1)
    }

    fn run_fn(
        &self,
        rid: ClassId,
        pos: Vec<V>,
        named: Vec<(String, V)>,
        ctx: &Rc<Ctx>,
    ) -> Option<V> {
        let depth = self.fn_depth.get();
        if depth >= MAX_CALL_DEPTH {
            return None;
        }
        // 結果を覚えるのは、引数がスカラーだけで、`redeclare` の差し替えが無いとき（関数の中で引く定数が呼び出しの文脈で変わらない）。成功した値だけ
        let key = (ctx.cenv.try_borrow().is_ok_and(|c| c.is_empty())
            && (pos.iter().chain(named.iter().map(|(_, v)| v))).all(|v| matches!(v, V::S(_))))
        .then(|| (rid, ctx.heir, format!("{pos:?}{named:?}")));
        if let Some(v) = key
            .as_ref()
            .and_then(|k| self.fn_memo.borrow().get(k).cloned())
        {
            return Some(v);
        }
        self.fn_depth.set(depth + 1);
        let r = self.run_body(rid, pos, named, ctx);
        self.fn_depth.set(depth);
        if let (Some(k), Some(v @ V::S(_))) = (key, &r) {
            self.fn_memo.borrow_mut().insert(k, v.clone());
        }
        r
    }

    fn run_body(
        &self,
        rid: ClassId,
        pos: Vec<V>,
        named: Vec<(String, V)>,
        ctx: &Rc<Ctx>,
    ) -> Option<V> {
        let cd = self.p.class(rid)?;
        let impure = tokens(cd.syntax())
            .take_while(|t| t.kind() != K::Ident)
            .any(|t| t.kind() == K::KwImpure);
        if impure
            || cd.kind_token()?.kind() != K::KwFunction
            || (cd.syntax().children()).any(|c| c.kind() == K::ExternalClause)
        {
            return None;
        }
        // 断る関数（上の条件）は Frame を作らないので数えない（`eval` は同じ式を何度も評価する）
        self.tick_n(CALL_COST)?;
        let file = self.p.lib.classes[rid].file;
        let mut fr = Frame {
            file,
            ctx: Rc::new(Ctx {
                pfx: FN.to_string(),
                lineage: self.p.lineage(rid),
                locals: Vec::new(),
                cenv: ctx.cenv.clone(),
                heir: ctx.heir,
            }),
            vars: HashMap::new(),
        };
        let (mut n_in, mut out) = (0, None);
        for e in cd.elements() {
            match e {
                // 入力・出力を持つ基底（`Icons.Function` のように何も持たないものだけ通す）
                Element::Extends(x) => {
                    let Some(Resolution::Class(b)) =
                        x.name().and_then(|n| self.p.res_of(file, n.range()))
                    else {
                        return None;
                    };
                    let bd = self.p.class(b)?;
                    if (bd.elements().iter())
                        .any(|e| matches!(e, Element::Component(_) | Element::Extends(_)))
                        || (bd.syntax().children())
                            .any(|c| matches!(c.kind(), K::AlgorithmSection | K::ExternalClause))
                    {
                        return None;
                    }
                }
                Element::Component(cc) => {
                    let tn = cc.type_name()?;
                    let (Ty::Prim { ty, .. }, tdims) = self
                        .type_of(self.p.res_of(file, tn.range()), &tn.parts())
                        .ok()?
                    else {
                        return None;
                    };
                    let t = match ty.as_str() {
                        "Real" => 'R',
                        "Integer" => 'I',
                        "Boolean" => 'B',
                        _ => return None,
                    };
                    let toks: Vec<K> = tokens(cc.syntax()).map(|t| t.kind()).collect();
                    let (is_in, is_out) = (toks.contains(&K::KwInput), toks.contains(&K::KwOutput));
                    for dc in cc.declarations() {
                        let (name, _) = dc.name()?;
                        let mut dims = Vec::new();
                        for sn in (dc.syntax().children().chain(cc.syntax().children()))
                            .filter(|c| c.kind() == K::Subscripts)
                        {
                            for el in sn.children_with_tokens() {
                                if el.as_token().is_some_and(|t| t.kind() == K::Colon) {
                                    dims.push(None);
                                } else if let Some(x) =
                                    el.as_node().filter(|x| Expr::cast((*x).clone()).is_some())
                                {
                                    let n = self.fx(x, &fr, 0)?;
                                    let n = usize::try_from(scalars(vec![n])?[0].int()?).ok()?;
                                    dims.push(Some(n).filter(|n| *n <= MAX_ITERS)?.into());
                                }
                            }
                        }
                        if !tdims.is_empty() || dims.len() > 1 {
                            return None;
                        }
                        let bind = (dc.syntax().children().find(|c| c.kind() == K::Modification))
                            .and_then(|m| m.children().find_map(Expr::cast));
                        let v = if is_in {
                            let given = (named.iter().find(|(k, _)| *k == name).map(|(_, v)| v))
                                .or_else(|| pos.get(n_in));
                            n_in += 1;
                            match given {
                                Some(v) => v.clone(),
                                None => self.fx(bind?.syntax(), &fr, 0)?,
                            }
                        } else if let Some(b) = bind {
                            self.fx(b.syntax(), &fr, 0)?
                        } else {
                            match dims.first() {
                                None => V::S(zero(t)),
                                Some(n) => V::A(Rc::new(vec![zero(t); n.unwrap_or(0)])),
                            }
                        };
                        if is_out && out.is_none() {
                            out = Some(name.clone());
                        }
                        let v = conform(t, &dims, v)?;
                        // 配列の確保・複製は要素の数だけ数える（確保する要素の総数が `MAX_ITERS` に収まる。S374・#149）
                        if let V::A(a) = &v {
                            self.tick_n(a.len())?;
                        }
                        fr.vars.insert(name, Slot { ty: t, v });
                    }
                }
                _ => {}
            }
        }
        // 余った位置引数・存在しない名前つき引数は、呼び出しの誤り
        if pos.len() > n_in || named.iter().any(|(k, _)| !fr.vars.contains_key(k)) {
            return None;
        }
        for sec in cd
            .syntax()
            .children()
            .filter(|c| c.kind() == K::AlgorithmSection)
        {
            let stmts: Vec<_> = sec
                .children()
                .filter(|c| c.kind() == K::Statement)
                .collect();
            match self.block(&stmts, &mut fr, 0)? {
                Flow::Next => {}
                Flow::Return => break,
                Flow::Break => return None,
            }
        }
        Some(fr.vars.remove(&out?)?.v)
    }

    fn block(&self, stmts: &[SyntaxNode], fr: &mut Frame, d: u32) -> Option<Flow> {
        for s in stmts {
            match self.exec(s, fr, d)? {
                Flow::Next => {}
                f => return Some(f),
            }
        }
        Some(Flow::Next)
    }

    fn cond(&self, e: &SyntaxNode, fr: &Frame, d: u32) -> Option<bool> {
        match self.fx(e, fr, d)? {
            V::S(Val::B(b)) => Some(b),
            _ => None,
        }
    }

    fn exec(&self, st: &SyntaxNode, fr: &mut Frame, d: u32) -> Option<Flow> {
        self.tick()?;
        if d > MAX_NEST {
            return None;
        }
        let d = d + 1;
        let body =
            || -> Vec<SyntaxNode> { st.children().filter(|c| c.kind() == K::Statement).collect() };
        match tokens(st).next()?.kind() {
            K::KwReturn => Some(Flow::Return),
            K::KwBreak => Some(Flow::Break),
            K::KwIf => {
                // 上から順に条件を評価し、最初に真の枝の文だけを集める（そのあとの条件は評価しない）
                let (mut want, mut taken, mut active) = (false, false, false);
                let mut sel = Vec::new();
                for e in st.children_with_tokens() {
                    match e {
                        rowan::NodeOrToken::Token(t) => match t.kind() {
                            K::KwIf | K::KwElseif => want = true,
                            K::KwElse => (active, taken) = (!taken, true),
                            _ => {}
                        },
                        rowan::NodeOrToken::Node(n) if want && Expr::cast(n.clone()).is_some() => {
                            want = false;
                            active = !taken && self.cond(&n, fr, d)?;
                            taken |= active;
                        }
                        rowan::NodeOrToken::Node(n) if active && n.kind() == K::Statement => {
                            sel.push(n)
                        }
                        _ => {}
                    }
                }
                self.block(&sel, fr, d)
            }
            K::KwWhile => {
                let c = st.children().find(|c| Expr::cast(c.clone()).is_some())?;
                let stmts = body();
                while self.cond(&c, fr, d)? {
                    self.tick()?;
                    match self.block(&stmts, fr, d)? {
                        Flow::Next => {}
                        Flow::Break => break,
                        Flow::Return => return Some(Flow::Return),
                    }
                }
                Some(Flow::Next)
            }
            K::KwFor => {
                let fi = single(
                    (st.children().find(|c| c.kind() == K::ForIndices)?)
                        .children()
                        .filter(|c| c.kind() == K::ForIndex),
                )?;
                let var = tokens(&fi)
                    .find(|t| t.kind() == K::Ident)?
                    .text()
                    .to_string();
                let vals = self.for_vals(&fi.children().find_map(Expr::cast)?, fr, d)?;
                let (stmts, saved) = (body(), fr.vars.remove(&var));
                let mut flow = Flow::Next;
                for v in vals {
                    self.tick()?;
                    fr.vars.insert(
                        var.clone(),
                        Slot {
                            ty: 'I',
                            v: V::S(Val::I(v)),
                        },
                    );
                    match self.block(&stmts, fr, d)? {
                        Flow::Next => {}
                        Flow::Break => break,
                        Flow::Return => {
                            flow = Flow::Return;
                            break;
                        }
                    }
                }
                fr.vars.remove(&var);
                fr.vars.extend(saved.map(|s| (var, s)));
                Some(flow)
            }
            _ => {
                let mut es = st.children().filter_map(Expr::cast);
                let lhs = es.next()?;
                let Some(rhs) = es.next() else {
                    // 副作用のある呼び出しは評価しない。`assert` だけは条件を確かめる
                    let Expr::Call(c) = lhs else { return None };
                    return (c.callee()? == "assert"
                        && self.cond(c.args().first()?.syntax(), fr, d)?)
                    .then_some(Flow::Next);
                };
                let v = self.fx(rhs.syntax(), fr, d)?;
                self.assign(&lhs, v, fr, d)?;
                Some(Flow::Next)
            }
        }
    }

    /// `x := v`・`x[i] := v`
    fn assign(&self, lhs: &Expr, v: V, fr: &mut Frame, d: u32) -> Option<()> {
        let Expr::ComponentRef(c) = lhs else {
            return None;
        };
        let parts = c.parts();
        let [name] = parts.as_slice() else {
            return None;
        };
        let at = self.index(lhs.syntax(), fr, d)?;
        let slot = fr.vars.get_mut(name)?;
        match (at, v, &mut slot.v) {
            (None, v, cur) => {
                let dims = match cur {
                    V::S(_) => vec![],
                    V::A(a) => vec![Some(a.len())],
                };
                if let V::A(a) = &v {
                    self.tick_n(a.len())?;
                }
                *cur = conform(slot.ty, &dims, v)?;
            }
            (Some(i), V::S(x), V::A(a)) => *Rc::make_mut(a).get_mut(i)? = coerce(slot.ty, x)?,
            _ => return None,
        }
        Some(())
    }

    /// 参照 `x[i]` の添字（0 始まり）。添字が無ければ `Some(None)`、`[i, j]`・`[:]` は `None`
    fn index(&self, n: &SyntaxNode, fr: &Frame, d: u32) -> Option<Option<usize>> {
        let mut subs = n.children().filter(|c| c.kind() == K::Subscripts);
        let Some(sn) = subs.next() else {
            return Some(None);
        };
        let e = single(sn.children().filter(|c| Expr::cast(c.clone()).is_some()))?;
        if subs.next().is_some() || tokens(&sn).any(|t| t.kind() == K::Colon) {
            return None;
        }
        let i = scalars(vec![self.fx(&e, fr, d)?])?[0].int()?;
        Some(Some(usize::try_from(i.checked_sub(1)?).ok()?))
    }

    /// `for` の範囲 `a:b`・`a:s:b`・`{…}` の整数の値（反復の数は `MAX_ITERS` まで。範囲は配列を作らず 1 つずつ出す）
    fn for_vals(&self, e: &Expr, fr: &Frame, d: u32) -> Option<Box<dyn Iterator<Item = i64>>> {
        let int = |x: &Expr| scalars(vec![self.fx(x.syntax(), fr, d)?])?[0].int();
        if let Expr::Range(r) = e {
            let (a, s, b) = match r.parts().as_slice() {
                [a, b] => (int(a)?, 1, int(b)?),
                [a, s, b] => (int(a)?, int(s).filter(|s| *s != 0)?, int(b)?),
                _ => return None,
            };
            let span = if s > 0 {
                b.checked_sub(a)?
            } else {
                a.checked_sub(b)?
            };
            // `k * |s| <= span` なので `a + k * s` は溢れない。反復の数 `span / |s| + 1` は `i64::MAX` で溢れうる（S374・#151）
            let n = if span < 0 {
                0
            } else {
                (span / s.checked_abs()?).checked_add(1)?
            };
            return (n <= MAX_ITERS as i64).then(|| {
                Box::new((0..n).map(move |k| a + k * s)) as Box<dyn Iterator<Item = i64>>
            });
        }
        match self.fx(e.syntax(), fr, d)? {
            V::A(a) => {
                self.tick_n(a.len())?;
                let v = a.iter().map(|x| x.int()).collect::<Option<Vec<_>>>()?;
                Some(Box::new(v.into_iter()))
            }
            V::S(_) => None,
        }
    }

    /// 関数の中の式
    fn fx(&self, n: &SyntaxNode, fr: &Frame, d: u32) -> Option<V> {
        if d > MAX_NEST {
            return None;
        }
        let d = d + 1;
        let s = |x: &SyntaxNode| scalars(vec![self.fx(x, fr, d)?]).map(|v| v[0]);
        match Expr::cast(n.clone())? {
            Expr::Paren(p) => match p.items().as_slice() {
                [e] => self.fx(e.syntax(), fr, d),
                _ => None,
            },
            Expr::Unary(u) => unop(u.op()?.kind(), s(u.operand()?.syntax())?).map(V::S),
            Expr::Binary(b) => {
                let (operands, ops) = (b.operands(), b.ops());
                let mut acc = s(operands.first()?.syntax())?;
                for (o, op) in operands[1..].iter().zip(ops.iter()) {
                    // `and`・`or` は左で決まれば右を評価しない（`size(b, 1) > 0 and max(b)`）
                    if matches!(
                        (op.kind(), acc),
                        (K::KwAnd, Val::B(false)) | (K::KwOr, Val::B(true))
                    ) {
                        break;
                    }
                    acc = binop(op.kind(), acc, s(o.syntax())?)?;
                }
                Some(V::S(acc))
            }
            Expr::If(i) => {
                for (c, v) in i.branches() {
                    if let Val::B(true) = s(c.syntax())? {
                        return self.fx(v.syntax(), fr, d);
                    }
                }
                self.fx(i.else_branch()?.syntax(), fr, d)
            }
            Expr::Array(a) if !n.children().any(|c| c.kind() == K::Comprehension) => a
                .items()
                .iter()
                .map(|e| s(e.syntax()))
                .collect::<Option<_>>()
                .map(|v| V::A(Rc::new(v))),
            Expr::ComponentRef(c) => {
                if let [name] = c.parts().as_slice()
                    && let Some(slot) = fr.vars.get(name)
                {
                    return match (self.index(n, fr, d)?, &slot.v) {
                        (None, v) => Some(v.clone()),
                        (Some(i), V::A(a)) => a.get(i).map(|x| V::S(*x)),
                        _ => None,
                    };
                }
                self.eval(n, fr.file, &fr.ctx, d).map(V::S)
            }
            Expr::Call(c) => {
                let (name, args) = (c.callee()?, c.args());
                let vals = || {
                    args.iter()
                        .map(|a| self.fx(a.syntax(), fr, d))
                        .collect::<Option<Vec<_>>>()
                };
                match name.as_str() {
                    "size" => {
                        let local = match args.first() {
                            Some(Expr::ComponentRef(r)) => match r.parts().as_slice() {
                                [x] => fr.vars.get(x),
                                _ => None,
                            },
                            _ => None,
                        };
                        let Some(Slot { v: V::A(a), .. }) = local else {
                            return self.eval(n, fr.file, &fr.ctx, d).map(V::S);
                        };
                        (s(args.get(1)?.syntax())?.int()? == 1)
                            .then_some(V::S(Val::I(a.len() as i64)))
                    }
                    "max" | "min" => {
                        let vs = vals()?;
                        let sc;
                        let xs: &[Val] = match vs.as_slice() {
                            [V::A(a)] => a,
                            _ => {
                                sc = scalars(vs.clone())?;
                                &sc
                            }
                        };
                        self.tick_n(xs.len())?;
                        extremum(xs, name == "max").map(V::S)
                    }
                    f if BUILTINS.contains(&f) => builtin(f, &scalars(vals()?)?).map(V::S),
                    _ => {
                        let r = c.syntax().children().find_map(ast::ComponentRef::cast)?;
                        let Some(Resolution::Class(rid)) = self.p.res_of(fr.file, r.range()) else {
                            return None;
                        };
                        let pos = vals()?;
                        let named = (c.named_args().into_iter())
                            .map(|(k, e)| Some((k, self.fx(e.syntax(), fr, d)?)))
                            .collect::<Option<Vec<_>>>()?;
                        self.run_fn(rid, pos, named, &fr.ctx)
                    }
                }
            }
            // 数・真偽・文字列・列挙・パッケージの定数
            _ => self.eval(n, fr.file, &fr.ctx, d).map(V::S),
        }
    }
}

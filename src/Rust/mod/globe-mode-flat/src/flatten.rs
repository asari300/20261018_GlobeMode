//! 平らにする本体。クラスをインスタンス化し（`extends` を取り込み、修飾を外側から当てる）、変数と方程式を集める。

use crate::{Causality, Equation, FlatModel, Origin, Project, Res, Var, VarKind, un};
use crate::{Snip, SnipItem};
use globe_mode_resolve::{ClassId, Reason, Ref, RefKind, Resolution};
use globe_mode_syntax::ast::{self, Element, Expr};
use globe_mode_syntax::{SyntaxKind as K, SyntaxNode};
use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::rc::Rc;

mod fneval;
mod graph;
mod stream;

use fneval::{BUILTINS, builtin, extremum, unop};
use stream::OUT_STREAM;

/// 修飾 `x(start = 1) = 2`。値は書いた場所の文脈（`Bind::ctx`）で名前を付け替える
#[derive(Clone, Default)]
struct Mod {
    val: Option<Bind>,
    sub: ModMap,
    each: bool,
    /// `redeclare …`（S297）。外側が勝つ
    redecl: Option<Rc<Redecl>>,
}

/// 修飾の `redeclare`。型の名前は書いた場所で（そのときの `Ctx::cenv` で）解決済み
enum Redecl {
    /// `redeclare [parameter] T x(…) = …`。`(…)` と `= …` は `Mod` の側にある
    Comp {
        snip: Rc<Snip>,
        cc: ast::ComponentClause,
        d: ast::Declaration,
        ctx: Rc<Ctx>,
        res: Option<Resolution>,
        parts: Vec<String>,
        extra: ModMap,
    },
    /// `redeclare package M = X(…)`・`redeclare model M = X`: 名前 `M` をクラス `X`（と `(…)` の修飾）に差し替える
    Class { cid: ClassId, extra: ModMap },
}

/// パッケージの定数の束縛（名前 → 束縛）。作れなかったら `None`
type PkgEnv = Option<Rc<HashMap<String, Bind>>>;

/// 差し替えるクラス（名前 → クラスと `(…)` の修飾）。インスタンスごと
type Cenv = Rc<RefCell<BTreeMap<String, (ClassId, ModMap)>>>;
type ModMap = BTreeMap<String, Mod>;

#[derive(Clone)]
struct Bind {
    node: SyntaxNode,
    file: u32,
    ctx: Rc<Ctx>,
    /// レコードの束縛 `r = other` を欄に分けたときの `.f.g`（印字だけに足す。評価はしない）
    suffix: String,
    /// 評価せず組み立てた式（`Complex` の演算子の展開。S329）。あれば印字はこれ
    lit: Option<String>,
}

/// 式を書いた場所: インスタンスの名前の前置き（`a.b.`）、そのクラスと基底の全部、`for` の添字の値
struct Ctx {
    pfx: String,
    lineage: Rc<Vec<ClassId>>,
    locals: Vec<(String, i64)>,
    /// このインスタンスで `redeclare` された名前（S297）。`for` の中の文脈と共有する
    cenv: Cenv,
    /// このインスタンスのクラスが入っているパッケージの、差し替え後のクラス（`Medium.BaseProperties` を引いたときの `Medium` の中身）。
    /// 受け継いだクラスの中の、同じパッケージのメンバー（`ThermodynamicState`・`nXi`）を、こちらで引き直す
    heir: Option<ClassId>,
    /// 関数の中（`FN`）から見える、呼び出したインスタンスの系統（兄弟の関数の `redeclare function` を引く。S377）。`FN` 以外は `None`（`inst()` を使う）
    inst: Option<Rc<Vec<ClassId>>>,
}

impl Ctx {
    /// 関数の呼び出しがインスタンスの中から来たときの、そのインスタンスの系統。パッケージの文脈・インスタンスの外から来た関数は `None`
    fn inst(&self) -> Option<&Rc<Vec<ClassId>>> {
        match self.pfx.as_str() {
            PKG => None,
            FN => self.inst.as_ref(),
            _ => Some(&self.lineage),
        }
    }

    fn local(&self, n: &str) -> Option<i64> {
        self.locals
            .iter()
            .rev()
            .find(|(k, _)| k == n)
            .map(|(_, v)| *v)
    }
}

/// 差し替えの組（名前 → クラス）の hash。`(…)` の修飾つきの差し替えがあるときは `None`（修飾の中身は鍵にしない）
fn cenv_hash(c: &Cenv) -> Option<u64> {
    use std::hash::{Hash, Hasher};
    let c = c.try_borrow().ok()?;
    let mut h = std::collections::hash_map::DefaultHasher::new();
    for (k, (cid, extra)) in c.iter() {
        if !extra.is_empty() {
            return None;
        }
        (k, cid).hash(&mut h);
    }
    Some(h.finish())
}

/// 文脈の中身の hash（式を評価した結果を覚える鍵。`Rc` の同一性は使わない: パッケージの定数の文脈は評価のたびに作り直す）
fn ctx_hash(ctx: &Ctx) -> Option<u64> {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    (
        &ctx.pfx,
        &*ctx.lineage,
        &ctx.locals,
        ctx.heir,
        cenv_hash(&ctx.cenv)?,
    )
        .hash(&mut h);
    Some(h.finish())
}

/// 節点の中に、名前 `names` のどれかの識別子があるか（宣言の場所で修飾された欄を、型の既定値の式が使うかの判定）
fn mentions(n: &SyntaxNode, names: &[&String]) -> bool {
    n.descendants()
        .any(|x| tokens(&x).any(|t| t.kind() == K::Ident && names.iter().any(|k| *k == t.text())))
}

fn merge_mod(o: &Mod, i: &Mod) -> Mod {
    Mod {
        val: o.val.clone().or_else(|| i.val.clone()),
        sub: merge(&o.sub, &i.sub),
        each: o.each || i.each,
        redecl: o.redecl.clone().or_else(|| i.redecl.clone()),
    }
}

/// 外側（`outer`）が勝つ
fn merge(outer: &ModMap, inner: &ModMap) -> ModMap {
    let mut out = inner.clone();
    for (k, o) in outer {
        let m = match out.get(k) {
            Some(i) => merge_mod(o, i),
            None => o.clone(),
        };
        out.insert(k.clone(), m);
    }
    out
}

#[derive(Clone, Copy, Default)]
struct Flags {
    kind: Option<VarKind>,
    causality: Causality,
    flow: bool,
    stream: bool,
}

/// `a` の値を優先し、無いところを `b` で埋める
fn fill(a: Flags, b: Flags) -> Flags {
    Flags {
        kind: a.kind.or(b.kind),
        causality: if a.causality != Causality::Acausal {
            a.causality
        } else {
            b.causality
        },
        flow: a.flow || b.flow,
        stream: a.stream || b.stream,
    }
}

fn flags_of(toks: impl Iterator<Item = K>) -> Res<Flags> {
    let mut f = Flags::default();
    for k in toks {
        match k {
            K::KwParameter => f.kind = Some(VarKind::Parameter),
            K::KwConstant => f.kind = Some(VarKind::Constant),
            K::KwDiscrete => f.kind = Some(VarKind::Discrete),
            K::KwInput => f.causality = Causality::Input,
            K::KwOutput => f.causality = Causality::Output,
            K::KwFlow => f.flow = true,
            K::KwStream => f.stream = true,
            _ => {}
        }
    }
    Ok(f)
}

/// 親の `Flags` から子へ引き継ぐもの（parameter・constant は中の変数すべてに及ぶ）
fn inherit(f: Flags) -> Flags {
    Flags {
        kind: f
            .kind
            .filter(|k| matches!(k, VarKind::Parameter | VarKind::Constant)),
        ..f
    }
}

#[derive(Clone, Copy, Debug)]
enum Val {
    I(i64),
    R(f64),
    B(bool),
    /// 文字列（`F::strs` の表の番号。同じ中身は同じ番号。S329 は hash、S377 で表にした）
    S(u32),
    /// 列挙の要素（列挙のクラスと、その中の順番）
    E(ClassId, usize),
}

impl Val {
    fn f(self) -> f64 {
        match self {
            Val::I(i) => i as f64,
            Val::R(r) => r,
            Val::B(b) => f64::from(u8::from(b)),
            Val::S(_) => f64::NAN,
            Val::E(_, i) => i as f64,
        }
    }
    fn int(self) -> Option<i64> {
        match self {
            Val::I(i) => Some(i),
            Val::R(r) if r.fract() == 0.0 && r.abs() < 1e15 => Some(r as i64),
            _ => None,
        }
    }
}

/// 文字列の表（S377）。`Val` は `Copy` なので `Val::S` は番号を持ち、辞書順・連結は中身を引く。表は伸びるだけ（番号は変わらない）
#[derive(Default)]
struct StrTab {
    v: Vec<Rc<str>>,
    ix: HashMap<Rc<str>, u32>,
}

#[derive(Default)]
struct Strs(RefCell<StrTab>);

/// 連結で作る文字列の長さの上限（`c_i = c_{i-1} + c_{i-1}` の連鎖が 2^n バイトを作らない）
const MAX_STR: usize = 1 << 16;

impl Strs {
    fn intern(&self, s: &str) -> Val {
        let t = &mut *self.0.borrow_mut();
        if let Some(&i) = t.ix.get(s) {
            return Val::S(i);
        }
        let (r, i): (Rc<str>, _) = (s.into(), t.v.len() as u32);
        t.v.push(r.clone());
        t.ix.insert(r, i);
        Val::S(i)
    }

    fn get(&self, i: u32) -> Rc<str> {
        self.0.borrow().v[i as usize].clone()
    }

    /// 辞書順（UTF-8 のバイト順 = 符号位置の順。omc の `checkModel` と同じ）
    fn cmp(&self, a: u32, b: u32) -> std::cmp::Ordering {
        self.get(a).cmp(&self.get(b))
    }

    fn cat(&self, a: u32, b: u32) -> Option<Val> {
        let (x, y) = (self.get(a), self.get(b));
        (x.len() + y.len() <= MAX_STR).then(|| self.intern(&format!("{x}{y}")))
    }
}

/// 文字列のリテラル（引用符つき）の中身。エスケープ `\n` などを解く（MLS の S-ESCAPE。知らない `\x` はそのまま）
fn unquote(lit: &str) -> String {
    let mut out = String::with_capacity(lit.len());
    let mut it = lit.strip_prefix('"').unwrap_or(lit).chars();
    while let Some(c) = it.next() {
        if c == '"' && it.as_str().is_empty() {
            break;
        }
        if c != '\\' {
            out.push(c);
            continue;
        }
        match it.next() {
            Some('n') => out.push('\n'),
            Some('t') => out.push('\t'),
            Some('r') => out.push('\r'),
            Some('a') => out.push('\x07'),
            Some('b') => out.push('\x08'),
            Some('f') => out.push('\x0c'),
            Some('v') => out.push('\x0b'),
            Some(e @ ('\\' | '"' | '\'' | '?')) => out.push(e),
            Some(e) => {
                out.push('\\');
                out.push(e);
            }
            None => out.push('\\'),
        }
    }
    out
}

enum Item {
    Decl {
        cc: ast::ComponentClause,
        file: u32,
        mods: Rc<ModMap>,
        /// `protected` でない（コネクタなら、親から見える。`protected` のコネクタは外側の flow = 0 の対象にしない）
        public: bool,
    },
    Sec {
        node: SyntaxNode,
        file: u32,
    },
}

enum Ty {
    Prim {
        ty: String,
        mods: ModMap,
        flags: Flags,
        conn: bool,
    },
    Comp {
        cid: ClassId,
        mods: ModMap,
        flags: Flags,
        conn: bool,
    },
}

#[derive(Default)]
struct Level {
    conns: Vec<(String, String, Option<Origin>)>,
}

const MAX_VARS: usize = 200_000;
/// 方程式（`when`・`initial`・`connect` の端を含む）の数の上限。入れ子の `for` の積は範囲の上限（10 万）では抑えられない（S339）
#[cfg(not(test))]
const MAX_EQS: usize = 1_000_000;
/// 試験では小さくする（debug のビルドは 1 本 50 µs。下の `nested_for_equation_count_is_capped`）
#[cfg(test)]
const MAX_EQS: usize = 10_000;
/// `for` の方程式の反復の数の上限。方程式を作らない入れ子の `for`（本体が空・偽の `if` だけ）は `MAX_EQS` に数えられない（S365・#136）
#[cfg(not(test))]
const MAX_ITERS: usize = 1_000_000;
#[cfg(test)]
const MAX_ITERS: usize = 10_000;
/// スカラーの総数（変数・方程式それぞれ）と、変数 1 つの次元の積の上限。`MAX_VARS`・`MAX_EQS` は配列を 1 つと数えるので、
/// `Real x[10000000]` を通し、DAE の構築が 1 スカラー約 300 バイトを使う（S365・#139）。次元の積は溢れる大きさも書ける（#137）
const MAX_SCALARS: usize = 2_000_000;

/// 修飾の表の中の特別な名前: レコード全体の束縛 `r = other`（`other` の欄を、型の欄ごとに束縛する。宣言が後ろにある兄弟のレコードや、レコードを返す関数の呼び出し）
const WHOLE: &str = "*";

struct F<'a> {
    p: &'a Project,
    vars: Vec<Var>,
    var_idx: BTreeMap<String, usize>,
    eqs: Vec<Equation>,
    whens: Vec<Equation>,
    ieqs: Vec<Equation>,
    algs: Vec<String>,
    /// 今平らにしている方程式の位置（`push` が付ける）
    cur: Option<Origin>,
    env: HashMap<String, Bind>,
    /// `inner` で宣言した部品の完全な名前
    inners: HashSet<String>,
    /// `outer` の部品 → 結びつけた `inner` の完全な名前
    alias: HashMap<String, String>,
    /// `inner outer` の部品（実体を持つ）→ 外側の `inner` の完全な名前。同じ階層の `connect` の端だけ、外側の `inner` に付け替える（S361）
    bridge: HashMap<String, String>,
    /// 条件が偽で落とした部品の完全な名前（これにつながる `connect` も落とす）
    removed: Vec<String>,
    /// まだ平らにしていない宣言（`size(x, 1)` が後ろで宣言した配列の大きさを引くため）
    pending: HashMap<String, Pending>,
    depth: std::cell::Cell<u32>,
    /// `inst_class` の入れ子の深さ（自分を部品に持つモデルでスタックを溢れさせないための上限。S320・#111）
    inst_depth: std::cell::Cell<u32>,
    /// パッケージ（差し替え先を含む）の定数の束縛。`extends` の修飾で決まる値も入る（S297）
    pkg_envs: RefCell<HashMap<ClassId, PkgEnv>>,
    /// パッケージの `extends P(redeclare record R = X)` が決めた差し替え（`pkg_env` と一緒に作る）
    pkg_cenvs: RefCell<HashMap<ClassId, Cenv>>,
    /// `equalityConstraint` を持つレコード（過剰決定コネクタ）の部品の完全な名前 → （その関数の完全修飾名、戻り値の大きさ）（S312・S334）
    oc_nodes: BTreeMap<String, (String, usize)>,
    /// 過剰決定コネクタの接続集合（`graph::OcSet`）と、すべての `connect` の端
    oc_sets: Vec<graph::OcSet>,
    conn_pairs: Vec<(String, String)>,
    /// 外側の階層で宣言した `inner` につなぐ `connect`（`outer` 経由）: （その `inner` を宣言する階層の前置き, 端 a, 端 b, 位置, 持ち越す前の階層で各端が外側のコネクタか）。
    /// 接続集合は階層をまたいで 1 つ（`outer` を持つ部品の全部がその `inner` につながる。`inner` が後ろで宣言されていて葉がまだ無いこともある）なので、
    /// 宣言する階層の `gen_connects` で集合に入れる（S361）
    late_conns: Vec<LateConn>,
    /// stream 変数の接続集合（MLS 15.2）。端は（葉の名前, 外側のコネクタの端か）。`expand_streams` が `inStream` などを展開する（S355）
    stream_sets: Vec<Vec<(String, bool)>>,
    /// 条件に `Connections.isRoot`・`rooted` を含む `if` の方程式（根の決定のあとに枝を選ぶ。S333）
    oc_ifs: Vec<graph::OcIf>,
    /// 条件に `cardinality` を含む `if` の方程式（全部の `connect` がそろってから枝を選ぶ。S361）
    card_ifs: Vec<CardIf>,
    /// `for` の方程式の反復の数（`MAX_ITERS` で断る。S365）
    iters: usize,
    /// 関数の評価器（`fneval`。S370・S374）: この flat で数えた仕事量（呼び出しごとに戻さない）、呼び出しの入れ子の深さ、
    /// 呼び出しの結果（関数・`heir`・差し替えの組・スカラーの引数 → 値。失敗も覚える）
    fn_steps: std::cell::Cell<usize>,
    fn_depth: std::cell::Cell<u32>,
    fn_memo: RefCell<HashMap<fneval::MemoKey, Option<fneval::V>>>,
    /// 名前の定数・parameter の束縛の式の値（S376・#158）。鍵は式の場所と、評価する文脈の中身（`bind_key`）。成功した値だけ
    eval_memo: RefCell<HashMap<(u32, u32, u32, u64), Val>>,
    /// 文字列の表（`Val::S` の番号。S377）
    strs: Strs,
    /// `cardinality(c)` の値（`connect` の端として出てきた数）。`card_ifs` を決めるときだけ `Some`
    card: Option<HashMap<String, i64>>,
    /// expandable connector の中を平らにしている入れ子の深さ（0 より大きいあいだに宣言した変数は `exp_vars` に入る。S367）
    exp_depth: u32,
    /// expandable connector の宣言済みの欄（束縛が無いもの）の変数の番号
    exp_vars: HashSet<usize>,
    /// 方程式を作った接続集合の端の変数の番号（`exp_vars` のうちここに入らないものは `unused_expandable`）
    conn_vars: HashSet<usize>,
    /// 端がすべて `exp_vars` の potential の接続集合（階層ごと）。全体の集合の端がどれも読まれなければ落とす（Rumoca と同じ。`settle_expandable`）
    exp_sets: Vec<ExpSet>,
    /// `exp_vars` の変数を端に含む potential の接続集合（階層ごと）の端の変数の番号
    pot_sets: Vec<Vec<usize>>,
}

/// 端がすべて expandable connector の宣言済みの欄の接続集合（その方程式は、使われると分かるまで `eqs` に入れない）
struct ExpSet {
    vars: Vec<usize>,
    eqs: Vec<Equation>,
}

type LateConn = (String, String, String, Option<Origin>, (bool, bool));

/// 置き場所は方程式の列の中の `marker` の 1 行
struct CardIf {
    marker: String,
    file: u32,
    ctx: Rc<Ctx>,
    branches: Vec<(Option<SyntaxNode>, Vec<Equation>)>,
}

struct Pending {
    cc: ast::ComponentClause,
    d: ast::Declaration,
    file: u32,
    ctx: Rc<Ctx>,
    m: Mod,
}

/// `inst_class` の入れ子の上限（MSL の部品の入れ子は 30 段に届かない）
const MAX_INST_DEPTH: u32 = 64;

/// 「パッケージの定数を評価する文脈」の印（`Ctx::pfx`）
const PKG: &str = "\u{0}";

/// 「関数の呼び出しの出力の大きさを評価する文脈」の印（`Ctx::pfx`。`locals` に入力の整数の値を束ねる）
const FN: &str = "\u{1}";

pub(crate) fn run(p: &Project, cid: ClassId, model: &str) -> Res<FlatModel> {
    let mut f = F {
        p,
        vars: Vec::new(),
        var_idx: BTreeMap::new(),
        eqs: Vec::new(),
        whens: Vec::new(),
        ieqs: Vec::new(),
        algs: Vec::new(),
        cur: None,
        env: HashMap::new(),
        inners: HashSet::new(),
        alias: HashMap::new(),
        bridge: HashMap::new(),
        removed: Vec::new(),
        pending: HashMap::new(),
        depth: std::cell::Cell::new(0),
        inst_depth: std::cell::Cell::new(0),
        pkg_envs: RefCell::default(),
        pkg_cenvs: RefCell::default(),
        oc_nodes: BTreeMap::new(),
        oc_sets: Vec::new(),
        conn_pairs: Vec::new(),
        late_conns: Vec::new(),
        stream_sets: Vec::new(),
        oc_ifs: Vec::new(),
        card_ifs: Vec::new(),
        iters: 0,
        fn_steps: std::cell::Cell::new(0),
        fn_depth: std::cell::Cell::new(0),
        fn_memo: RefCell::default(),
        eval_memo: RefCell::default(),
        strs: Strs::default(),
        card: None,
        exp_depth: 0,
        exp_vars: HashSet::new(),
        conn_vars: HashSet::new(),
        exp_sets: Vec::new(),
        pot_sets: Vec::new(),
    };
    // パッケージの中のモデルを直接平らにするときは、そのパッケージが差し替え後のパッケージ（`TwoPhaseWater.BaseProperties`）
    let heir = p.lib.classes[cid]
        .parent
        .filter(|q| p.kind_of(*q) == Some(K::KwPackage));
    let (own, _) = f.inst_class(cid, "", &ModMap::new(), Flags::default(), heir)?;
    // 最上位のモデル自身の公開のコネクタの flow は 0（外へ流れ出ない。Rumoca の flat も同じ。`protected` は gen_connects）
    for n in own {
        for (suffix, vi, count) in f.leaves(&n) {
            if f.vars[vi].flow {
                f.eqs.push(Equation {
                    text: format!("{n}{suffix} = 0"),
                    count,
                    origin: f.vars[vi].origin.clone(),
                });
            }
        }
    }
    if let Some((_, a, b, ..)) = f.late_conns.first() {
        return un(
            "connect",
            format!("{a} and {b}: inner is never instantiated"),
        );
    }
    f.apply_connection_graph();
    f.resolve_card_ifs()?;
    f.expand_streams()?;
    f.settle_expandable();
    // 配列は 1 つと数える `MAX_VARS`・`MAX_EQS` では抑えられない、スカラーの総数（S365・#139）
    let sat = usize::saturating_add;
    let vs = (f.vars.iter()).fold(0, |a, v| {
        sat(a, dims_product(&v.dims).unwrap_or(usize::MAX))
    });
    let es = (f.eqs.iter().chain(&f.whens).chain(&f.ieqs)).fold(0, |a, e| sat(a, e.count));
    if vs > MAX_SCALARS || es > MAX_SCALARS {
        return un("too-large", "too many scalar variables or equations");
    }
    Ok(FlatModel {
        name: model.to_string(),
        vars: f.vars,
        equations: f.eqs,
        when_equations: f.whens,
        initial_equations: f.ieqs,
        algorithms: f.algs,
    })
}

fn tokens(n: &SyntaxNode) -> impl Iterator<Item = globe_mode_syntax::SyntaxToken> {
    n.children_with_tokens()
        .filter_map(|e| e.into_token())
        .filter(|t| !t.kind().is_trivia())
}

fn idx_names(dims: &[usize]) -> Vec<String> {
    let mut out = vec![String::new()];
    for &d in dims {
        out = out
            .iter()
            .flat_map(|p| {
                (1..=d).map(move |i| {
                    if p.is_empty() {
                        format!("[{i}")
                    } else {
                        format!("{p},{i}")
                    }
                })
            })
            .collect();
    }
    if dims.is_empty() {
        return out;
    }
    out.iter().map(|s| format!("{s}]")).collect()
}

impl F<'_> {
    // ---------------------------------------------------------------- クラスのインスタンス化

    /// 戻り値は、このインスタンスが直接持つコネクタの名前（親が「内側のコネクタ」として使う公開のものと、`protected` を含む全部）
    fn inst_class(
        &mut self,
        cid: ClassId,
        pfx: &str,
        mods: &ModMap,
        flags: Flags,
        heir: Option<ClassId>,
    ) -> Res<(Vec<String>, Vec<String>)> {
        // 上限は Rumoca の GM-E3001（循環）に合わせて断る。debug でも溢れない段数（#111）
        if self.inst_depth.get() >= MAX_INST_DEPTH {
            return un("deep-instantiation", self.p.lib.qualified_name(cid));
        }
        self.inst_depth.set(self.inst_depth.get() + 1);
        let exp = self.p.class(cid).is_some_and(|cd| {
            tokens(cd.syntax())
                .map(|t| t.kind())
                .take_while(|k| *k != K::Ident)
                .any(|k| k == K::KwExpandable)
        });
        self.exp_depth += u32::from(exp);
        let r = self.inst_class_body(cid, pfx, mods, flags, heir);
        self.exp_depth -= u32::from(exp);
        self.inst_depth.set(self.inst_depth.get() - 1);
        r
    }

    fn inst_class_body(
        &mut self,
        cid: ClassId,
        pfx: &str,
        mods: &ModMap,
        flags: Flags,
        heir: Option<ClassId>,
    ) -> Res<(Vec<String>, Vec<String>)> {
        let Some(cd) = self.p.class(cid).cloned() else {
            return un("unresolved", "class without syntax");
        };
        if matches!(
            cd.kind_token().map(|t| t.kind()),
            Some(K::KwFunction | K::KwPackage)
        ) {
            return un("not-a-model", self.p.lib.qualified_name(cid));
        }
        let ctx = Rc::new(Ctx {
            pfx: pfx.to_string(),
            lineage: self.p.lineage(cid),
            locals: Vec::new(),
            cenv: Cenv::default(),
            heir,
            inst: None,
        });
        self.add_cenv(&ctx, mods);
        let mut items = Vec::new();
        self.collect(cid, mods, &ctx, &mut items, 0)?;
        // 束縛を先に登録する（大きさ・添字が同じインスタンスの parameter を使うので）
        let mut decls = Vec::new();
        for it in &items {
            let Item::Decl {
                cc,
                file,
                mods,
                public,
            } = it
            else {
                continue;
            };
            for d in cc.declarations() {
                let Some((name, _)) = d.name() else { continue };
                let own = self.mod_of(
                    d.syntax().children().find(|c| c.kind() == K::Modification),
                    *file,
                    &ctx,
                    false,
                )?;
                let mut m = match mods.get(&name) {
                    Some(o) => merge_mod(o, &own),
                    None => own,
                };
                // レコードの束縛 `r = other` の欄（S324）: 外側の明示した修飾が無ければ、欄 `f` は `other.f` に束縛される
                if let Some(w) = mods.get(WHOLE).and_then(|w| w.val.as_ref())
                    && mods.get(&name).is_none_or(|o| o.val.is_none())
                {
                    m.val = Some(Bind {
                        suffix: format!("{}.{name}", w.suffix),
                        ..w.clone()
                    });
                }
                if let Some(b) = &m.val {
                    self.env.insert(format!("{pfx}{name}"), b.clone());
                }
                // inner は先に登録し、outer は外側の同じ名前の inner を探す（別の部品の束縛・方程式でも使うので先に）
                let toks: Vec<K> = tokens(cc.syntax())
                    .map(|t| t.kind())
                    .take_while(|k| !matches!(k, K::Ident | K::Dot))
                    .collect();
                let full = format!("{pfx}{name}");
                let found = toks
                    .contains(&K::KwOuter)
                    .then(|| self.find_inner(pfx, &name))
                    .flatten();
                match found {
                    // `inner outer` は実体を持ち（中の部品の `outer` はこちらにつながる）、同じ階層の `connect` の端だけ外側の `inner` につながる（MLS 5.4）
                    Some(t) if toks.contains(&K::KwInner) => {
                        self.bridge.insert(full.clone(), t);
                    }
                    Some(t) => {
                        self.alias.insert(full.clone(), t);
                    }
                    None if toks.contains(&K::KwOuter) && !toks.contains(&K::KwInner) => {
                        return un("inner-outer", format!("outer {full} has no inner"));
                    }
                    None => {}
                }
                if toks.contains(&K::KwInner) {
                    self.inners.insert(full);
                }
                self.pending.insert(
                    format!("{pfx}{name}"),
                    Pending {
                        cc: cc.clone(),
                        d: d.clone(),
                        file: *file,
                        ctx: ctx.clone(),
                        m: m.clone(),
                    },
                );
                decls.push((cc.clone(), *file, d, name, m, *public));
            }
        }
        let mut own_conns = Vec::new();
        let mut own_all = Vec::new();
        let mut inside = Vec::new();
        for (cc, file, d, name, m, public) in decls {
            // `protected` のコネクタは親から見えない（外側の flow = 0 の対象にしない）
            let mut own_here = Vec::new();
            self.inst_decl(
                &ctx,
                file,
                &cc,
                &d,
                &name,
                m,
                inherit(flags),
                &mut own_here,
                &mut inside,
            )?;
            if public {
                own_conns.extend(own_here.iter().cloned());
            }
            own_all.extend(own_here);
        }
        let mut lv = Level::default();
        for it in &items {
            if let Item::Sec { node, file } = it {
                self.section(node, *file, &ctx, &mut lv)?;
            }
        }
        let prot: Vec<String> = own_all
            .iter()
            .filter(|n| !own_conns.contains(n))
            .cloned()
            .collect();
        self.gen_connects(&ctx.pfx, &lv, &inside, &own_all, &prot)?;
        Ok((own_conns, own_all))
    }

    /// `pfx` のインスタンスから見て、外側へたどって最初の同じ名前の `inner`（`pfx` 自身は見ない）
    fn find_inner(&self, pfx: &str, name: &str) -> Option<String> {
        let mut cur = pfx.to_string();
        loop {
            let t = cur.strip_suffix('.')?;
            cur = t.rfind('.').map_or(String::new(), |i| t[..=i].to_string());
            let cand = format!("{cur}{name}");
            if self.inners.contains(&cand) {
                return Some(self.alias.get(&cand).cloned().unwrap_or(cand));
            }
        }
    }

    /// `extends` を取り込みながら、宣言と節を宣言の順に集める
    fn collect(
        &self,
        cid: ClassId,
        mods: &ModMap,
        ctx: &Rc<Ctx>,
        items: &mut Vec<Item>,
        depth: u32,
    ) -> Res<()> {
        if depth > 60 {
            return un("deep-extends", self.p.lib.qualified_name(cid));
        }
        let Some(cd) = self.p.class(cid).cloned() else {
            return un("unresolved", "class without syntax");
        };
        let file = self.p.lib.classes[cid].file;
        if let Some(sb) = cd.short_body() {
            // short class を部品の型でなく基底として使う（`model A = B(k = 1)`）
            let Some(base) = sb.base() else {
                return un("short-class", "no base");
            };
            let m2 = match sb
                .syntax()
                .children()
                .find(|c| c.kind() == K::ClassModification)
            {
                Some(cm) => merge(mods, &self.mods_of(&cm, file, ctx)?),
                None => mods.clone(),
            };
            self.add_cenv(ctx, &m2);
            return match self.class_res(file, &base, ctx).0 {
                Some(Resolution::Class(b)) => self.collect(b, &m2, ctx, items, depth + 1),
                _ => un("extends-builtin", base.parts().join(".")),
            };
        }
        if cd.is_class_extends() {
            // `redeclare model extends M(…)`: 受け継いだ `M` を先に取り込み、`(…)` の修飾を当てる
            let Some(b) = self.p.class_ext_base(cid) else {
                return un("class-extends", self.p.lib.qualified_name(cid));
            };
            let m2 = match cd
                .syntax()
                .children()
                .find(|c| c.kind() == K::ClassModification)
            {
                Some(cm) => merge(mods, &self.mods_of(&cm, file, ctx)?),
                None => mods.clone(),
            };
            self.add_cenv(ctx, &m2);
            self.collect(b, &m2, ctx, items, depth + 1)?;
        }
        let shared = Rc::new(mods.clone());
        let mut public = true;
        for e in cd.elements() {
            match e {
                Element::Visibility(p) => public = p,
                Element::Extends(x) => {
                    let Some(n) = x.name() else { continue };
                    let m2 = match x
                        .syntax()
                        .children()
                        .find(|c| c.kind() == K::ClassModification)
                    {
                        Some(cm) => merge(mods, &self.mods_of(&cm, file, ctx)?),
                        None => mods.clone(),
                    };
                    self.add_cenv(ctx, &m2);
                    match self.class_res(file, &n, ctx).0 {
                        Some(Resolution::Class(b)) => {
                            self.collect(b, &m2, ctx, items, depth + 1)?
                        }
                        Some(Resolution::Unresolved(Reason::Redeclare)) => {
                            return un("redeclare", n.parts().join("."));
                        }
                        Some(Resolution::Builtin) => {
                            return un("extends-builtin", n.parts().join("."));
                        }
                        _ => return un("unresolved", format!("extends {}", n.parts().join("."))),
                    }
                }
                Element::Component(cc) => items.push(Item::Decl {
                    cc,
                    file,
                    mods: shared.clone(),
                    public,
                }),
                _ => {}
            }
        }
        for ch in cd.syntax().children() {
            match ch.kind() {
                K::EquationSection | K::AlgorithmSection => {
                    items.push(Item::Sec { node: ch, file })
                }
                K::ExternalClause => return un("external", self.p.lib.qualified_name(cid)),
                _ => {}
            }
        }
        Ok(())
    }

    // ---------------------------------------------------------------- 宣言

    #[allow(clippy::too_many_arguments)]
    fn inst_decl(
        &mut self,
        ctx: &Rc<Ctx>,
        file: u32,
        cc: &ast::ComponentClause,
        d: &ast::Declaration,
        name: &str,
        m: Mod,
        parent: Flags,
        own: &mut Vec<String>,
        inside: &mut Vec<String>,
    ) -> Res<()> {
        if self.vars.len() > MAX_VARS {
            return un("too-large", "too many variables");
        }
        // `redeclare` された部品は、差し替えた宣言の型・接頭辞・添字を使う（修飾 `m` は外側が勝って合わさっている）
        let redecl = match m.redecl.as_deref() {
            Some(Redecl::Comp {
                snip,
                cc,
                d,
                ctx,
                res,
                parts,
                extra,
            }) => Some((
                snip.fid,
                cc.clone(),
                d.clone(),
                ctx.clone(),
                res.clone(),
                parts.clone(),
                extra.clone(),
            )),
            _ => None,
        };
        // 差し替えられた部品には、元の宣言の `constrainedby C(x = …)` の修飾も掛かる（MLS 7.3.2。差し替えた側の明示した修飾が勝つ。S361）
        let mut m = m;
        if redecl.is_some()
            && let Some(cm) = cc
                .syntax()
                .children()
                .find(|c| c.kind() == K::ClassModification)
        {
            m.sub = merge(&m.sub, &self.mods_of(&cm, file, ctx)?);
        }
        let own_flags = flags_of(
            tokens(redecl.as_ref().map_or(cc, |r| &r.1).syntax())
                .map(|t| t.kind())
                .take_while(|k| !matches!(k, K::Ident | K::Dot)),
        )?;
        if self.alias.contains_key(&format!("{}{name}", ctx.pfx)) {
            return Ok(()); // outer: 部品は作らず、参照を inner の名前に付け替える
        }
        if let Some(c) = d.syntax().children().find(|c| c.kind() == K::Condition) {
            // 条件が定数に評価できるときだけ。偽なら宣言を落とす
            let v = c
                .children()
                .find_map(Expr::cast)
                .and_then(|e| self.eval(e.syntax(), file, ctx, 0));
            match v {
                Some(Val::B(true)) => {}
                Some(Val::B(false)) => {
                    self.removed.push(format!("{}{name}", ctx.pfx));
                    return Ok(());
                }
                _ => return un("conditional-component", format!("{name} if {}", c.text())),
            }
        }
        let mut heir = ctx.heir;
        let (tres, tparts, textra) = match &redecl {
            Some(r) => (r.4.clone(), r.5.clone(), r.6.clone()),
            None => {
                let Some(tn) = cc.type_name() else {
                    return un("unresolved", "no type name");
                };
                let (res, extra, h) = self.class_res(file, &tn, ctx);
                heir = h.or_else(|| self.alias_heir(file, &tn, &res)).or(ctx.heir);
                (res, tn.parts(), extra)
            }
        };
        let (mut ty, tdims) = self.type_of(tres, &tparts)?;
        if let Ty::Comp { mods, .. } = &mut ty {
            *mods = merge(mods, &textra);
        }
        let dims = match &redecl {
            Some(r)
                if r.2
                    .syntax()
                    .children()
                    .chain(r.1.syntax().children())
                    .any(|c| c.kind() == K::Subscripts) =>
            {
                self.decl_dims(&r.3, r.0, &r.1, &r.2, name, &m)?
            }
            _ => self.decl_dims(ctx, file, cc, d, name, &m)?,
        };
        let dims = [dims, tdims].concat();
        let full = format!("{}{}", ctx.pfx, name);
        match ty {
            Ty::Prim {
                ty,
                mods,
                flags,
                conn,
            } => {
                let eff = fill(parent, fill(own_flags, flags));
                let mods = merge(&m.sub, &mods);
                let mut attrs = Vec::new();
                let mut start = None;
                for (k, am) in &mods {
                    if let Some(b) = &am.val {
                        let t = self.print_bind(b)?;
                        if k == "start" {
                            start = Some(t);
                        } else {
                            attrs.push((k.clone(), t));
                        }
                    }
                }
                let binding = m.val.as_ref().map(|b| self.print_bind(b)).transpose()?;
                if ty == "StateSelect" {
                    return Ok(()); // 組み込みの列挙。Rumoca の flat も変数にしない
                }
                let kind = eff.kind.unwrap_or(if ty == "Real" {
                    VarKind::Continuous
                } else {
                    VarKind::Discrete
                });
                if self.var_idx.contains_key(&full) {
                    return Ok(()); // 同じ要素が 2 つの `extends` から届いた
                }
                self.check_dims(&full, &dims)?;
                self.var_idx.insert(full.clone(), self.vars.len());
                if self.exp_depth > 0 && binding.is_none() {
                    self.exp_vars.insert(self.vars.len());
                }
                self.vars.push(Var {
                    name: full.clone(),
                    kind,
                    causality: eff.causality,
                    flow: eff.flow,
                    stream: eff.stream,
                    ty,
                    dims: dims.clone(),
                    start,
                    binding,
                    attrs,
                    origin: self.p.origin(file, d.syntax()),
                    unused_expandable: false,
                });
                if conn {
                    own.push(full);
                }
            }
            Ty::Comp {
                cid,
                mods,
                flags,
                conn,
            } => {
                // 外部オブジェクト（`extends ExternalObject`）は評価せず、不透明な 1 つの変数として残す（S324）
                if self.is_ext_obj(cid) {
                    if self.var_idx.contains_key(&full) {
                        return Ok(());
                    }
                    let binding = m.val.as_ref().map(|b| self.print_bind(b)).transpose()?;
                    let eff = fill(parent, fill(own_flags, flags));
                    self.check_dims(&full, &dims)?;
                    self.var_idx.insert(full.clone(), self.vars.len());
                    self.vars.push(Var {
                        name: full,
                        kind: eff.kind.unwrap_or(VarKind::Parameter),
                        causality: eff.causality,
                        flow: false,
                        stream: false,
                        ty: self.p.lib.qualified_name(cid),
                        dims,
                        start: None,
                        binding,
                        attrs: Vec::new(),
                        origin: self.p.origin(file, d.syntax()),
                        unused_expandable: false,
                    });
                    return Ok(());
                }
                // `R r = R(a = 1)`・`r = other` は欄ごとの修飾にする。明示した修飾 > 束縛 > 型の修飾
                // 配列のレコードは要素ごとに: `{R(…), R(…)}`・`{… for k in 1:n}` は要素の式、別の配列 `x` は `x[k]`（`[i,j]` も。`pick_elem`）
                // 添字の文字列を作る前に個数を見る（積の overflow も。#109）
                let n = dims.iter().try_fold(1usize, |a, &d| a.checked_mul(d));
                if n.is_none_or(|n| n > MAX_VARS - self.vars.len().min(MAX_VARS)) {
                    return un("too-large", format!("{name}[] too many elements"));
                }
                let bound: Vec<ModMap> = match &m.val {
                    Some(b) if dims.is_empty() => vec![self.record_value(b, name, cid)?],
                    Some(b) => {
                        if let Some(Expr::Array(a)) = Expr::cast(b.node.clone())
                            && b.suffix.is_empty()
                            && !b.node.children().any(|c| c.kind() == K::Comprehension)
                            && a.items().len() != dims[0]
                        {
                            return un("composite-binding", format!("{name}[] size differs"));
                        }
                        // 別のレコードの欄の配列 `rcData[n] = cellData.rcData`（`r = other` の欄。suffix は `.rcData`）は、要素ごとに `.rcData[k]`（S367）
                        idx_names(&dims)
                            .into_iter()
                            .map(|ix| {
                                let el = Bind {
                                    suffix: format!("{}{ix}", b.suffix),
                                    ..b.clone()
                                };
                                self.record_value(&el, name, cid)
                            })
                            .collect::<Res<_>>()?
                    }
                    None => vec![ModMap::new()],
                };
                let eff = fill(parent, fill(own_flags, flags));
                let split = !dims.is_empty() && self.has_array_value(&m.sub);
                let oc = self.p.lineage(cid).iter().find_map(|c| {
                    let f = *self.p.lib.classes[*c].classes.get("equalityConstraint")?;
                    // 大きさが決められなければ 0（今までと同じ。等式を作らない）
                    let n = self
                        .class_out_dims(f, ctx, None)
                        .and_then(|d| dims_product(&d))
                        .unwrap_or(0);
                    Some((self.p.lib.qualified_name(f), n))
                });
                let mut items = HashMap::new();
                for (e, ix) in idx_names(&dims).into_iter().enumerate() {
                    let el = format!("{full}{ix}");
                    if let Some(o) = &oc {
                        self.oc_nodes.insert(el.clone(), o.clone());
                    }
                    let sub_e = if split {
                        self.split_mod(
                            &m.sub,
                            &ix,
                            (dims.len() == 1).then_some((e, dims[0])),
                            dims.len(),
                            &mut items,
                        )?
                    } else {
                        m.sub.clone()
                    };
                    let mods = merge(&sub_e, &merge(&bound[e.min(bound.len() - 1)], &mods));
                    let (sub, _) =
                        self.inst_class(cid, &format!("{el}."), &mods, inherit(eff), heir)?;
                    if conn {
                        own.push(el);
                    } else {
                        inside.extend(sub);
                    }
                }
            }
        }
        Ok(())
    }

    fn has_array_value(&self, m: &ModMap) -> bool {
        m.values().any(|x| {
            (!x.each
                && x.val
                    .as_ref()
                    .is_some_and(|b| self.split_kind(b, 0).is_some()))
                || self.has_array_value(&x.sub)
        })
    }

    /// 分けられる値の形（`split_kind`）。それに加えて、次元が `nd` 個以上の配列の変数・レコードの配列への参照
    /// （`cellData = stackData.cellData`）は、要素ごとに添字 `[ix]` を付ける（`Some(None)`。S367）
    fn split_kind(&self, b: &Bind, nd: usize) -> Option<Option<SyntaxNode>> {
        if let Some(k) = split_kind(&b.node, nd) {
            return Some(k);
        }
        let Some(Expr::ComponentRef(r)) = Expr::cast(b.node.clone()) else {
            return None;
        };
        if !b.suffix.is_empty()
            || b.lit.is_some()
            || b.node.children().any(|x| x.kind() == K::Subscripts)
        {
            return None;
        }
        let t = self.cref(&r, b.file, &b.ctx).ok()?;
        (self.var_dims(&t)?.len() >= nd.max(1)).then_some(None)
    }

    /// 部品の配列への修飾 `c[n](a = {…})` を要素 `ix`（`[2]`・`[1,2]`）の分に分ける。`each` の修飾は分けない（S329）。
    /// 1 次元で `{…}` の項が並んでいれば項そのもの、それ以外（`for` つきの内包・多次元）は `({…})[ix]`
    /// `one` は 1 次元のときの（要素の番号 0 始まり, 大きさ）
    /// `items` は `{…}` の項（配列の式ごとに 1 回だけ集める。要素ごとに作り直すと O(n²)。S339）
    fn split_mod(
        &self,
        m: &ModMap,
        ix: &str,
        one: Option<(usize, usize)>,
        nd: usize,
        items: &mut HashMap<SyntaxNode, Option<Vec<SyntaxNode>>>,
    ) -> Res<ModMap> {
        let mut out = ModMap::new();
        for (k, x) in m {
            let mut x = x.clone();
            if !x.each {
                if let Some(b) = x.val.as_mut()
                    && let Some(kind) = self.split_kind(b, nd)
                {
                    // `fill(x, n…)` は要素ごとに `x`
                    if let Some(v) = kind {
                        b.node = v;
                        out.insert(k.clone(), x);
                        continue;
                    }
                    // `for` つきの内包は項に分けない（`None`）
                    let its = items.entry(b.node.clone()).or_insert_with(|| {
                        let plain = !b.node.children().any(|c| c.kind() == K::Comprehension);
                        Expr::cast(b.node.clone())
                            .filter(|_| plain)
                            .and_then(|e| match e {
                                Expr::Array(a) => {
                                    Some(a.items().iter().map(|x| x.syntax().clone()).collect())
                                }
                                _ => None,
                            })
                    });
                    match (one, its) {
                        (Some((e, n)), Some(i)) => {
                            if i.len() != n {
                                return un("array-modification", format!("{k} size differs"));
                            }
                            b.node = i[e].clone();
                        }
                        _ => b.suffix = format!("{}{ix}", b.suffix),
                    }
                }
                x.sub = self.split_mod(&x.sub, ix, one, nd, items)?;
            }
            out.insert(k.clone(), x);
        }
        Ok(out)
    }

    /// 変数 1 つのスカラーの数（次元の積）が上限を超えていないか。以降の変数ごとの `dims.iter().product()`（flat・dae）は溢れない（S365・#137・#139）
    fn check_dims(&self, name: &str, dims: &[usize]) -> Res<()> {
        match dims_product(dims) {
            Some(n) if n <= MAX_SCALARS => Ok(()),
            _ => un("too-large", format!("{name}[] too many elements")),
        }
    }

    /// `extends ExternalObject` を持つクラスか
    fn is_ext_obj(&self, cid: ClassId) -> bool {
        self.p.lineage(cid).iter().any(|c| {
            self.p.class(*c).is_some_and(|cd| {
                cd.elements().into_iter().any(|e| {
                    matches!(e, Element::Extends(x)
                        if x.name().is_some_and(|n| n.parts() == ["ExternalObject"]))
                })
            })
        })
    }

    /// レコードの束縛（コンストラクタの呼び出しか、別のレコードの部品）を、欄ごとの修飾にする
    fn record_value(&self, b: &Bind, name: &str, cid: ClassId) -> Res<ModMap> {
        let bind = |e: &SyntaxNode, suffix: String| Bind {
            node: e.clone(),
            file: b.file,
            ctx: b.ctx.clone(),
            suffix,
            lit: None,
        };
        let mut out = ModMap::new();
        // 配列の式 `{…}`・`{… for k in 1:n}` への添字 `[k]` は、その要素の式（S336）
        if let Some(e) = self.pick_elem(b)? {
            return self.record_value(&e, name, cid);
        }
        // 条件が定数に決まる `if` の式は、選ばれた枝（S329）
        if let Some(Expr::If(i)) = Expr::cast(b.node.clone())
            && let Some(e) = self.const_if_expr(&i, b.file, &b.ctx)
        {
            return self.record_value(&bind(e.syntax(), b.suffix.clone()), name, cid);
        }
        match Expr::cast(b.node.clone()) {
            Some(Expr::Call(c)) => {
                let rid = c
                    .syntax()
                    .children()
                    .find_map(ast::ComponentRef::cast)
                    .and_then(|r| match self.p.res_of(b.file, r.range()) {
                        Some(Resolution::Class(rid)) => Some(rid),
                        _ => None,
                    });
                // レコードを返す関数の呼び出し `R = Frames.nullRotation()` は評価せず、欄ごとに `f().T` の形で束縛する
                if rid.is_some_and(|rid| matches!(self.p.kind_of(rid), Some(K::KwFunction))) {
                    out.insert(WHOLE.to_string(), field_mod(b.clone()));
                    return Ok(out);
                }
                let rid = rid.filter(|rid| {
                    matches!(self.p.kind_of(*rid), Some(K::KwRecord | K::KwOperator))
                });
                let Some(rid) = rid else {
                    // `fill(Complex(1, 0), n)` のような組み込みの関数の呼び出しは、`Complex` の式として欄ごとに
                    return match self.complex_value(b, cid)? {
                        Some(m) => Ok(m),
                        None => un("composite-binding", format!("{name} = {}", b.node.text())),
                    };
                };
                let mut fields = Vec::new();
                self.record_fields(rid, &mut fields, 0);
                for (f, a) in fields.iter().zip(c.args()) {
                    out.insert(f.clone(), field_mod(bind(a.syntax(), String::new())));
                }
                for (f, a) in c.named_args() {
                    out.insert(f, field_mod(bind(a.syntax(), String::new())));
                }
            }
            Some(Expr::ComponentRef(r))
                if !b.node.children().any(|x| x.kind() == K::Subscripts) =>
            {
                let t = self.cref(&r, b.file, &b.ctx)?;
                // 部品の配列を通した欄 `plug.pin.i` の要素 `[k]` は `plug.pin[k].i`（名前そのままの式 `lit` で束縛する。S336）
                let through = self
                    .index_leaf(&t, &b.suffix)
                    .filter(|c| *c != format!("{t}{}", b.suffix));
                let (t, sfx) = match &through {
                    Some(c) => (c.clone(), String::new()),
                    None => (t, b.suffix.clone()),
                };
                let pre = format!("{t}{sfx}.");
                let leaves: Vec<&str> = self
                    .var_idx
                    .range(pre.clone()..)
                    .take_while(|(k, _)| k.starts_with(&pre))
                    .map(|(k, _)| &k[pre.len()..])
                    .collect();
                if leaves.is_empty() || leaves.iter().any(|l| l.contains('[')) {
                    // 宣言が後ろにある兄弟のレコード（まだ欄が無い）は、型の欄ごとに束縛する
                    out.insert(WHOLE.to_string(), field_mod(b.clone()));
                    return Ok(out);
                }
                for l in leaves {
                    let path: Vec<&str> = l.split('.').collect();
                    let el = match &through {
                        Some(c) => Bind {
                            lit: Some(format!("{c}.{l}")),
                            ..bind(&b.node, String::new())
                        },
                        None => bind(&b.node, format!("{}.{l}", b.suffix)),
                    };
                    nest(&mut out, &path, el);
                }
            }
            // `x.i[1]` のように添字のある参照は、型の欄ごとに `x.i[1].f`
            Some(Expr::ComponentRef(_)) => {
                out.insert(WHOLE.to_string(), field_mod(b.clone()));
            }
            _ => {
                // `Complex` の演算子の式 `a - b` は評価せず、欄ごとの式にする（S329）
                if let Some(m) = self.complex_value(b, cid)? {
                    return Ok(m);
                }
                return un("composite-binding", format!("{name} = {}", b.node.text()));
            }
        }
        Ok(out)
    }

    /// `Complex` の式の束縛を欄 `re`・`im` ごとの式にする（`complex_parts`）。`Complex` でない・読めない形は `None`
    fn complex_value(&self, b: &Bind, cid: ClassId) -> Res<Option<ModMap>> {
        if !is_index(&b.suffix) || !self.is_msl_complex(cid) {
            return Ok(None);
        }
        let Some((re, im)) = self.complex_parts(&b.node, b, 0)? else {
            return Ok(None);
        };
        let mut out = ModMap::new();
        for (f, t) in [("re", re), ("im", im.unwrap_or_else(|| "0".into()))] {
            let bind = Bind {
                node: b.node.clone(),
                file: b.file,
                ctx: b.ctx.clone(),
                suffix: String::new(),
                lit: Some(t),
            };
            out.insert(f.to_string(), field_mod(bind));
        }
        Ok(Some(out))
    }

    /// 束縛の式が配列の式 `{a, b}`・`{e for k in 1:n}` で、束縛に添字 `[i, …]` が付いているとき、その要素の束縛
    /// （残りの添字は付けたまま。内包の `k` は要素の値を局所変数として持つ文脈で）。S336
    fn pick_elem(&self, b: &Bind) -> Res<Option<Bind>> {
        let Some(Expr::Array(a)) = Expr::cast(b.node.clone()) else {
            return Ok(None);
        };
        let Some((head, tail)) = b.suffix.strip_prefix('[').and_then(|s| s.split_once(']')) else {
            return Ok(None);
        };
        let (first, more) = head
            .split_once(',')
            .map_or((head, None), |(f, m)| (f, Some(m)));
        let Ok(i) = first.trim().parse::<usize>() else {
            return Ok(None);
        };
        let suffix = match more {
            Some(m) => format!("[{m}]{tail}"),
            None => tail.to_string(),
        };
        let size = || {
            un(
                "composite-binding",
                format!("{} size differs", b.node.text()),
            )
        };
        if let Some(c) = b.node.children().find(|c| c.kind() == K::Comprehension) {
            let body = c.children().find_map(Expr::cast);
            let fis: Vec<SyntaxNode> = c
                .children()
                .find(|x| x.kind() == K::ForIndices)
                .map(|x| x.children().filter(|f| f.kind() == K::ForIndex).collect())
                .unwrap_or_default();
            // shortcut: 添字が 1 つの内包だけ（`for i in …, j in …` は多次元。MSL の例題では出ない）
            let ([fi], Some(body)) = (fis.as_slice(), body) else {
                return un("composite-binding", b.node.text());
            };
            let var = tokens(fi)
                .find(|t| t.kind() == K::Ident)
                .map(|t| t.text().to_string());
            let range = fi.children().find(|x| Expr::cast(x.clone()).is_some());
            let (Some(var), Some(range)) = (var, range) else {
                return un("composite-binding", b.node.text());
            };
            // 範囲 `a:s:b` の i 番目は全部を作らずに出す（束縛の要素ごとに呼ばれる。S365・#138）
            let k = i.checked_sub(1);
            let v = match Expr::cast(range.clone()) {
                Some(Expr::Range(x)) => {
                    let (a, s, e) = self.range_abc(&x, &range, b.file, &b.ctx)?;
                    (k.and_then(|k| i64::try_from(k).ok()))
                        .and_then(|k| k.checked_mul(s)?.checked_add(a))
                        .filter(|v| (s > 0 && *v <= e) || (s < 0 && *v >= e))
                }
                _ => {
                    let vals = self.range_vals(&range, b.file, &b.ctx)?;
                    k.and_then(|k| vals.get(k).copied())
                }
            };
            let Some(v) = v else {
                return size();
            };
            let mut locals = b.ctx.locals.clone();
            locals.push((var, v));
            let ctx = Rc::new(Ctx {
                pfx: b.ctx.pfx.clone(),
                lineage: b.ctx.lineage.clone(),
                locals,
                cenv: b.ctx.cenv.clone(),
                heir: b.ctx.heir,
                inst: b.ctx.inst.clone(),
            });
            return Ok(Some(Bind {
                node: body.syntax().clone(),
                ctx,
                suffix,
                lit: None,
                ..b.clone()
            }));
        }
        let Some(item) = i.checked_sub(1).and_then(|k| a.items().into_iter().nth(k)) else {
            return size();
        };
        Ok(Some(Bind {
            node: item.syntax().clone(),
            suffix,
            lit: None,
            ..b.clone()
        }))
    }

    /// 配列の式の葉 `t`（`plug.pin.v`・`u`）の要素 `ix`（`[2]`）の名前。配列でなければ `None`（スカラーは全要素で同じ）。S336
    fn index_leaf(&self, t: &str, ix: &str) -> Option<String> {
        if ix.is_empty() || !is_index(ix) || t.ends_with(']') {
            return None;
        }
        // 変数そのもの、または欄を持つレコード（`Complex` の `.re` など）
        let has = |c: &str| {
            let pre = format!("{c}.");
            self.var_idx.contains_key(c)
                || self
                    .var_idx
                    .range(pre.clone()..)
                    .next()
                    .is_some_and(|(k, _)| k.starts_with(&pre))
        };
        let whole = format!("{t}{ix}");
        if self
            .var_idx
            .get(t)
            .is_some_and(|&v| !self.vars[v].dims.is_empty())
            || has(&whole)
        {
            return Some(whole);
        }
        t.match_indices('.')
            .map(|(i, _)| format!("{}{ix}{}", &t[..i], &t[i..]))
            .find(|c| has(c))
    }

    /// 最上位の `Complex` で、欄が `re`・`im` だけ（名前だけ同じユーザー定義のレコードを展開しない。S339）
    fn is_msl_complex(&self, cid: ClassId) -> bool {
        let mut f = Vec::new();
        self.record_fields(cid, &mut f, 0);
        self.p.lib.qualified_name(cid) == "Complex" && f == ["re", "im"]
    }

    /// `Complex` の式を (実部, 虚部) の式にする。虚部 `None` は Real。読めない形は `None`（S329）。
    /// shortcut: 演算子の関数の中身は読まず、MSL の `Complex` の定義（`+ - * /`、単項 `-`）を決め打つ。`^`・`==` と、
    /// 型が分からない参照（宣言が後ろの部品。Complex と見なす）は上位の計測で足りなくなったら型を引く
    fn complex_parts(&self, n: &SyntaxNode, b: &Bind, depth: u32) -> Res<Option<CxPart>> {
        if depth > 24 {
            return Ok(None);
        }
        let d = depth + 1;
        let txt = |x: &SyntaxNode| self.text(x, b.file, &b.ctx);
        let real = |x: &SyntaxNode| -> Res<Option<CxPart>> { Ok(Some((txt(x)?, None))) };
        let Some(e) = Expr::cast(n.clone()) else {
            return Ok(None);
        };
        match e {
            Expr::Literal(_) => real(n),
            Expr::Paren(p) => match p.items().as_slice() {
                [e] => self.complex_parts(e.syntax(), b, d),
                _ => Ok(None),
            },
            Expr::Unary(u) => {
                let Some(o) = u.operand() else {
                    return Ok(None);
                };
                let Some((re, im)) = self.complex_parts(o.syntax(), b, d)? else {
                    return Ok(None);
                };
                Ok(match u.op().map(|t| t.kind()) {
                    Some(K::Plus) => Some((re, im)),
                    Some(K::Minus) => Some((
                        format!("-{}", cx_par(&re)),
                        im.map(|i| format!("-{}", cx_par(&i))),
                    )),
                    _ => None,
                })
            }
            Expr::Binary(bx) => {
                let operands = bx.operands();
                let ops = bx.ops();
                let Some(first) = operands.first() else {
                    return Ok(None);
                };
                let Some(mut acc) = self.complex_parts(first.syntax(), b, d)? else {
                    return Ok(None);
                };
                for (o, op) in operands[1..].iter().zip(ops.iter()) {
                    let Some(r) = self.complex_parts(o.syntax(), b, d)? else {
                        return Ok(None);
                    };
                    let Some(v) = cx_bin(op.kind(), acc, r) else {
                        return Ok(None);
                    };
                    acc = v;
                }
                Ok(Some(acc))
            }
            Expr::If(i) => {
                let mut parts = Vec::new();
                for (c, v) in i.branches() {
                    let Some(v) = self.complex_parts(v.syntax(), b, d)? else {
                        return Ok(None);
                    };
                    parts.push((Some(txt(c.syntax())?), v));
                }
                let Some(e) = i.else_branch() else {
                    return Ok(None);
                };
                let Some(e) = self.complex_parts(e.syntax(), b, d)? else {
                    return Ok(None);
                };
                parts.push((None, e));
                let cx = parts.iter().any(|(_, p)| p.1.is_some());
                let pick = |im: bool| {
                    let mut s = String::new();
                    for (c, p) in &parts {
                        let t = if im {
                            p.1.clone().unwrap_or_else(|| "0".into())
                        } else {
                            p.0.clone()
                        };
                        match c {
                            Some(c) if s.is_empty() => s = format!("if {c} then {t}"),
                            Some(c) => s += &format!(" elseif {c} then {t}"),
                            None => s += &format!(" else {t}"),
                        }
                    }
                    s
                };
                Ok(Some((pick(false), cx.then(|| pick(true)))))
            }
            Expr::ComponentRef(_) => {
                let t = txt(n)?;
                // 配列の式の要素（束縛に `[k]` が付いているとき）は、配列の変数・部品の配列を通した欄の `[k]`
                let t = self.index_leaf(&t, &b.suffix).unwrap_or(t);
                if self.var_idx.contains_key(&format!("{t}.re")) {
                    return Ok(Some((format!("{t}.re"), Some(format!("{t}.im")))));
                }
                // Real の部品・定数・`time` は Real。宣言が後ろでまだ無いものは Complex と見なす
                if t == "time"
                    || self.var_idx.contains_key(&t)
                    || self.eval(n, b.file, &b.ctx, 0).is_some()
                {
                    return real(n);
                }
                Ok(Some((format!("{t}.re"), Some(format!("{t}.im")))))
            }
            Expr::Call(c) => {
                // 配列の式の要素（束縛に `[k]` が付いているとき）: `fill(x, n…)` の要素は `x`（S336）
                if !b.suffix.is_empty()
                    && c.callee().as_deref() == Some("fill")
                    && let Some(x) = c.args().first()
                {
                    return self.complex_parts(x.syntax(), b, d);
                }
                let rid = c
                    .syntax()
                    .children()
                    .find_map(ast::ComponentRef::cast)
                    .and_then(|r| match self.p.res_of(b.file, r.range()) {
                        Some(Resolution::Class(rid)) => Some(rid),
                        _ => None,
                    });
                let Some(rid) = rid else {
                    return real(n); // 組み込みの関数（`sqrt`・`abs`）は Real
                };
                let q = self.p.lib.qualified_name(rid);
                if q == "Complex" || q.starts_with("Complex.") {
                    let (args, named) = (c.args(), c.named_args());
                    let get = |i: usize, f: &str| {
                        let e = named
                            .iter()
                            .find(|(k, _)| k == f)
                            .map(|(_, e)| e)
                            .or(args.get(i));
                        e.map(|e| txt(e.syntax())).transpose()
                    };
                    let (re, im) = (get(0, "re")?, get(1, "im")?);
                    return Ok(re.map(|r| (r, Some(im.unwrap_or_else(|| "0".into())))));
                }
                if let Some(f) = q.strip_prefix("Modelica.ComplexMath.") {
                    if matches!(f, "abs" | "arg" | "real" | "imag") {
                        return real(n);
                    }
                    let t = txt(n)?;
                    return Ok(Some((format!("{t}.re"), Some(format!("{t}.im")))));
                }
                Ok(None)
            }
            _ => Ok(None),
        }
    }

    /// レコードのコンストラクタの位置引数の順（`extends` の欄が先。constant は除く）
    fn record_fields(&self, cid: ClassId, out: &mut Vec<String>, depth: u32) {
        let Some(cd) = self.p.class(cid) else { return };
        let file = self.p.lib.classes[cid].file;
        if depth > 30 {
            return;
        }
        let mut bases: Vec<ast::Name> =
            cd.short_body().and_then(|s| s.base()).into_iter().collect();
        for e in cd.elements() {
            match e {
                Element::Extends(x) => bases.extend(x.name()),
                Element::Component(cc)
                    if !tokens(cc.syntax()).any(|t| t.kind() == K::KwConstant) =>
                {
                    out.extend(cc.declarations().iter().filter_map(|d| Some(d.name()?.0)));
                }
                _ => {}
            }
        }
        // `extends` の欄は自分の欄より先に並ぶ
        let own = std::mem::take(out);
        for n in bases {
            if let Some(Resolution::Class(b)) = self.p.res_of(file, n.range()) {
                self.record_fields(b, out, depth + 1);
            }
        }
        out.extend(own);
    }

    /// 大きさ: 宣言の添字、型の添字。`[:]` は束縛の式の形から決める。添字が列挙の型名なら要素の数
    fn decl_dims(
        &self,
        ctx: &Rc<Ctx>,
        file: u32,
        cc: &ast::ComponentClause,
        d: &ast::Declaration,
        name: &str,
        m: &Mod,
    ) -> Res<Vec<usize>> {
        let mut dims = Vec::new();
        let mut unsized_ = false;
        for sn in d
            .syntax()
            .children()
            .chain(cc.syntax().children())
            .filter(|c| c.kind() == K::Subscripts)
        {
            unsized_ |= tokens(&sn).any(|t| t.kind() == K::Colon);
            for e in sn.children().filter_map(Expr::cast) {
                if let Expr::ComponentRef(r) = &e
                    && let Some(Resolution::Class(cid)) = self.p.res_of(file, r.range())
                    && self.is_enum(cid)
                {
                    dims.push(self.p.lib.classes[cid].components.len());
                    continue;
                }
                match self.eval(e.syntax(), file, ctx, 0).and_then(Val::int) {
                    Some(n) if n >= 0 => dims.push(n as usize),
                    _ => {
                        let b = self
                            .env
                            .get(&format!("{}{}", ctx.pfx, e.syntax().text()))
                            .map(|b| b.node.text().to_string());
                        return un(
                            "non-constant-dim",
                            format!("{name}: {} = {b:?}", e.syntax().text()),
                        );
                    }
                }
            }
        }
        if unsized_ {
            // shortcut: 大きさを決められるのは束縛の式の形が分かるときだけ（`bind_dims`。ファイルを読む外部関数の戻り値などは断る）
            match m.val.as_ref().and_then(|b| self.bind_dims(b)) {
                Some(v) if !v.is_empty() => dims = v,
                _ => {
                    let b = (m.val.as_ref()).map(|b| format!("{}{}", b.node.text(), b.suffix));
                    return un("unsized-array", format!("{name} = {b:?}"));
                }
            }
        }
        Ok(dims)
    }

    /// 型名を、組み込みの型（短いクラスの鎖をたどった属性の修飾つき）か、部品をもつクラスにする
    fn type_of(&self, mut res: Option<Resolution>, parts: &[String]) -> Res<(Ty, Vec<usize>)> {
        let mut acc = ModMap::new();
        let mut flags = Flags::default();
        let mut conn = false;
        let mut tdims: Vec<usize> = Vec::new();
        let mut last = parts.last().cloned().unwrap_or_default();
        let tctx = Rc::new(Ctx {
            pfx: String::new(),
            lineage: Rc::new(Vec::new()),
            locals: Vec::new(),
            cenv: Cenv::default(),
            heir: None,
            inst: None,
        });
        for _ in 0..64 {
            match res {
                Some(Resolution::Builtin) => {
                    return Ok((
                        Ty::Prim {
                            ty: last,
                            mods: acc,
                            flags,
                            conn,
                        },
                        tdims,
                    ));
                }
                Some(Resolution::Class(cid)) => {
                    let cd = self.p.class(cid).ok_or_else(|| crate::Unsupported {
                        kind: "unresolved",
                        detail: "class without syntax".into(),
                    })?;
                    conn |= cd.kind_token().map(|t| t.kind()) == Some(K::KwConnector);
                    let Some(sb) = cd.short_body() else {
                        // `type T extends Real(unit = "V");`（長い形の型）は、短い形と同じに基底をたどる
                        let file = self.p.lib.classes[cid].file;
                        let base = (cd.kind_token().map(|t| t.kind()) == Some(K::KwType))
                            .then(|| {
                                cd.elements().into_iter().find_map(|e| match e {
                                    Element::Extends(x) => Some(x),
                                    _ => None,
                                })
                            })
                            .flatten();
                        if let Some(x) = base
                            && let Some(n) = x.name()
                        {
                            if let Some(cm) = x
                                .syntax()
                                .children()
                                .find(|c| c.kind() == K::ClassModification)
                            {
                                acc = merge(&acc, &self.mods_of(&cm, file, &tctx)?);
                            }
                            last = n.parts().last().cloned().unwrap_or_default();
                            res = self.p.res_of(file, n.range());
                            continue;
                        }
                        return Ok((
                            Ty::Comp {
                                cid,
                                mods: acc,
                                flags,
                                conn,
                            },
                            tdims,
                        ));
                    };
                    let file = self.p.lib.classes[cid].file;
                    let toks: Vec<K> = tokens(sb.syntax()).map(|t| t.kind()).collect();
                    if toks.contains(&K::KwEnumeration) {
                        let q = self.p.lib.qualified_name(cid);
                        return Ok((
                            Ty::Prim {
                                ty: q,
                                mods: acc,
                                flags,
                                conn,
                            },
                            tdims,
                        ));
                    }
                    if toks.contains(&K::KwDer) {
                        return un("der-class", self.p.lib.qualified_name(cid));
                    }
                    // `type Axis = Real[3]`: 添字は型の宣言の文脈（定数だけ）で評価し、使う側の添字の後ろに足す
                    for sn in sb.syntax().children().filter(|c| c.kind() == K::Subscripts) {
                        for e in sn.children().filter_map(Expr::cast) {
                            match self.eval(e.syntax(), file, &tctx, 0).and_then(Val::int) {
                                Some(n) if n >= 0 => tdims.push(n as usize),
                                _ => return un("array-type", self.p.lib.qualified_name(cid)),
                            }
                        }
                    }
                    flags = fill(
                        flags,
                        flags_of(
                            toks.iter()
                                .copied()
                                .take_while(|k| !matches!(k, K::Ident | K::Dot))
                                .skip(1),
                        )?,
                    );
                    if let Some(cm) = sb
                        .syntax()
                        .children()
                        .find(|c| c.kind() == K::ClassModification)
                    {
                        acc = merge(&acc, &self.mods_of(&cm, file, &tctx)?);
                    }
                    let Some(base) = sb.base() else {
                        return un("short-class", "no base");
                    };
                    last = base.parts().last().cloned().unwrap_or_default();
                    res = self.p.res_of(file, base.range());
                }
                Some(Resolution::Unresolved(Reason::Redeclare)) => return un("redeclare", last),
                other => {
                    return un(
                        "unresolved",
                        format!("type {} ({other:?})", parts.join(".")),
                    );
                }
            }
        }
        un("deep-extends", "short class chain")
    }

    // ---------------------------------------------------------------- 修飾

    fn mods_of(&self, cm: &SyntaxNode, file: u32, ctx: &Rc<Ctx>) -> Res<ModMap> {
        let mut out = ModMap::new();
        for ch in cm.children() {
            match ch.kind() {
                K::Redeclaration => {
                    if let Some((name, m)) = self.redeclaration(&ch, file, ctx)? {
                        out = merge(&ModMap::from([(name, m)]), &out);
                    }
                }
                K::BreakModification => return un("break", ch.text().to_string()),
                K::ElementModification => {
                    let each = tokens(&ch).any(|t| t.kind() == K::KwEach);
                    let Some(n) = ch.children().find_map(ast::Name::cast) else {
                        continue;
                    };
                    let mut leaf = self.mod_of(
                        ch.children().find(|c| c.kind() == K::Modification),
                        file,
                        ctx,
                        each,
                    )?;
                    leaf.each = each;
                    let parts = n.parts();
                    for p in parts[1..].iter().rev() {
                        leaf = Mod {
                            sub: ModMap::from([(p.clone(), leaf)]),
                            ..Mod::default()
                        };
                    }
                    let one = ModMap::from([(parts[0].clone(), leaf)]);
                    out = merge(&one, &out);
                }
                _ => {}
            }
        }
        Ok(out)
    }

    /// `redeclare …` を `(名前, Mod)` にする。関数の差し替えは `None`（関数は平らにしない）。
    /// 型の名前はここ（書いた場所）で解決する
    fn redeclaration(
        &self,
        red: &SyntaxNode,
        file: u32,
        ctx: &Rc<Ctx>,
    ) -> Res<Option<(String, Mod)>> {
        let Some(snip) = self.p.snippet(file, red) else {
            return un("redeclare", red.text().to_string());
        };
        match &snip.item {
            SnipItem::Comp(cc, d) => {
                let (Some((name, _)), Some(tn)) = (d.name(), cc.type_name()) else {
                    return un("redeclare", red.text().to_string());
                };
                let (res, extra, _) = self.class_res(snip.fid, &tn, ctx);
                let mut m = self.mod_of(
                    d.syntax().children().find(|c| c.kind() == K::Modification),
                    snip.fid,
                    ctx,
                    false,
                )?;
                m.redecl = Some(Rc::new(Redecl::Comp {
                    snip: snip.clone(),
                    cc: cc.clone(),
                    d: d.clone(),
                    ctx: ctx.clone(),
                    res,
                    parts: tn.parts(),
                    extra,
                }));
                Ok(Some((name, m)))
            }
            SnipItem::Class(cd) => {
                let (Some(name), Some(sb)) = (cd.name(), cd.short_body()) else {
                    return un("redeclare", red.text().to_string());
                };
                // 関数は平らにしない（差し替え先は呼び出しの評価器だけが引く。S376・#155）。引けない・修飾つきのものは今までどおり捨てる
                let is_fn = matches!(cd.kind_token().map(|t| t.kind()), Some(K::KwFunction));
                let Some(base) = sb.base() else {
                    return un("redeclare", red.text().to_string());
                };
                let (Some(Resolution::Class(cid)), _, _) = self.class_res(snip.fid, &base, ctx)
                else {
                    if is_fn {
                        return Ok(None);
                    }
                    return un("redeclare", format!("{name} = {}", base.parts().join(".")));
                };
                let cm = (sb.syntax().children()).find(|c| c.kind() == K::ClassModification);
                let extra = match cm {
                    // 関数の `(…)` の修飾は評価器が見ない。あることだけ覚える（`call_target` は断る）
                    Some(_) if is_fn => ModMap::from([(String::new(), Mod::default())]),
                    Some(cm) => self.mods_of(&cm, snip.fid, ctx)?,
                    None => ModMap::new(),
                };
                let m = Mod {
                    redecl: Some(Rc::new(Redecl::Class { cid, extra })),
                    ..Mod::default()
                };
                Ok(Some((name, m)))
            }
        }
    }

    /// 修飾の中の `redeclare package M = X` を、インスタンスの差し替えの表に足す（先に足したものが勝つ）
    fn add_cenv(&self, ctx: &Rc<Ctx>, mods: &ModMap) {
        for (k, m) in mods {
            if let Some(Redecl::Class { cid, extra }) = m.redecl.as_deref() {
                ctx.cenv
                    .borrow_mut()
                    .entry(k.clone())
                    .or_insert_with(|| (*cid, extra.clone()));
            }
        }
    }

    /// 型名の解決。先頭がこのインスタンスで差し替えられた名前（`Medium.BaseProperties` の `Medium`）なら、
    /// 差し替え先のクラスの中を引く。差し替えがあれば、その `(…)` の修飾も返す
    fn class_res(
        &self,
        file: u32,
        tn: &ast::Name,
        ctx: &Rc<Ctx>,
    ) -> (Option<Resolution>, ModMap, Option<ClassId>) {
        let stat = self.p.res_of(file, tn.range());
        let parts = tn.parts();
        let hit = (!tn.is_global())
            .then(|| ctx.cenv.borrow().get(&parts[0]).cloned())
            .flatten();
        let Some((mut cur, extra)) = hit else {
            // 受け継いだクラスの中の型名（`ThermodynamicState`）は、差し替え後のパッケージのメンバーで引き直す
            if let (Some(h), Some(Resolution::Class(c)), 1) = (ctx.heir, &stat, parts.len())
                && let Some(c2) = self.heir_member(h, *c)
            {
                return (Some(Resolution::Class(c2)), ModMap::new(), None);
            }
            return (stat, ModMap::new(), None);
        };
        let mut owner = None;
        for p in &parts[1..] {
            owner = Some(cur);
            match self.member_class(cur, p) {
                Some(c) => cur = c,
                None => return (stat, ModMap::new(), None),
            }
        }
        let extra = if parts.len() == 1 {
            extra
        } else {
            ModMap::new()
        };
        (Some(Resolution::Class(cur)), extra, owner)
    }

    /// `Medium.BaseProperties`（`package Medium = MoistAir` の別名）のように、静的に引けたクラスの外側が部分的な（`partial`）パッケージで、
    /// 型名の前半がそれを受け継いだパッケージを指すとき、その受け継いだ側（差し替え後のパッケージと同じ扱い）
    fn alias_heir(&self, file: u32, tn: &ast::Name, res: &Option<Resolution>) -> Option<ClassId> {
        let parts = tn.parts();
        if parts.len() < 2 || file >= crate::SNIP_FILE_BASE || tn.is_global() {
            return None;
        }
        let Some(Resolution::Class(c)) = res else {
            return None;
        };
        let p0 = self.p.lib.classes[*c].parent?;
        // 受け継いだ側の型（`class extends` の基底の中の `ThermodynamicState`）を引き直すのは、これらのときだけ（毎回やると遅い）
        if !self.p.is_partial(p0) && !self.p.class(*c).is_some_and(|cd| cd.is_class_extends()) {
            return None;
        }
        let scope = self.p.lib.class_at(file, u32::from(tn.range().start()))?;
        let r = Ref {
            file,
            range: tn.range(),
            scope,
            kind: RefKind::Type,
            parts: parts[..parts.len() - 1].to_vec(),
            global: false,
            local: false,
        };
        match self.p.lib.resolve(&r) {
            Resolution::Class(a) if self.p.lineage(a).contains(&p0) => Some(a),
            _ => None,
        }
    }

    /// パッケージ `pkg` の中（受け継ぎを含む）のクラス `name`。`pkg` の外のクラスは返さない
    fn member_class(&self, pkg: ClassId, name: &str) -> Option<ClassId> {
        // パッケージ自身の `extends P(redeclare record R = X)` が先
        self.pkg_env(pkg);
        let hit = self
            .pkg_cenvs
            .borrow()
            .get(&pkg)
            .and_then(|e| e.borrow().get(name).map(|(c, _)| *c));
        if hit.is_some() {
            return hit;
        }
        let r = Ref {
            file: self.p.lib.classes[pkg].file,
            range: rowan::TextRange::default(),
            scope: pkg,
            kind: RefKind::Type,
            parts: vec![name.to_string()],
            global: false,
            local: false,
        };
        match self.p.lib.resolve(&r) {
            Resolution::Class(c)
                if self.p.lib.classes[c]
                    .parent
                    .is_some_and(|q| self.p.lineage(pkg).contains(&q)) =>
            {
                Some(c)
            }
            _ => None,
        }
    }

    /// 静的に解決したクラス `c`（パッケージ `P` のメンバー）を、`P` を受け継いだパッケージ `heir` の同じ名前のメンバーに差し替える
    fn heir_member(&self, heir: ClassId, c: ClassId) -> Option<ClassId> {
        let p = self.p.lib.classes[c].parent?;
        if p == heir || !self.p.lineage(heir).contains(&p) {
            return None;
        }
        self.member_class(heir, &self.p.lib.classes[c].name)
            .filter(|c2| *c2 != c)
    }

    /// `Modification`（`(…)` と `= 式`）を `Mod` にする
    fn mod_of(&self, m: Option<SyntaxNode>, file: u32, ctx: &Rc<Ctx>, each: bool) -> Res<Mod> {
        let Some(m) = m else {
            return Ok(Mod {
                each,
                ..Mod::default()
            });
        };
        let sub = match m.children().find(|c| c.kind() == K::ClassModification) {
            Some(cm) => self.mods_of(&cm, file, ctx)?,
            None => ModMap::new(),
        };
        let val = m
            .children()
            .find(|c| Expr::cast(c.clone()).is_some())
            .map(|node| Bind {
                node,
                file,
                ctx: ctx.clone(),
                suffix: String::new(),
                lit: None,
            });
        Ok(Mod {
            val,
            sub,
            each,
            redecl: None,
        })
    }

    fn print_bind(&self, b: &Bind) -> Res<String> {
        if let Some(l) = &b.lit {
            return Ok(l.clone());
        }
        let t = self.text(&b.node, b.file, &b.ctx)?;
        // 添字は参照にしか付けられない。式には括弧を付ける（S329）
        if b.suffix.starts_with('[') && b.node.kind() != K::ComponentRef {
            return Ok(format!("({t}){}", b.suffix));
        }
        Ok(t + &b.suffix)
    }

    // ---------------------------------------------------------------- 式の印字

    fn text(&self, n: &SyntaxNode, file: u32, ctx: &Rc<Ctx>) -> Res<String> {
        let mut out = Vec::new();
        self.pr(n, file, ctx, &mut out)?;
        Ok(join(&out))
    }

    fn pr(&self, n: &SyntaxNode, file: u32, ctx: &Rc<Ctx>, out: &mut Vec<String>) -> Res<()> {
        if let Some(c) = ast::ComponentRef::cast(n.clone()) {
            let t = self.cref(&c, file, ctx)?;
            // 部品の配列を通した欄 `a.pin.v` は、Rumoca の flat と同じに要素の配列 `{a.pin[1].v, …}` にする
            out.push(match self.array_elems(&t) {
                Some(v) => format!("{{{}}}", v.join(", ")),
                None => t,
            });
            return Ok(());
        }
        // 内包 `e for j in r` の中の `inStream`・`actualStream` は、`j` ごとに `e` を並べる（添字が定数でないと端の名前が決まらない。S361）
        if let Some(Expr::Comprehension(c)) = Expr::cast(n.clone())
            && c.syntax()
                .descendants()
                .filter_map(ast::CallExpr::cast)
                .any(|x| matches!(x.callee().as_deref(), Some("inStream" | "actualStream")))
        {
            let mut fis = c
                .syntax()
                .children()
                .find(|x| x.kind() == K::ForIndices)
                .into_iter()
                .flat_map(|x| x.children().filter(|f| f.kind() == K::ForIndex));
            // shortcut: 添字が 1 つの内包だけ（`for i in …, j in …` は断る）
            let (Some(fi), None) = (fis.next(), fis.next()) else {
                return un("stream", c.syntax().text().to_string());
            };
            let var = tokens(&fi)
                .find(|t| t.kind() == K::Ident)
                .map(|t| t.text().to_string());
            let range = fi.children().find(|x| Expr::cast(x.clone()).is_some());
            let body = c.syntax().children().find_map(Expr::cast);
            let (Some(var), Some(range), Some(body)) = (var, range, body) else {
                return un("stream", c.syntax().text().to_string());
            };
            // 親が `{…}` でない（`sum(inStream(f[i].h) for i in 1:3)`）ときは、並べた項を `{…}` で包んで引数 1 つの配列にする（S372）
            let braces = c.syntax().parent().is_none_or(|p| p.kind() != K::ArrayExpr);
            if braces {
                out.push("{".to_string());
            }
            for (k, v) in self.range_vals(&range, file, ctx)?.into_iter().enumerate() {
                let mut locals = ctx.locals.clone();
                locals.push((var.clone(), v));
                let lctx = Rc::new(Ctx {
                    pfx: ctx.pfx.clone(),
                    lineage: ctx.lineage.clone(),
                    locals,
                    cenv: ctx.cenv.clone(),
                    heir: ctx.heir,
                    inst: ctx.inst.clone(),
                });
                if k > 0 {
                    out.push(",".to_string());
                }
                self.pr(body.syntax(), file, &lctx, out)?;
            }
            if braces {
                out.push("}".to_string());
            }
            return Ok(());
        }
        // 条件が定数の `if` 式は、選んだ枝だけを印字する（Rumoca の flat も落とす。`der` が枝にあれば状態の数に効く。S312）
        if let Some(Expr::If(i)) = Expr::cast(n.clone())
            && let Some(taken) = self.const_if_expr(&i, file, ctx)
        {
            let paren = !matches!(
                taken,
                Expr::ComponentRef(_) | Expr::Literal(_) | Expr::Call(_)
            );
            if paren {
                out.push("(".to_string());
            }
            self.pr(taken.syntax(), file, ctx, out)?;
            if paren {
                out.push(")".to_string());
            }
            return Ok(());
        }
        for e in n.children_with_tokens() {
            if let Some(t) = e.as_token() {
                if !t.kind().is_trivia() {
                    out.push(t.text().to_string());
                }
            } else if let Some(c) = e.as_node()
                && !matches!(c.kind(), K::Annotation | K::Comment)
            {
                self.pr(c, file, ctx, out)?;
            }
        }
        Ok(())
    }

    /// 条件が先頭から順に定数の真偽に評価できる `if` 式の、選ばれる枝（途中で評価できなければ None）
    fn const_if_expr(&self, i: &ast::IfExpr, file: u32, ctx: &Rc<Ctx>) -> Option<Expr> {
        for (c, v) in i.branches() {
            match self.eval(c.syntax(), file, ctx, 0)? {
                Val::B(true) => return Some(v),
                Val::B(false) => {}
                _ => return None,
            }
        }
        i.else_branch()
    }

    fn subs(&self, sn: &SyntaxNode, file: u32, ctx: &Rc<Ctx>) -> Res<String> {
        let mut parts = Vec::new();
        for e in sn.children_with_tokens() {
            if let Some(t) = e.as_token() {
                if t.kind() == K::Colon {
                    parts.push(":".to_string());
                }
            } else if let Some(c) = e.as_node()
                && Expr::cast(c.clone()).is_some()
            {
                parts.push(match self.eval(c, file, ctx, 0) {
                    Some(Val::I(i)) => i.to_string(),
                    _ => self.text(c, file, ctx)?,
                });
            }
        }
        Ok(format!("[{}]", parts.join(",")))
    }

    /// `connect` の端の印字。最後の添字が範囲 `[a:s:b]` で評価できるなら、要素ごとの端 `x[k]`（それ以外は 1 つ）。S367
    fn conn_ends(&self, c: &ast::ComponentRef, file: u32, ctx: &Rc<Ctx>) -> Res<Vec<String>> {
        let t = self.cref(c, file, ctx)?;
        let range = (c
            .syntax()
            .children()
            .filter(|x| x.kind() == K::Subscripts)
            .last())
        .and_then(|sn| single(sn.children().filter(|x| Expr::cast(x.clone()).is_some())))
        .filter(|e| matches!(Expr::cast(e.clone()), Some(Expr::Range(_))));
        // shortcut: 範囲の印字に `[` を含まないものだけ（範囲の最後の添字の `]` までを `[k]` に置き換える）
        let (Some(r), Some(at)) = (range, t.rfind('[')) else {
            return Ok(vec![t]);
        };
        let (Ok(ks), Some(close)) = (self.range_vals(&r, file, ctx), t[at..].find(']')) else {
            return Ok(vec![t]);
        };
        let (head, tail) = (&t[..at], &t[at + close + 1..]);
        Ok(ks
            .into_iter()
            .map(|k| format!("{head}[{k}]{tail}"))
            .collect())
    }

    /// 成分参照の印字。部品は前置きを付け、`for` の添字は値にし、添字は定数なら評価する
    fn cref(&self, c: &ast::ComponentRef, file: u32, ctx: &Rc<Ctx>) -> Res<String> {
        let n = c.syntax();
        let mut global = false;
        let mut segs: Vec<(String, String)> = Vec::new();
        for e in n.children_with_tokens() {
            if let Some(t) = e.as_token() {
                match t.kind() {
                    K::Dot if segs.is_empty() => global = true,
                    K::Ident => segs.push((t.text().to_string(), String::new())),
                    _ => {}
                }
            } else if let Some(sn) = e.as_node()
                && sn.kind() == K::Subscripts
                && let Some(last) = segs.last_mut()
            {
                last.1 = self.subs(sn, file, ctx)?;
            }
        }
        if segs.is_empty() {
            return Ok(String::new());
        }
        let r = self.p.res_of(file, c.range());
        let mut qualified = None;
        match &r {
            Some(Resolution::Local) => {
                if let Some(v) = ctx.local(&segs[0].0) {
                    segs[0].0 = v.to_string();
                }
            }
            Some(Resolution::Component(cid, idx)) => {
                if ctx.lineage.contains(cid) {
                    let full = format!("{}{}", ctx.pfx, segs[0].0);
                    segs[0].0 = self.alias.get(&full).cloned().unwrap_or(full);
                } else {
                    // 外側のパッケージの定数・列挙の要素: 完全修飾で出す
                    let comp = &self.p.lib.classes[*cid].components[*idx].name;
                    if let Some(k) = segs.iter().position(|(s, _)| s == comp) {
                        qualified = Some((self.p.lib.qualified_name(*cid), k));
                    }
                }
            }
            Some(Resolution::Unresolved(Reason::Redeclare)) => {
                return un("redeclare", c.parts().join("."));
            }
            Some(Resolution::Unresolved(why)) => {
                return un("unresolved", format!("{} ({why:?})", c.parts().join(".")));
            }
            _ => {}
        }
        let body = |from: usize| {
            segs[from..]
                .iter()
                .map(|(s, x)| format!("{s}{x}"))
                .collect::<Vec<_>>()
                .join(".")
        };
        let s = match qualified {
            Some((q, k)) => format!("{q}.{}", body(k)),
            None => body(0),
        };
        Ok(if global { format!(".{s}") } else { s })
    }

    // ---------------------------------------------------------------- 定数の評価

    /// 参照 `r` の名前の先頭 `k` 個を型として引いたクラス
    fn prefix_class(&self, file: u32, r: &ast::ComponentRef, k: usize) -> Option<ClassId> {
        if file >= crate::SNIP_FILE_BASE {
            return None;
        }
        let scope = self.p.lib.class_at(file, u32::from(r.range().start()))?;
        let pre = Ref {
            file,
            range: r.range(),
            scope,
            kind: RefKind::Type,
            parts: r.parts()[..k].to_vec(),
            global: r.is_global(),
            local: false,
        };
        match self.p.lib.resolve(&pre) {
            Resolution::Class(h) => Some(h),
            _ => None,
        }
    }

    /// パッケージの定数 `P5.n`（宣言したクラスは `cid`）の参照の、名前が通ったパッケージ `P5`。`cid` と同じか、`cid` を受け継いでいなければ `None`（S376・#156）
    fn via_class(&self, file: u32, r: &ast::ComponentRef, cid: ClassId) -> Option<ClassId> {
        let n = r.parts().len();
        let h = self.prefix_class(file, r, n.checked_sub(1).filter(|k| *k > 0)?)?;
        (h != cid && self.p.lineage(h).contains(&cid)).then_some(h)
    }

    /// `String(v)`（整数・論理・列挙の要素。実数は書式が要るので決めない。S377）
    fn string_of(&self, v: Val) -> Option<Val> {
        Some(self.strs.intern(&match v {
            Val::I(i) => i.to_string(),
            Val::B(b) => b.to_string(),
            Val::E(c, i) => self.p.lib.classes[c].components[i].name.clone(),
            _ => return None,
        }))
    }

    /// 名前の定数・parameter の束縛の式 `n` の値。成功した値を覚える（`c_i = c_{i-1} + c_{i-1}` の連鎖が 2^n 回の評価にならない。S376・#158）。
    /// 差し替えの `(…)` の修飾がある文脈は覚えない
    fn eval_bind(&self, n: &SyntaxNode, file: u32, ctx: &Rc<Ctx>, d: u32) -> Option<Val> {
        let r = n.text_range();
        let key = ctx_hash(ctx).map(|h| (file, r.start().into(), r.end().into(), h));
        if let Some(v) = key.and_then(|k| self.eval_memo.borrow().get(&k).copied()) {
            return Some(v);
        }
        let v = self.eval(n, file, ctx, d)?;
        if let Some(k) = key {
            self.eval_memo.borrow_mut().insert(k, v);
        }
        Some(v)
    }

    fn eval(&self, n: &SyntaxNode, file: u32, ctx: &Rc<Ctx>, depth: u32) -> Option<Val> {
        if depth > 48 {
            return None;
        }
        let d = depth + 1;
        match Expr::cast(n.clone())? {
            Expr::Literal(l) => {
                let t = l.token()?;
                match t.kind() {
                    K::UInt => t.text().parse().ok().map(Val::I),
                    K::UReal => t.text().parse().ok().map(Val::R),
                    K::KwTrue => Some(Val::B(true)),
                    K::KwFalse => Some(Val::B(false)),
                    K::Str => Some(self.strs.intern(&unquote(t.text()))),
                    _ => None,
                }
            }
            Expr::Paren(p) => match p.items().as_slice() {
                [e] => self.eval(e.syntax(), file, ctx, d),
                _ => None,
            },
            Expr::Unary(u) => {
                let v = self.eval(u.operand()?.syntax(), file, ctx, d)?;
                unop(u.op()?.kind(), v)
            }
            Expr::Binary(b) => {
                let operands = b.operands();
                let ops = b.ops();
                let mut acc = self.eval(operands.first()?.syntax(), file, ctx, d)?;
                for (o, op) in operands[1..].iter().zip(ops.iter()) {
                    let r = self.eval(o.syntax(), file, ctx, d)?;
                    acc = binop(op.kind(), acc, r, &self.strs)?;
                }
                Some(acc)
            }
            Expr::If(i) => {
                for (c, v) in i.branches() {
                    if let Val::B(true) = self.eval(c.syntax(), file, ctx, d)? {
                        return self.eval(v.syntax(), file, ctx, d);
                    }
                }
                self.eval(i.else_branch()?.syntax(), file, ctx, d)
            }
            Expr::ComponentRef(c) => {
                let parts = c.parts();
                // 添字つきの参照は、配列の式の束縛の要素（`kDegraded[i,1]`。部品の欄を通さない 1 つの名前だけ。S367）
                let mut subs = n.children().filter(|x| x.kind() == K::Subscripts);
                let sub = match (subs.next(), subs.next()) {
                    (None, _) => None,
                    (Some(sn), None) if parts.len() == 1 => Some(
                        (sn.children().filter_map(Expr::cast))
                            .map(|e| self.eval(e.syntax(), file, ctx, d)?.int())
                            .collect::<Option<Vec<_>>>()?,
                    ),
                    _ => return None,
                };
                let at = |b: &Bind| match &sub {
                    Some(ix) => self.eval_at(&b.node, ix, b.file, &b.ctx, d),
                    None => self.eval_bind(&b.node, b.file, &b.ctx, d),
                };
                // `Medium.nX`: `Medium` が `redeclare` された package なら、差し替え先の定数
                if sub.is_none()
                    && let Some(b) = self.dyn_const(&c, ctx)
                {
                    let (node, f, c2) = b?;
                    return self.eval_bind(&node, f, &c2, d);
                }
                match &self.p.res_of(file, c.range())? {
                    Resolution::Local if parts.len() == 1 && sub.is_none() => {
                        ctx.local(&parts[0]).map(Val::I)
                    }
                    Resolution::Component(cid, idx) => {
                        // 関数の入力は呼び出しの引数の値（`output Real y[n]` の `n`）。関数の中の定数は下のパッケージの定数と同じ
                        if ctx.pfx == FN
                            && parts.len() == 1
                            && let Some(v) = ctx.local(&parts[0])
                        {
                            return Some(Val::I(v));
                        }
                        if ctx.pfx != PKG && ctx.pfx != FN && ctx.lineage.contains(cid) {
                            // `outer` の部品の欄 `world.enableAnimation` は、先頭を inner の名前に付け替える（S329）
                            let key = format!("{}{}", ctx.pfx, parts.join("."));
                            let key = match self.alias.get(&format!("{}{}", ctx.pfx, parts[0])) {
                                Some(a) => [a.as_str()]
                                    .into_iter()
                                    .chain(parts[1..].iter().map(String::as_str))
                                    .collect::<Vec<_>>()
                                    .join("."),
                                None => key,
                            };
                            let Some(b) = self.env.get(&key) else {
                                // まだ平らにしていない部品（宣言が後ろの `inner world`）の欄は、修飾か型の既定値から
                                let tgt = key.strip_suffix(&format!(".{}", parts.last()?))?;
                                // 欄の既定値が同じ部品の別の欄（`massDynamics = energyDynamics`）を引くときは、前置きが部品（`parts` は欄だけ。S355）
                                return (parts.len() <= 2 && sub.is_none())
                                    .then(|| self.pending_field(tgt, parts.last()?, d))
                                    .flatten();
                            };
                            if b.lit.is_some() {
                                return None;
                            }
                            // `r = other` の欄は `other.f` の束縛を評価する（`other` がさらに別のレコードの欄なら、鎖をたどる。S367）
                            let mut cur = std::borrow::Cow::Borrowed(b);
                            for _ in 0..16 {
                                if cur.suffix.is_empty() {
                                    break;
                                }
                                cur = std::borrow::Cow::Owned(self.field_bind(&cur)?);
                            }
                            let b = &*cur;
                            if !b.suffix.is_empty() || b.lit.is_some() {
                                return None;
                            }
                            at(b)
                        } else {
                            // 列挙の要素、またはパッケージの定数
                            let comp = &self.p.lib.classes[*cid].components[*idx].name;
                            if parts.last() != Some(comp) {
                                return None;
                            }
                            if self.is_enum(*cid) {
                                return Some(Val::E(*cid, *idx));
                            }
                            let via = self.via_class(file, &c, *cid);
                            let (node, f, c2) = self.pkg_binding(ctx, *cid, comp, via)?;
                            match &sub {
                                Some(ix) => self.eval_at(&node, ix, f, &c2, d),
                                None => self.eval_bind(&node, f, &c2, d),
                            }
                        }
                    }
                    _ => None,
                }
            }
            Expr::Call(c) => {
                let args = c.args();
                let v = |i: usize| self.eval(args.get(i)?.syntax(), file, ctx, d);
                match c.callee()?.as_str() {
                    "size" => {
                        let Expr::ComponentRef(r) = args.first()? else {
                            return None;
                        };
                        let k = v(1)?.int()? as usize;
                        // パッケージの定数の配列（`size(substanceNames, 1)`）は、束縛の式の形から
                        let bound = match self.dyn_const(r, ctx) {
                            Some(b) => Some(b?),
                            None => match &self.p.res_of(file, r.range())? {
                                Resolution::Component(cid, idx)
                                    if ctx.pfx == PKG || !ctx.lineage.contains(cid) =>
                                {
                                    let comp = &self.p.lib.classes[*cid].components[*idx].name;
                                    self.pkg_binding(ctx, *cid, comp, self.via_class(file, r, *cid))
                                }
                                _ => None,
                            },
                        };
                        if let Some((node, f, c2)) = bound {
                            let dims = self.dims_of(&node, f, &c2)?;
                            return dims.get(k.checked_sub(1)?).map(|&x| Val::I(x as i64));
                        }
                        let name = self.cref(r, file, ctx).ok()?;
                        let dims = self.var_dims(&name)?;
                        dims.get(k.checked_sub(1)?).map(|&x| Val::I(x as i64))
                    }
                    // 全部の `connect` がそろうまで（`card` が `Some` になるまで）は評価できない
                    "cardinality" => {
                        let Expr::ComponentRef(r) = args.first()? else {
                            return None;
                        };
                        let name = self.cref(r, file, ctx).ok()?;
                        Some(Val::I(*self.card.as_ref()?.get(&name).unwrap_or(&0)))
                    }
                    "max" | "min" => {
                        let is_max = c.callee()?.as_str() == "max";
                        // `max({a, b})`・`max([a; b])` は要素の最大
                        let xs: Vec<Val> = match args.as_slice() {
                            [a @ (Expr::Array(_) | Expr::Matrix(_))] => a
                                .syntax()
                                .children()
                                .filter_map(Expr::cast)
                                .map(|e| self.eval(e.syntax(), file, ctx, d))
                                .collect::<Option<_>>()?,
                            _ => (0..args.len()).map(v).collect::<Option<_>>()?,
                        };
                        extremum(&xs, is_max, &self.strs)
                    }
                    f if BUILTINS.contains(&f) => {
                        builtin(f, &(0..args.len()).map(v).collect::<Option<Vec<_>>>()?)
                    }
                    "String" if args.len() == 1 && c.named_args().is_empty() => {
                        self.string_of(v(0)?)
                    }
                    // Modelica の関数（`algorithm` を実行する。`fneval`。S370）
                    _ => self.call_fn(&c, file, ctx, d),
                }
            }
            _ => None,
        }
    }

    /// 配列の式 `{a, b}`・`{e for k in 1:n}` の要素の値（内包は添字が 1 つのものだけ。`k` は要素の値を局所変数として評価する）。S367
    fn eval_items(&self, n: &SyntaxNode, file: u32, ctx: &Rc<Ctx>, d: u32) -> Option<Vec<Val>> {
        let Some(Expr::Array(a)) = Expr::cast(n.clone()) else {
            return None;
        };
        let Some(c) = n.children().find(|c| c.kind() == K::Comprehension) else {
            return (a.items().iter())
                .map(|e| self.eval(e.syntax(), file, ctx, d))
                .collect();
        };
        let body = c.children().find_map(Expr::cast)?;
        let fis = c.children().find(|x| x.kind() == K::ForIndices)?;
        let fi = single(fis.children().filter(|f| f.kind() == K::ForIndex))?;
        let var = tokens(&fi)
            .find(|t| t.kind() == K::Ident)?
            .text()
            .to_string();
        let range = fi.children().find(|x| Expr::cast(x.clone()).is_some())?;
        self.range_vals(&range, file, ctx)
            .ok()?
            .into_iter()
            .map(|v| {
                let mut locals = ctx.locals.clone();
                locals.push((var.clone(), v));
                let ctx = Rc::new(Ctx {
                    locals,
                    pfx: ctx.pfx.clone(),
                    lineage: ctx.lineage.clone(),
                    cenv: ctx.cenv.clone(),
                    heir: ctx.heir,
                    inst: ctx.inst.clone(),
                });
                self.eval(body.syntax(), file, &ctx, d)
            })
            .collect()
    }

    /// 配列の式の束縛 `{a, b}`・`[a, b; c, d]` の要素 `ix`（1 始まり。要素はスカラーだけ）の値。S367
    fn eval_at(&self, n: &SyntaxNode, ix: &[i64], file: u32, ctx: &Rc<Ctx>, d: u32) -> Option<Val> {
        let Some((&k, rest)) = ix.split_first() else {
            return self.eval(n, file, ctx, d);
        };
        let k = usize::try_from(k.checked_sub(1)?).ok()?;
        match Expr::cast(n.clone())? {
            Expr::Array(a) if !n.children().any(|c| c.kind() == K::Comprehension) => {
                self.eval_at(a.items().get(k)?.syntax(), rest, file, ctx, d)
            }
            // 行は `;`、行の中は `,`。`[r, c]` は行 r の c 番目
            Expr::Matrix(m) => {
                let mut rows: Vec<Vec<Expr>> = vec![Vec::new()];
                for ch in m.syntax().children_with_tokens() {
                    if let Some(e) = ch.as_node().and_then(|c| Expr::cast(c.clone())) {
                        rows.last_mut()?.push(e);
                    } else if ch.as_token().is_some_and(|t| t.kind() == K::Semi) {
                        rows.push(Vec::new());
                    }
                }
                let (&c, rest) = rest.split_first()?;
                let e = rows.get(k)?.get(usize::try_from(c.checked_sub(1)?).ok()?)?;
                self.eval_at(e.syntax(), rest, file, ctx, d)
            }
            _ => None,
        }
    }

    /// 変数の大きさ。まだ平らにしていなければ、宣言から出す（循環は深さで止める）
    fn var_dims(&self, name: &str) -> Option<Vec<usize>> {
        if let Some(&i) = self.var_idx.get(name) {
            return Some(self.vars[i].dims.clone());
        }
        let Some(p) = self.pending.get(name) else {
            // 宣言が後ろの部品の欄 `stop.on`: 型の宣言から
            let (tgt, field) = name.rsplit_once('.')?;
            if !self.not_instantiated(tgt) {
                return None;
            }
            let cid = self.pending_type(tgt)?;
            let modified: Vec<&String> = self.pending.get(tgt)?.m.sub.keys().collect();
            return self.pkg_field_dims(cid, field, self.pending_bind(tgt, field), &modified);
        };
        if self.depth.get() > 16 {
            return None;
        }
        self.depth.set(self.depth.get() + 1);
        let r = self.decl_dims(&p.ctx, p.file, &p.cc, &p.d, name, &p.m);
        self.depth.set(self.depth.get() - 1);
        r.ok()
    }

    /// まだ平らにしていない部品 `tgt` の欄 `field` の束縛。宣言した場所の修飾が先、無ければ型の既定値（S329）
    fn pending_bind(&self, tgt: &str, field: &str) -> Option<Bind> {
        let p = self.pending.get(tgt)?;
        if let Some(b) = p.m.sub.get(field).and_then(|x| x.val.as_ref()) {
            return Some(b.clone());
        }
        // 型の既定値の式が、宣言の場所で修飾された欄を使うなら決めない（この文脈は修飾を知らない）
        let env = self.pkg_env(self.pending_type(tgt)?)?;
        let mut b = env.get(field)?.clone();
        // 既定値が別の欄そのもの（`massDynamics = energyDynamics`）なら、その欄の修飾か既定値にたどる（S355）
        for _ in 0..8 {
            let Some(Expr::ComponentRef(c)) = Expr::cast(b.node.clone()) else {
                break;
            };
            let parts = c.parts();
            let [f] = parts.as_slice() else { break };
            if !b.suffix.is_empty() || b.lit.is_some() {
                break;
            }
            if let Some(m) = p.m.sub.get(f).and_then(|x| x.val.as_ref()) {
                return Some(m.clone());
            }
            let Some(next) = env.get(f) else { break };
            b = next.clone();
        }
        let modified: Vec<&String> = p.m.sub.keys().collect();
        (!mentions(&b.node, &modified)).then_some(b)
    }

    /// 部品 `tgt` の宣言があって、まだ中身を平らにしていない（配列でなく、`tgt.…` の変数が無い）。
    /// `pending` は平らにした後も残るので、変数の有無でも見る（部品の配列の欄 `pin.i` は要素の数をかけるので対象外）
    fn not_instantiated(&self, tgt: &str) -> bool {
        let Some(p) = self.pending.get(tgt) else {
            return false;
        };
        let pre = format!("{tgt}.");
        let has_sub = |n: &SyntaxNode| n.children().any(|c| c.kind() == K::Subscripts);
        !(has_sub(p.d.syntax())
            || has_sub(p.cc.syntax())
            || self
                .var_idx
                .range(pre.clone()..)
                .next()
                .is_some_and(|(k, _)| k.starts_with(&pre)))
    }

    /// まだ平らにしていない部品 `tgt` の型（部品をもつクラス）
    fn pending_type(&self, tgt: &str) -> Option<ClassId> {
        let p = self.pending.get(tgt)?;
        let tn = p.cc.type_name()?;
        let (res, _, _) = self.class_res(p.file, &tn, &p.ctx);
        match self.type_of(res, &tn.parts()).ok()? {
            (Ty::Comp { cid, .. }, _) => Some(cid),
            _ => None,
        }
    }

    /// クラス `cid` の欄 `field` の大きさ。宣言の添字と束縛（`over` があればそれ、無ければ `extends` の修飾も合わせたもの）から。
    /// `modified`（宣言の場所で修飾された欄の名前）を宣言が直接使うなら、この文脈は修飾を知らないので決めない。
    /// shortcut: 欄の型の添字（`type Axis = Real[3]`）は足さない。修飾の影響は 1 段だけ見る（別の欄を通る参照は見ない）
    fn pkg_field_dims(
        &self,
        cid: ClassId,
        field: &str,
        over: Option<Bind>,
        modified: &[&String],
    ) -> Option<Vec<usize>> {
        let ctx = Rc::new(Ctx {
            pfx: PKG.to_string(),
            lineage: self.p.lineage(cid),
            locals: Vec::new(),
            cenv: Cenv::default(),
            heir: None,
            inst: None,
        });
        let mut items = Vec::new();
        self.collect(cid, &ModMap::new(), &ctx, &mut items, 0)
            .ok()?;
        items.iter().find_map(|it| {
            let Item::Decl { cc, file, mods, .. } = it else {
                return None;
            };
            let d = cc
                .declarations()
                .into_iter()
                .find(|d| d.name().is_some_and(|(n, _)| n == field))?;
            let own = self
                .mod_of(
                    d.syntax().children().find(|c| c.kind() == K::Modification),
                    *file,
                    &ctx,
                    false,
                )
                .ok()?;
            let mut m = match mods.get(field) {
                Some(o) => merge_mod(o, &own),
                None => own,
            };
            if mentions(d.syntax(), modified)
                || (over.is_none() && m.val.as_ref().is_some_and(|b| mentions(&b.node, modified)))
            {
                return None;
            }
            m.val = over.clone().or(m.val);
            self.decl_dims(&ctx, *file, cc, &d, field, &m).ok()
        })
    }

    /// まだ平らにしていない部品 `tgt` の欄 `field` の値（S329）。
    /// shortcut: 既定値が他の欄を参照していれば評価しない（`pkg_env` の文脈は部品の欄を知らない）
    fn pending_field(&self, tgt: &str, field: &str, d: u32) -> Option<Val> {
        let b = self.pending_bind(tgt, field)?;
        if !b.suffix.is_empty() || b.lit.is_some() {
            return None;
        }
        self.eval(&b.node, b.file, &b.ctx, d)
    }

    /// レコードの束縛 `r = other` を欄に分けたもの（`suffix` が `.f`）の、`other.f` 自身の束縛。
    /// `other` が平らになっていれば環境から、宣言が後ろでまだなら `pending_bind`（`.f` が 1 段のとき）
    fn field_bind(&self, b: &Bind) -> Option<Bind> {
        let Some(Expr::ComponentRef(r)) = Expr::cast(b.node.clone()) else {
            return None;
        };
        // 欄 `.f`、またはレコードの配列の要素の欄 `[1,1].f`（S367）
        let elem = b.suffix.starts_with('[') && b.suffix.contains("].");
        if !(b.suffix.starts_with('.') || elem)
            || b.node.children().any(|x| x.kind() == K::Subscripts)
        {
            return None;
        }
        let t = self.cref(&r, b.file, &b.ctx).ok()?;
        if let Some(x) = self.env.get(&format!("{t}{}", b.suffix)) {
            return Some(x.clone());
        }
        self.pending_bind(&t, b.suffix.strip_prefix('.')?)
    }

    fn is_enum(&self, cid: ClassId) -> bool {
        self.p
            .class(cid)
            .and_then(|c| c.short_body())
            .is_some_and(|sb| tokens(sb.syntax()).any(|t| t.kind() == K::KwEnumeration))
    }

    /// パッケージの定数の束縛。差し替え後のパッケージ（`ctx` のパッケージ文脈か `heir`）が `extends` の修飾などで決めた値を先に見る
    fn pkg_binding(
        &self,
        ctx: &Rc<Ctx>,
        cid: ClassId,
        comp: &str,
        via: Option<ClassId>,
    ) -> Option<(SyntaxNode, u32, Rc<Ctx>)> {
        // 参照の名前が通ったパッケージ（`P5.n`）が先
        let h = via.or(if ctx.pfx == PKG {
            ctx.lineage.first().copied()
        } else {
            ctx.heir
        });
        // 差し替え後のパッケージの `extends` の修飾が効くのは、そのパッケージの系統の定数だけ（別のパッケージの同じ名前の定数に当てない。
        // `K.small = Svc.Machine.small` を評価するときに `Svc.Machine.small` が `K.small` を引いて輪になる）
        if let Some(b) = h
            .filter(|h| self.p.lineage(*h).contains(&cid))
            .and_then(|h| self.pkg_env(h))
            .and_then(|e| e.get(comp).cloned())
        {
            return Some((b.node, b.file, b.ctx));
        }
        let (n, f, c, _) = self.pkg_const(cid, comp)?;
        Some((n, f, c))
    }

    /// `Medium.name`（`Medium` がこのインスタンスで差し替えられた package）の定数の束縛。
    /// 外側の `None` は差し替えられた package の定数ではない（静的に引く）、内側の `None` は差し替えられているが決まらない。
    /// `redeclare package P = PP(n = 5)` の `(…)` の修飾は、`n` はその束縛、`n` に届く定数（`reaches_mod`）は決まらない（S377）
    fn dyn_const(
        &self,
        c: &ast::ComponentRef,
        ctx: &Rc<Ctx>,
    ) -> Option<Option<(SyntaxNode, u32, Rc<Ctx>)>> {
        let parts = c.parts();
        if parts.len() != 2
            || c.is_global()
            || c.syntax().children().any(|x| x.kind() == K::Subscripts)
        {
            return None;
        }
        let (cid, extra) = ctx.cenv.borrow().get(&parts[0])?.clone();
        let env = self.pkg_env(cid)?;
        let name = &parts[1];
        if let Some(m) = extra.get(name) {
            return Some(
                (m.sub.is_empty() && m.redecl.is_none())
                    .then(|| m.val.clone())
                    .flatten()
                    .map(|b| (b.node, b.file, b.ctx)),
            );
        }
        let b = env.get(name)?.clone();
        if self.reaches_mod(&env, name, &extra, &mut Vec::new()) {
            return Some(None);
        }
        Some(Some((b.node, b.file, b.ctx)))
    }

    /// パッケージの定数 `name` の束縛の式が、`(…)` の修飾 `extra` の名前を含むか（束縛をたどった先の定数も）、関数を呼ぶ（本体が読むかもしれない）か
    fn reaches_mod(
        &self,
        env: &HashMap<String, Bind>,
        name: &str,
        extra: &ModMap,
        seen: &mut Vec<String>,
    ) -> bool {
        if extra.is_empty() || seen.iter().any(|s| s == name) {
            return false;
        }
        seen.push(name.to_string());
        let Some(b) = env.get(name) else { return false };
        b.node.descendants().any(|x| {
            let call = ast::CallExpr::cast(x.clone()).is_some_and(|c| {
                !c.callee().is_some_and(|f| {
                    BUILTINS.contains(&f.as_str()) || matches!(f.as_str(), "size" | "max" | "min")
                })
            });
            call || tokens(&x).any(|t| {
                t.kind() == K::Ident
                    && (extra.contains_key(t.text())
                        || self.reaches_mod(env, t.text(), extra, seen))
            })
        })
    }

    /// パッケージ `cid`（`extends` の修飾も合わせた）の定数の束縛。束縛のあるものだけ
    fn pkg_env(&self, cid: ClassId) -> PkgEnv {
        if let Some(e) = self.pkg_envs.borrow().get(&cid) {
            return e.clone();
        }
        // 輪の予防: 作っている間は空として答える
        self.pkg_envs.borrow_mut().insert(cid, None);
        let ctx = Rc::new(Ctx {
            pfx: PKG.to_string(),
            lineage: self.p.lineage(cid),
            locals: Vec::new(),
            cenv: Cenv::default(),
            heir: None,
            inst: None,
        });
        let mut items = Vec::new();
        let collected = self.collect(cid, &ModMap::new(), &ctx, &mut items, 0);
        self.pkg_cenvs.borrow_mut().insert(cid, ctx.cenv.clone());
        let env = collected.ok().map(|()| {
            let mut map: HashMap<String, Bind> = HashMap::new();
            for it in &items {
                let Item::Decl { cc, file, mods, .. } = it else {
                    continue;
                };
                for d in cc.declarations() {
                    let Some((name, _)) = d.name() else { continue };
                    let own = self.mod_of(
                        d.syntax().children().find(|c| c.kind() == K::Modification),
                        *file,
                        &ctx,
                        false,
                    );
                    let Ok(own) = own else { continue };
                    let m = match mods.get(&name) {
                        Some(o) => merge_mod(o, &own),
                        None => own,
                    };
                    if let Some(b) = m.val {
                        map.entry(name).or_insert(b);
                    }
                }
            }
            Rc::new(map)
        });
        self.pkg_envs.borrow_mut().insert(cid, env.clone());
        env
    }

    /// パッケージの定数 `constant T name = 式` の式と、それを評価する文脈
    fn pkg_const(&self, cid: ClassId, name: &str) -> Option<(SyntaxNode, u32, Rc<Ctx>, bool)> {
        let file = self.p.lib.classes[cid].file;
        for e in self.p.class(cid)?.elements() {
            let Element::Component(cc) = e else { continue };
            let is_const = tokens(cc.syntax())
                .map(|t| t.kind())
                .take_while(|k| !matches!(k, K::Ident | K::Dot))
                .any(|k| k == K::KwConstant);
            if !is_const {
                continue;
            }
            for d in cc.declarations() {
                if d.name().is_some_and(|(n, _)| n == name) {
                    let m = d
                        .syntax()
                        .children()
                        .find(|c| c.kind() == K::Modification)?;
                    let node = m.children().find(|c| Expr::cast(c.clone()).is_some())?;
                    let ctx = Rc::new(Ctx {
                        pfx: PKG.to_string(),
                        lineage: self.p.lineage(cid),
                        locals: Vec::new(),
                        cenv: Cenv::default(),
                        heir: None,
                        inst: None,
                    });
                    let scalar = !d
                        .syntax()
                        .children()
                        .chain(cc.syntax().children())
                        .any(|c| c.kind() == K::Subscripts);
                    return Some((node, file, ctx, scalar));
                }
            }
        }
        None
    }

    /// ユーザー定義の関数の呼び出しの結果の形: 出力が 1 つで、その大きさが定数か整数の引数で決まるとき
    /// （`Frames.angularVelocity2` は `Real w[3]`、`relNodePositions(12)` は `xsi[nNodes]`）。
    /// スカラーの出力・レコードの出力は `None`（今までと同じく 1 つと数える）
    fn fn_out_dims(&self, c: &ast::CallExpr, file: u32, ctx: &Rc<Ctx>) -> Option<Vec<usize>> {
        let r = c.syntax().children().find_map(ast::ComponentRef::cast)?;
        let (rid, _) = self.call_target(&r, file, ctx)?;
        self.class_out_dims(rid, ctx, Some((c, file)))
    }

    /// 関数 `rid` の入力のうち整数のものを、呼び出し `call` の引数（無ければ既定値）の値に束ねた文脈（`pfx` は `FN`）
    fn fn_frame(&self, rid: ClassId, call: (&ast::CallExpr, u32), ctx: &Rc<Ctx>) -> Rc<Ctx> {
        let (c, file) = call;
        let ffile = self.p.lib.classes[rid].file;
        let mk = |locals: Vec<(String, i64)>| {
            Rc::new(Ctx {
                pfx: FN.to_string(),
                lineage: self.p.lineage(rid),
                locals,
                cenv: ctx.cenv.clone(),
                heir: ctx.heir,
                inst: ctx.inst().cloned(),
            })
        };
        let (pos, named) = (c.args(), c.named_args());
        let mut fctx = mk(Vec::new());
        let inputs = self.p.class(rid).into_iter().flat_map(|cd| cd.elements());
        let mut i = 0;
        for e in inputs {
            let Element::Component(cc) = e else { continue };
            if !tokens(cc.syntax()).any(|t| t.kind() == K::KwInput) {
                continue;
            }
            for d in cc.declarations() {
                let Some((name, _)) = d.name() else { continue };
                let given = named
                    .iter()
                    .find(|(k, _)| *k == name)
                    .map(|(_, e)| e)
                    .or(pos.get(i));
                i += 1;
                let v = match given {
                    Some(a) => self.eval(a.syntax(), file, ctx, 0),
                    None => d
                        .syntax()
                        .children()
                        .find(|c| c.kind() == K::Modification)
                        .and_then(|m| m.children().find(|c| Expr::cast(c.clone()).is_some()))
                        .and_then(|n| self.eval(&n, ffile, &fctx, 0)),
                };
                if let Some(v) = v.and_then(Val::int) {
                    let mut locals = fctx.locals.clone();
                    locals.push((name, v));
                    fctx = mk(locals);
                }
            }
        }
        fctx
    }

    /// 関数のクラスの、出力が 1 つのときの戻り値の大きさ（宣言の添字が定数のもの。`call` があれば整数の入力も）
    fn class_out_dims(
        &self,
        rid: ClassId,
        ctx: &Rc<Ctx>,
        call: Option<(&ast::CallExpr, u32)>,
    ) -> Option<Vec<usize>> {
        let cd = self.p.class(rid)?;
        let ffile = self.p.lib.classes[rid].file;
        let ctx = &match call {
            Some(c) => self.fn_frame(rid, c, ctx),
            None => ctx.clone(),
        };
        let mut outs = cd.elements().into_iter().filter_map(|e| match e {
            Element::Component(cc) if tokens(cc.syntax()).any(|t| t.kind() == K::KwOutput) => {
                Some(cc)
            }
            _ => None,
        });
        let cc = outs.next()?;
        let decls = cc.declarations();
        let [d] = decls.as_slice() else {
            return None;
        };
        if outs.next().is_some() {
            return None;
        }
        let mut dims = Vec::new();
        for sn in d
            .syntax()
            .children()
            .chain(cc.syntax().children())
            .filter(|c| c.kind() == K::Subscripts)
        {
            for e in sn.children().filter_map(Expr::cast) {
                let n = self.eval(e.syntax(), ffile, ctx, 0).and_then(Val::int)?;
                dims.push(usize::try_from(n).ok()?);
            }
        }
        (!dims.is_empty()).then_some(dims)
    }

    /// 束縛の配列の形。レコードの束縛 `r = other` を欄に分けたもの（`suffix` が `.f`）は `other.f` の大きさ
    fn bind_dims(&self, b: &Bind) -> Option<Vec<usize>> {
        // `cellData[1,1].OCV_SOC`（レコードの配列の要素の欄。S367）も、平らにした変数の大きさを引く
        if b.suffix.starts_with('.') || (b.suffix.starts_with('[') && b.suffix.contains("].")) {
            if let Some(Expr::ComponentRef(r)) = Expr::cast(b.node.clone())
                && let Ok(t) = self.cref(&r, b.file, &b.ctx)
                && let Some(v) = self.var_dims(&format!("{t}{}", b.suffix))
            {
                return Some(v);
            }
            let f = self.field_bind(b)?;
            return (f.suffix.is_empty() && f.lit.is_none())
                .then(|| self.dims_of(&f.node, f.file, &f.ctx))
                .flatten();
        }
        self.dims_of(&b.node, b.file, &b.ctx)
    }

    /// 式の配列の形（スカラーは空）。分からなければ `None`
    // shortcut: 戻り値の形を見る関数は、要素ごとの組み込み・`vector`・出力が 1 つで大きさが定数か整数の引数で決まるユーザー定義だけ。必要になったら足す
    fn dims_of(&self, n: &SyntaxNode, file: u32, ctx: &Rc<Ctx>) -> Option<Vec<usize>> {
        let ev = |x: &Expr| {
            self.eval(x.syntax(), file, ctx, 0)
                .and_then(Val::int)
                .and_then(|v| usize::try_from(v).ok())
        };
        match Expr::cast(n.clone())? {
            Expr::Literal(_) => Some(Vec::new()),
            Expr::Paren(p) => match p.items().as_slice() {
                [e] => self.dims_of(e.syntax(), file, ctx),
                _ => None,
            },
            Expr::Unary(u) => self.dims_of(u.operand()?.syntax(), file, ctx),
            Expr::Binary(b) => {
                let ops = b.ops();
                let mut it = b.operands().into_iter();
                let mut out = self.dims_of(it.next()?.syntax(), file, ctx)?;
                for (i, o) in it.enumerate() {
                    let d = self.dims_of(o.syntax(), file, ctx)?;
                    if ops.get(i).is_some_and(|t| t.kind() == K::Star)
                        && !out.is_empty()
                        && !d.is_empty()
                    {
                        // 行列の積（`*`）: ベクトルどうしは内積でスカラー
                        out = match (out.as_slice(), d.as_slice()) {
                            ([n], [m]) if n == m => Vec::new(),
                            ([n, k], [m]) if k == m => vec![*n],
                            ([n], [m, k]) if n == m => vec![*k],
                            ([n, k], [m, l]) if k == m => vec![*n, *l],
                            _ => return None,
                        };
                    } else if out.is_empty() {
                        out = d;
                    } else if !d.is_empty() && d != out {
                        return None;
                    }
                }
                Some(out)
            }
            Expr::If(i) => match self.const_if_expr(&i, file, ctx) {
                Some(e) => self.dims_of(e.syntax(), file, ctx),
                // 条件が決まらないときは最初の枝。ほかの枝の形が分かって食い違うなら決めない
                None => {
                    let first = self.dims_of(i.branches().first()?.1.syntax(), file, ctx)?;
                    let rest = i
                        .branches()
                        .into_iter()
                        .skip(1)
                        .map(|b| b.1)
                        .chain(i.else_branch());
                    rest.filter_map(|e| self.dims_of(e.syntax(), file, ctx))
                        .all(|d| d == first)
                        .then_some(first)
                }
            },
            // `end` はスカラー
            Expr::End(_) => Some(Vec::new()),
            // `{e for i in r}`: 範囲の長さ、そのあとに `e` の形
            Expr::Comprehension(c) => {
                let mut out = Vec::new();
                let fis = c.syntax().children().find(|x| x.kind() == K::ForIndices)?;
                for ix in fis.children().filter(|x| x.kind() == K::ForIndex) {
                    let r = ix.children().find_map(Expr::cast)?;
                    match self.dims_of(r.syntax(), file, ctx)?.as_slice() {
                        [n] => out.push(*n),
                        _ => return None,
                    }
                }
                out.extend(self.dims_of(
                    c.syntax().children().find_map(Expr::cast)?.syntax(),
                    file,
                    ctx,
                )?);
                Some(out)
            }
            Expr::Array(a) => {
                let items = a.items();
                if let [e @ Expr::Comprehension(_)] = items.as_slice() {
                    return self.dims_of(e.syntax(), file, ctx);
                }
                let mut v = vec![items.len()];
                v.extend(self.dims_of(items.first()?.syntax(), file, ctx)?);
                Some(v)
            }
            Expr::Matrix(m) => {
                // 行は `;`、行の中は `,` で区切る。要素はスカラー（1×1）・ベクトル（n×1）・行列（n×m）で、行の中は横に、行どうしは縦に連結する
                // （`[y] = [u1; u2; u3]` の `y`・`u1`… はベクトル）
                let (mut rows, mut cur) = (0, None::<(usize, usize)>);
                let mut width = None;
                for ch in m.syntax().children_with_tokens() {
                    if let Some(c) = ch.as_node().and_then(|c| Expr::cast(c.clone())) {
                        let d = self.dims_of(c.syntax(), file, ctx)?;
                        let (r, w) = match d.as_slice() {
                            [] => (1, 1),
                            [n] => (*n, 1),
                            [n, k] => (*n, *k),
                            _ => return None,
                        };
                        cur = match cur {
                            None => Some((r, w)),
                            Some((r0, w0)) if r0 == r => Some((r0, w0 + w)),
                            _ => return None,
                        };
                    } else if ch.as_token().is_some_and(|t| t.kind() == K::Semi) {
                        let (r, w) = cur.take()?;
                        if width.is_some_and(|w0| w0 != w) {
                            return None;
                        }
                        (rows, width) = (rows + r, Some(w));
                    }
                }
                let (r, w) = cur?;
                if width.is_some_and(|w0| w0 != w) {
                    return None;
                }
                Some(vec![rows + r, w])
            }
            Expr::Range(r) => {
                let v: Vec<f64> = r
                    .parts()
                    .iter()
                    .map(|e| self.eval(e.syntax(), file, ctx, 0).map(Val::f))
                    .collect::<Option<_>>()?;
                let (a, s, b) = match v.as_slice() {
                    [a, b] => (*a, 1.0, *b),
                    [a, s, b] if *s != 0.0 => (*a, *s, *b),
                    _ => return None,
                };
                Some(vec![(((b - a) / s).floor() + 1.0).max(0.0) as usize])
            }
            Expr::Call(c) => {
                let name = c.callee()?;
                let args = c.args();
                match name.rsplit('.').next()? {
                    "zeros" | "ones" => args.iter().map(ev).collect(),
                    "fill" => args.get(1..)?.iter().map(ev).collect(),
                    "identity" => Some(vec![ev(args.first()?)?; 2]),
                    "vector" => Some(vec![
                        (self.dims_of(args.first()?.syntax(), file, ctx)?)
                            .into_iter()
                            .fold(1, capped_mul),
                    ]),
                    "from_rpm" | "from_deg" | "to_deg" | "sqrt" | "abs" | "sin" | "cos" | "exp"
                    | "log" | "mod" | "rem" | "div" | "floor" | "ceil" | "integer" | "sign" => {
                        self.dims_of(args.first()?.syntax(), file, ctx)
                    }
                    _ => self.fn_out_dims(&c, file, ctx),
                }
            }
            Expr::ComponentRef(c) => {
                if n.children().any(|x| x.kind() == K::Subscripts) {
                    return self.indexed_dims(&c, file, ctx);
                }
                match &self.p.res_of(file, c.range())? {
                    Resolution::Local => Some(Vec::new()),
                    Resolution::Component(cid, idx)
                        if ctx.pfx == PKG || !ctx.lineage.contains(cid) =>
                    {
                        let comp = &self.p.lib.classes[*cid].components[*idx].name;
                        if c.parts().last() != Some(comp) {
                            return None;
                        }
                        if self.is_enum(*cid) {
                            return Some(Vec::new());
                        }
                        // `parameter` の欄（`pkg_env` にだけある）は定数でなくても束縛から
                        if self.pkg_const(*cid, comp).is_some_and(|c| c.3) {
                            return Some(Vec::new());
                        }
                        let via = self.via_class(file, &c, *cid);
                        let (node, f, c2) = self.pkg_binding(ctx, *cid, comp, via)?;
                        self.dims_of(&node, f, &c2)
                    }
                    Resolution::Component(..) => {
                        let t = self.cref(&c, file, ctx).ok()?;
                        self.var_dims(&t)
                    }
                    _ => None,
                }
            }
            _ => None,
        }
    }

    /// 添字のある参照 `x[1]`・`x[:, 2]` の形（このインスタンスの部品の、最後の欄の添字だけ）。
    /// スカラーの添字は次元を落とし、`:`・ベクトルの添字は残す
    fn indexed_dims(&self, c: &ast::ComponentRef, file: u32, ctx: &Rc<Ctx>) -> Option<Vec<usize>> {
        let sn = c
            .syntax()
            .children()
            .last()
            .filter(|x| x.kind() == K::Subscripts)?;
        let Some(Resolution::Component(cid, _)) = self.p.res_of(file, c.range()) else {
            return None;
        };
        if ctx.pfx == PKG || ctx.pfx == FN || !ctx.lineage.contains(&cid) {
            return None;
        }
        let t = self.cref(c, file, ctx).ok()?;
        let base = self.var_dims(t.strip_suffix(&self.subs(&sn, file, ctx).ok()?)?)?;
        let (mut out, mut k) = (Vec::new(), 0);
        for ch in sn.children_with_tokens() {
            if ch.as_token().is_some_and(|t| t.kind() == K::Colon) {
                out.push(*base.get(k)?);
            } else if let Some(e) = ch.as_node().filter(|e| Expr::cast((*e).clone()).is_some()) {
                match self.dims_of(e, file, ctx)?.as_slice() {
                    [] => {}
                    [m] => out.push(*m),
                    _ => return None,
                }
            } else {
                continue;
            }
            k += 1;
        }
        out.extend(base.get(k..)?);
        Some(out)
    }

    // ---------------------------------------------------------------- 方程式

    fn section(&mut self, node: &SyntaxNode, file: u32, ctx: &Rc<Ctx>, lv: &mut Level) -> Res<()> {
        if node.kind() == K::AlgorithmSection {
            let t = self.text(node, file, ctx)?;
            self.algs.push(t);
            return Ok(());
        }
        let initial = tokens(node).any(|t| t.kind() == K::KwInitial);
        for e in node.children().filter(|c| c.kind() == K::Equation) {
            self.equation(&e, file, ctx, initial, lv)?;
        }
        Ok(())
    }

    fn push(&mut self, initial: bool, text: String, count: usize) {
        let e = Equation {
            text,
            count,
            origin: self.cur.clone(),
        };
        if initial {
            self.ieqs.push(e);
        } else {
            self.eqs.push(e);
        }
    }

    fn equation(
        &mut self,
        node: &SyntaxNode,
        file: u32,
        ctx: &Rc<Ctx>,
        initial: bool,
        lv: &mut Level,
    ) -> Res<()> {
        if self.eqs.len() + self.ieqs.len() + self.whens.len() + lv.conns.len() > MAX_EQS {
            return un("too-large", "too many equations");
        }
        self.cur = self.p.origin(file, node);
        let first = tokens(node).next().map(|t| t.kind());
        match first {
            Some(K::KwFor) => {
                let fis: Vec<SyntaxNode> = node
                    .children()
                    .find(|c| c.kind() == K::ForIndices)
                    .map(|c| c.children().filter(|x| x.kind() == K::ForIndex).collect())
                    .unwrap_or_default();
                self.for_eq(node, &fis, file, ctx, initial, lv)
            }
            Some(K::KwIf) if self.const_branch(node, file, ctx).is_some() => {
                // 条件がすべて定数に評価できる `if` は選んだ枝だけを平らにする（`connect` も入る）
                for e in self.const_branch(node, file, ctx).unwrap_or_default() {
                    self.equation(&e, file, ctx, initial, lv)?;
                }
                Ok(())
            }
            Some(K::KwIf) if self.defer_root_if(node, file, ctx, initial, lv)? => Ok(()),
            Some(K::KwIf) if self.defer_card_if(node, file, ctx, initial, lv)? => Ok(()),
            Some(K::KwIf | K::KwWhen) => {
                if node
                    .descendants()
                    .filter_map(ast::CallExpr::cast)
                    .any(|c| c.callee().as_deref() == Some("connect"))
                {
                    return un("connect-in-if-when", node.text().to_string());
                }
                let t = self.text(node, file, ctx)?;
                let c = self.count_eq(node, file, ctx);
                if first == Some(K::KwWhen) && !initial {
                    self.whens.push(Equation {
                        text: t,
                        count: c,
                        origin: self.cur.clone(),
                    });
                } else {
                    self.push(initial, t, c);
                }
                Ok(())
            }
            _ => {
                let es: Vec<Expr> = node.children().filter_map(Expr::cast).collect();
                match es.as_slice() {
                    [Expr::Call(c)] if c.callee().as_deref() == Some("connect") => {
                        let a = c.args();
                        let [Expr::ComponentRef(x), Expr::ComponentRef(y)] = a.as_slice() else {
                            return un("connect", c.syntax().text().to_string());
                        };
                        let (xs, ys) =
                            (self.conn_ends(x, file, ctx)?, self.conn_ends(y, file, ctx)?);
                        // 範囲の切り出し `a[n:-1:1]`・`b[1:n]` は要素ごとの接続にする。大きさが合わなければ断る（先頭の 1 本だけつなぐと誤りが別の接続になる。S372）
                        if xs.len() != ys.len() {
                            return un(
                                "connect",
                                format!(
                                    "{} and {} differ in size",
                                    x.syntax().text(),
                                    y.syntax().text()
                                ),
                            );
                        }
                        for (x, y) in xs.into_iter().zip(ys) {
                            let (x, y) =
                                (self.bridge_end(&ctx.pfx, x), self.bridge_end(&ctx.pfx, y));
                            if !(self.is_removed(&x) || self.is_removed(&y)) {
                                lv.conns.push((x, y, self.cur.clone()));
                            }
                        }
                    }
                    [e] => {
                        let t = self.text(e.syntax(), file, ctx)?;
                        self.push(initial, t, 0);
                    }
                    [l, r] => {
                        let t = format!(
                            "{} = {}",
                            self.text(l.syntax(), file, ctx)?,
                            self.text(r.syntax(), file, ctx)?
                        );
                        let c = self.eq_count(l, r, file, ctx);
                        self.push(initial, t, c);
                    }
                    _ => return un("equation-shape", node.text().to_string()),
                }
                Ok(())
            }
        }
    }

    /// `l = r` のスカラーの数（どちらかの形から。決まらなければ 1）
    fn eq_count(&self, l: &Expr, r: &Expr, file: u32, ctx: &Rc<Ctx>) -> usize {
        let by_dims = |x: &Expr| {
            self.dims_of(x.syntax(), file, ctx)
                .and_then(|d| dims_product(&d))
        };
        self.shape(l, file, ctx)
            .or_else(|| self.shape(r, file, ctx))
            .or_else(|| by_dims(l))
            .or_else(|| by_dims(r))
            .unwrap_or(1)
    }

    /// `if` の方程式の枝（条件、枝の方程式）の並び。`else` の条件は `None`
    fn if_branches(node: &SyntaxNode) -> Vec<(Option<SyntaxNode>, Vec<SyntaxNode>)> {
        let mut branches: Vec<(Option<SyntaxNode>, Vec<SyntaxNode>)> = Vec::new();
        let mut want_cond = false;
        for e in node.children_with_tokens() {
            match e {
                rowan::NodeOrToken::Token(t) => match t.kind() {
                    K::KwIf | K::KwElseif => {
                        branches.push((None, Vec::new()));
                        want_cond = true;
                    }
                    K::KwElse => branches.push((None, Vec::new())),
                    _ => {}
                },
                rowan::NodeOrToken::Node(c) if want_cond && Expr::cast(c.clone()).is_some() => {
                    if let Some(b) = branches.last_mut() {
                        b.0 = Some(c);
                    }
                    want_cond = false;
                }
                rowan::NodeOrToken::Node(c) if c.kind() == K::Equation => {
                    if let Some(b) = branches.last_mut() {
                        b.1.push(c);
                    }
                }
                _ => {}
            }
        }
        branches
    }

    /// `if` の方程式の条件を先頭から評価し、真になる枝（無ければ `else`、それも無ければ空）の方程式を返す。
    /// 途中で評価できない条件があれば `None`
    fn const_branch(&self, node: &SyntaxNode, file: u32, ctx: &Rc<Ctx>) -> Option<Vec<SyntaxNode>> {
        for (cond, eqs) in Self::if_branches(node) {
            let Some(c) = cond else { return Some(eqs) }; // else
            match self.eval(&c, file, ctx, 0)? {
                Val::B(true) => return Some(eqs),
                Val::B(false) => {}
                _ => return None,
            }
        }
        Some(Vec::new())
    }

    /// 条件に `[not] Connections.isRoot(N)`・`rooted(N)` を含む `if` の方程式は、枝ごとに平らにして `oc_ifs` に置く。
    /// 根の決定（`graph.rs`）のあとで、真になる最初の枝だけを残す（S333）。条件が定数の枝は先に決める。
    /// 置けない形（ほかの条件が混ざる・`connect` を含む）なら false を返して今までの道に任せる
    fn defer_root_if(
        &mut self,
        node: &SyntaxNode,
        file: u32,
        ctx: &Rc<Ctx>,
        initial: bool,
        lv: &mut Level,
    ) -> Res<bool> {
        let mut branches = Vec::new();
        let mut any = false;
        for (cond, eqs) in Self::if_branches(node) {
            let Some(c) = cond else {
                branches.push((None, eqs));
                break;
            };
            match self.eval(&c, file, ctx, 0) {
                Some(Val::B(true)) => {
                    branches.push((None, eqs));
                    break;
                }
                Some(Val::B(false)) => {}
                Some(_) => return Ok(false),
                None => {
                    let t = self.text(&c, file, ctx)?;
                    let (not, t) = t
                        .strip_prefix("not ")
                        .map_or((false, t.as_str()), |r| (true, r));
                    let hit = [("Connections.isRoot", false), ("Connections.rooted", true)]
                        .into_iter()
                        .find_map(|(f, r)| graph::arg(t, f).map(|n| (r, not, n)));
                    let Some(h) = hit else { return Ok(false) };
                    any = true;
                    branches.push((Some(h), eqs));
                }
            }
        }
        if !any
            || node
                .descendants()
                .filter_map(ast::CallExpr::cast)
                .any(|c| c.callee().as_deref() == Some("connect"))
        {
            return Ok(false);
        }
        let at = if initial {
            self.ieqs.len()
        } else {
            self.eqs.len()
        };
        self.push(initial, String::new(), 0);
        let mut out = Vec::new();
        for (c, eqs) in branches {
            let (n0, k0) = (
                if initial {
                    self.ieqs.len()
                } else {
                    self.eqs.len()
                },
                self.oc_ifs.len(),
            );
            for e in eqs {
                self.equation(&e, file, ctx, initial, lv)?;
            }
            if self.oc_ifs.len() != k0 {
                return un("nested-isroot-if", node.text().to_string());
            }
            let tgt = if initial {
                &mut self.ieqs
            } else {
                &mut self.eqs
            };
            out.push((c, tgt.split_off(n0)));
        }
        self.oc_ifs.push(graph::OcIf {
            initial,
            at,
            branches: out,
        });
        Ok(true)
    }

    /// 同じ階層の `inner outer` の名前で始まる `connect` の端は、外側の `inner` の側に付け替える（Rumoca の flat も同じ）
    fn bridge_end(&self, pfx: &str, n: String) -> String {
        if self.bridge.is_empty() {
            return n;
        }
        let Some(rest) = n.strip_prefix(pfx) else {
            return n;
        };
        let seg = rest.split(['.', '[']).next().unwrap_or_default();
        match self.bridge.get(&format!("{pfx}{seg}")) {
            Some(t) => format!("{t}{}", &rest[seg.len()..]),
            None => n,
        }
    }

    /// 条件に `cardinality(c)` を含む `if` の方程式は、枝ごとに平らにして 1 行の目印を置く。全部の `connect` がそろったあと
    /// （`resolve_card_ifs`）で、条件が真になる最初の枝だけを残す。置けない形（`connect` を含む・入れ子）なら false を返して今までの道に任せる
    fn defer_card_if(
        &mut self,
        node: &SyntaxNode,
        file: u32,
        ctx: &Rc<Ctx>,
        initial: bool,
        lv: &mut Level,
    ) -> Res<bool> {
        let has_card = |n: &SyntaxNode| {
            n.descendants()
                .filter_map(ast::CallExpr::cast)
                .any(|c| c.callee().as_deref() == Some("cardinality"))
        };
        let branches = Self::if_branches(node);
        if !branches
            .iter()
            .any(|(c, _)| c.as_ref().is_some_and(has_card))
            || node
                .descendants()
                .filter_map(ast::CallExpr::cast)
                .any(|c| c.callee().as_deref() == Some("connect"))
        {
            return Ok(false);
        }
        let marker = format!("\u{2}card{}", self.card_ifs.len());
        self.push(initial, marker.clone(), 0);
        let mut out = Vec::new();
        let k0 = self.card_ifs.len();
        for (c, eqs) in branches {
            let tgt_len = |f: &Self| if initial { f.ieqs.len() } else { f.eqs.len() };
            let n0 = tgt_len(self);
            for e in eqs {
                self.equation(&e, file, ctx, initial, lv)?;
            }
            if self.card_ifs.len() != k0 {
                return un("nested-cardinality-if", node.text().to_string());
            }
            let tgt = if initial {
                &mut self.ieqs
            } else {
                &mut self.eqs
            };
            out.push((c, tgt.split_off(n0)));
        }
        self.card_ifs.push(CardIf {
            marker,
            file,
            ctx: ctx.clone(),
            branches: out,
        });
        Ok(true)
    }

    /// `cardinality(c)` は、`c` が `connect` の端に出てきた数（MLS 3.7.2.3。Rumoca の flat も同じ）。それで条件を評価して枝を決める
    fn resolve_card_ifs(&mut self) -> Res<()> {
        if self.card_ifs.is_empty() {
            return Ok(());
        }
        let mut counts: HashMap<String, i64> = HashMap::new();
        for (a, b) in &self.conn_pairs {
            *counts.entry(a.clone()).or_insert(0) += 1;
            *counts.entry(b.clone()).or_insert(0) += 1;
        }
        self.card = Some(counts);
        self.eval_memo.borrow_mut().clear();
        let mut chosen: HashMap<String, Vec<Equation>> = HashMap::new();
        for f in std::mem::take(&mut self.card_ifs) {
            let mut pick = None;
            for (c, eqs) in f.branches {
                let hit = match &c {
                    None => true,
                    Some(c) => match self.eval(c, f.file, &f.ctx, 0) {
                        Some(Val::B(b)) => b,
                        _ => return un("cardinality", c.text().to_string()),
                    },
                };
                if hit {
                    pick = Some(eqs);
                    break;
                }
            }
            chosen.insert(f.marker, pick.unwrap_or_default());
        }
        self.card = None;
        self.eval_memo.borrow_mut().clear();
        for list in [&mut self.eqs, &mut self.ieqs] {
            if list.iter().any(|e| chosen.contains_key(&e.text)) {
                *list = std::mem::take(list)
                    .into_iter()
                    .flat_map(|e| chosen.remove(&e.text).unwrap_or_else(|| vec![e]))
                    .collect();
            }
        }
        Ok(())
    }

    /// 条件が偽で落とした部品（またはその中・要素）か
    fn is_removed(&self, s: &str) -> bool {
        self.removed.iter().any(|r| {
            s.strip_prefix(r.as_str())
                .is_some_and(|t| t.is_empty() || t.starts_with(['.', '[']))
        })
    }

    fn for_eq(
        &mut self,
        node: &SyntaxNode,
        fis: &[SyntaxNode],
        file: u32,
        ctx: &Rc<Ctx>,
        initial: bool,
        lv: &mut Level,
    ) -> Res<()> {
        let Some((fi, rest)) = fis.split_first() else {
            for e in node.children().filter(|c| c.kind() == K::Equation) {
                self.equation(&e, file, ctx, initial, lv)?;
            }
            return Ok(());
        };
        let name = tokens(fi)
            .find(|t| t.kind() == K::Ident)
            .map(|t| t.text().to_string());
        let range = fi.children().find(|c| Expr::cast(c.clone()).is_some());
        let (Some(name), Some(range)) = (name, range) else {
            return un("for-without-range", node.text().to_string());
        };
        let vals = self.range_vals(&range, file, ctx)?;
        for v in vals {
            self.iters += 1;
            if self.iters > MAX_ITERS {
                return un("too-large", "too many loop iterations");
            }
            let mut locals = ctx.locals.clone();
            locals.push((name.clone(), v));
            let c2 = Rc::new(Ctx {
                pfx: ctx.pfx.clone(),
                lineage: ctx.lineage.clone(),
                locals,
                cenv: ctx.cenv.clone(),
                heir: ctx.heir,
                inst: ctx.inst.clone(),
            });
            self.for_eq(node, rest, file, &c2, initial, lv)?;
        }
        Ok(())
    }

    /// 範囲の式 `r`（の部品 `n`）の整数の値
    fn range_int(&self, n: &SyntaxNode, r: &SyntaxNode, file: u32, ctx: &Rc<Ctx>) -> Res<i64> {
        self.eval(n, file, ctx, 0)
            .and_then(Val::int)
            .ok_or_else(|| crate::Unsupported {
                kind: "non-constant-for",
                detail: r.text().to_string(),
            })
    }

    /// 範囲 `a:b`・`a:s:b` の（始め, 刻み, 終わり）
    fn range_abc(
        &self,
        x: &ast::RangeExpr,
        r: &SyntaxNode,
        file: u32,
        ctx: &Rc<Ctx>,
    ) -> Res<(i64, i64, i64)> {
        let p = x.parts();
        let n: Vec<i64> = (p.iter())
            .map(|e| self.range_int(e.syntax(), r, file, ctx))
            .collect::<Res<_>>()?;
        match n.as_slice() {
            [a, b] => Ok((*a, 1, *b)),
            [a, s, b] if *s != 0 => Ok((*a, *s, *b)),
            _ => un("non-constant-for", r.text().to_string()),
        }
    }

    fn range_vals(&self, r: &SyntaxNode, file: u32, ctx: &Rc<Ctx>) -> Res<Vec<i64>> {
        let ev = |n: &SyntaxNode| self.range_int(n, r, file, ctx);
        let v: Vec<i64> = match Expr::cast(r.clone()) {
            Some(Expr::Range(x)) => {
                let (a, s, b) = self.range_abc(&x, r, file, ctx)?;
                let mut out = Vec::new();
                let mut i = a;
                while (s > 0 && i <= b) || (s < 0 && i >= b) {
                    out.push(i);
                    // 終わりが `i64::MAX` 近くだと最後の要素のあとの `i += s` が溢れる（debug は panic。S372）
                    let Some(next) = i.checked_add(s) else {
                        break;
                    };
                    i = next;
                    if out.len() > 100_000 {
                        return un("too-large", "for range");
                    }
                }
                out
            }
            Some(Expr::Array(a)) => a
                .items()
                .iter()
                .map(|e| ev(e.syntax()))
                .collect::<Res<_>>()?,
            _ => vec![ev(r)?],
        };
        Ok(v)
    }

    /// スカラーの数: 配列の変数（添字で減る）は大きさ、`array` 式は項の数、それ以外は 1
    fn shape(&self, e: &Expr, file: u32, ctx: &Rc<Ctx>) -> Option<usize> {
        match e {
            Expr::Paren(p) => match p.items().as_slice() {
                [x] => self.shape(x, file, ctx),
                // `(a, b) = f(x)` は出力のスカラーの数の和（配列の出力は大きさの分）
                items => Some(
                    items
                        .iter()
                        .map(|x| self.shape(x, file, ctx).unwrap_or(1))
                        .sum(),
                ),
            },
            Expr::Call(c) if c.callee().as_deref() == Some("der") => {
                self.shape(c.args().first()?, file, ctx)
            }
            Expr::Array(a) => Some(
                self.comp_len(a.syntax(), file, ctx)
                    .unwrap_or_else(|| a.items().len()),
            ),
            Expr::ComponentRef(c) => {
                let t = self.cref(c, file, ctx).ok()?;
                let (base, nsub) = match t.find('[') {
                    Some(i)
                        if t.ends_with(']') && !t[..i].contains('.')
                            || t.rfind('[') > t.rfind('.') =>
                    {
                        let i = t.rfind('[')?;
                        (t[..i].to_string(), t[i..].matches(',').count() + 1)
                    }
                    _ => (t.clone(), 0),
                };
                // 部品の配列を通した欄 `a.pin.v`（`pin[1].v` …）は、要素の数
                let Some(&vi) = self.var_idx.get(&base) else {
                    let Some(v) = self.array_elems(&t) else {
                        return self.record_array_len(&t);
                    };
                    let first = *self.var_idx.get(&v[0])?;
                    return Some(v.len() * self.vars[first].dims.iter().product::<usize>());
                };
                let dims = &self.vars[vi].dims;
                if t.contains(':') {
                    return None;
                }
                (dims.len() > nsub).then(|| dims[nsub..].iter().product())
            }
            _ => None,
        }
    }

    /// 内包 `{e for i in 1:n}` の要素の数（範囲の長さの積。内包でない・範囲が定数でなければ `None`）。S336
    fn comp_len(&self, a: &SyntaxNode, file: u32, ctx: &Rc<Ctx>) -> Option<usize> {
        let fis = a
            .children()
            .find(|c| c.kind() == K::Comprehension)?
            .children()
            .find(|c| c.kind() == K::ForIndices)?;
        fis.children()
            .filter(|x| x.kind() == K::ForIndex)
            .try_fold(1usize, |n, fi| {
                let r = fi.children().find(|c| Expr::cast(c.clone()).is_some())?;
                Some(capped_mul(n, self.range_vals(&r, file, ctx).ok()?.len()))
            })
    }

    /// 要素ごとに展開されたレコード・部品の配列 `t[k].x` の要素の数（`Complex aw[3]` の `aw = {…}` は 3 本。Rumoca と同じ。S337）。
    /// 変数名の表は整列しているので、`t[` で始まる名前の範囲だけをなめる
    fn record_array_len(&self, t: &str) -> Option<usize> {
        let pre = format!("{t}[");
        let ks: HashSet<&str> = self
            .var_idx
            .range(pre.clone()..)
            .map_while(|(n, _)| n.strip_prefix(&pre))
            .filter_map(|r| r.split_once("].").map(|k| k.0))
            .collect();
        (!ks.is_empty()).then_some(ks.len())
    }

    /// `a.pin.v` で `a.pin` が部品の配列のとき、変数として在る `a.pin[k].v`（S324）。変数そのもの・添字つきは対象外
    fn array_elems(&self, t: &str) -> Option<Vec<String>> {
        if t.ends_with(']') || self.var_idx.contains_key(t) {
            return None;
        }
        t.match_indices('.').map(|(i, _)| i).find_map(|i| {
            let at = |k: usize| format!("{}[{k}]{}", &t[..i], &t[i..]);
            self.var_idx.contains_key(&at(1)).then(|| {
                (1..)
                    .map(at)
                    .take_while(|x| self.var_idx.contains_key(x))
                    .collect()
            })
        })
    }

    /// `if` / `when` / `for` の方程式のスカラーの数（最初の枝。範囲が定数でない `for` は 1 回）
    fn count_eq(&self, node: &SyntaxNode, file: u32, ctx: &Rc<Ctx>) -> usize {
        let first = tokens(node).next().map(|t| t.kind());
        let mut total = 0;
        for ch in node.children_with_tokens() {
            if let Some(t) = ch.as_token()
                && matches!(t.kind(), K::KwElse | K::KwElseif | K::KwElsewhen)
            {
                break;
            }
            if let Some(c) = ch.as_node()
                && c.kind() == K::Equation
            {
                total += match tokens(c).next().map(|t| t.kind()) {
                    Some(K::KwIf | K::KwWhen | K::KwFor) => self.count_eq(c, file, ctx),
                    _ => {
                        let es: Vec<Expr> = c.children().filter_map(Expr::cast).collect();
                        match es.as_slice() {
                            [l, r] => self.eq_count(l, r, file, ctx),
                            _ => 0,
                        }
                    }
                };
            }
        }
        if first == Some(K::KwFor) {
            let n = node
                .children()
                .find(|c| c.kind() == K::ForIndices)
                .map(|fis| {
                    fis.children()
                        .filter(|x| x.kind() == K::ForIndex)
                        .map(|fi| {
                            fi.children()
                                .find(|c| Expr::cast(c.clone()).is_some())
                                .and_then(|r| self.range_vals(&r, file, ctx).ok())
                                .map_or(1, |v| v.len())
                        })
                        .fold(1, capped_mul)
                })
                .unwrap_or(1);
            return capped_mul(total, n);
        }
        total
    }

    /// expandable connector の欄の仕上げ（MLS 9.1.3。Rumoca の flat と同じ）: 全体の接続集合（階層ごとの集合を端の共有で束ねる）の端が
    /// すべて宣言済みの欄で、どれもほかの方程式・algorithm・束縛から読まれないなら、その集合の等式は作らない。
    /// 方程式を作った集合に入らなかった欄は `unused_expandable`（変数には残し、DAE が数えない）
    fn settle_expandable(&mut self) {
        fn find(p: &mut HashMap<usize, usize>, x: usize) -> usize {
            let q = *p.entry(x).or_insert(x);
            if q == x {
                return x;
            }
            let r = find(p, q);
            p.insert(x, r);
            r
        }
        if self.exp_vars.is_empty() {
            return;
        }
        let mut parent = HashMap::new();
        for set in &self.pot_sets {
            for &v in &set[1..] {
                let (a, b) = (find(&mut parent, set[0]), find(&mut parent, v));
                parent.insert(b, a);
            }
        }
        let mut refs = HashSet::new();
        let texts = (self.eqs.iter().chain(&self.whens).chain(&self.ieqs)).map(|e| e.text.as_str());
        let binds = self.vars.iter().filter_map(|v| v.binding.as_deref());
        for t in texts
            .chain(self.algs.iter().map(String::as_str))
            .chain(binds)
        {
            ref_paths(t, &mut refs);
        }
        // 読まれる端を 1 つでも持つ、または宣言済みの欄でない端を持つ集合（根の番号）
        let mut used = HashSet::new();
        for set in &self.pot_sets {
            for &v in set {
                let n = &self.vars[v].name;
                if !self.exp_vars.contains(&v) || refs.contains(n) || refs.contains(&strip_subs(n))
                {
                    used.insert(find(&mut parent, v));
                }
            }
        }
        for s in std::mem::take(&mut self.exp_sets) {
            if used.contains(&find(&mut parent, s.vars[0])) {
                self.conn_vars.extend(s.vars);
                self.eqs.extend(s.eqs);
            }
        }
        for &i in &self.exp_vars {
            self.vars[i].unused_expandable = !self.conn_vars.contains(&i);
        }
    }

    // ---------------------------------------------------------------- connect

    /// コネクタ `n` の葉の変数: `(接尾辞, 変数の添字)`。コネクタが 1 つの変数のとき、または配列の要素のときは接尾辞が空
    fn leaves(&self, n: &str) -> Vec<(String, usize, usize)> {
        let cnt = |i: usize| self.vars[i].dims.iter().product::<usize>();
        if let Some(&i) = self.var_idx.get(n) {
            return vec![(String::new(), i, cnt(i))];
        }
        let pre = format!("{n}.");
        let v: Vec<_> = self
            .var_idx
            .range(pre.clone()..)
            .take_while(|(k, _)| k.starts_with(&pre))
            .map(|(k, &i)| (k[n.len()..].to_string(), i, cnt(i)))
            .collect();
        if !v.is_empty() {
            return v;
        }
        // 配列の変数の要素 `u[2]`
        if let Some(b) = n.rfind('[')
            && n.ends_with(']')
            && let Some(&i) = self.var_idx.get(&n[..b])
            && self.vars[i].dims.len() == n[b..].matches(',').count() + 1
        {
            let nsub = n[b..].matches(',').count() + 1;
            return vec![(String::new(), i, self.vars[i].dims[nsub..].iter().product())];
        }
        Vec::new()
    }

    /// 端 `n` が、宣言していない expandable connector の欄か（接続が欄を足す。MLS 9.1.3。Rumoca も断る）
    fn adds_member(&self, n: &str) -> bool {
        n.rsplit_once('.')
            .is_some_and(|(p, _)| self.leaves(p).iter().any(|l| self.exp_vars.contains(&l.1)))
    }

    /// 端 `n` が、外側の階層（`pfx` の祖先）で宣言した `inner` の中なら、その `inner` を宣言する階層の前置き
    fn late_owner(&self, n: &str, pfx: &str) -> Option<String> {
        n.match_indices('.')
            .map(|(i, _)| &n[..i])
            .find(|k| self.inners.contains(*k))
            .map(|k| &k[..k.rfind('.').map_or(0, |i| i + 1)])
            .filter(|o| *o != pfx && pfx.starts_with(o))
            .map(str::to_string)
    }

    /// 配列のコネクタ・部品の配列を通した端 `a.b.p`（`a.b` が部品の配列）を、要素ごとの名前 `a.b[1].p`, `a.b[2].p` … にする（S324）。
    /// 葉が引けるならそのまま 1 本。配列の添字は 1 つの次元だけ（`[i, j]` は展開しない）
    /// 2 つ目は展開した配列の次元（`[k]` は `[k]`、`[i, j]` は `[行, 列]`。展開しなければ空。大きさの比べに使う。S372）
    fn expand_end(&self, n: &str, depth: u32) -> (Vec<String>, Vec<usize>) {
        if !self.leaves(n).is_empty() || depth > 3 {
            return (vec![n.to_string()], vec![]);
        }
        let cuts = n
            .match_indices('.')
            .map(|(i, _)| i)
            .chain(std::iter::once(n.len()));
        for i in cuts {
            if n[..i].ends_with(']') {
                continue;
            }
            let at = |ix: &str| format!("{}[{ix}]{}", &n[..i], &n[i..]);
            let ends = |ix: &str| self.expand_end(&at(ix), depth + 1).0;
            let live = |v: &[String]| v.iter().all(|x| !self.leaves(x).is_empty());
            if !ends("1").iter().all(|x| self.leaves(x).is_empty()) {
                let rows: Vec<Vec<String>> = (1..=MAX_VARS)
                    .map(|k| ends(&k.to_string()))
                    .take_while(|v| live(v))
                    .collect();
                let dims = vec![rows.len()];
                return (rows.into_iter().flatten().collect(), dims);
            }
            // 2 次元の配列 `cell[Ns, Np]`（行ごと）。大きさは `[1,1]` から添字を延ばして数える（S367）
            if !ends("1,1").iter().all(|x| self.leaves(x).is_empty()) {
                let count = |f: &dyn Fn(usize) -> String| {
                    (1..=MAX_VARS).take_while(|&k| live(&ends(&f(k)))).count()
                };
                let (a, b) = (count(&|k| format!("{k},1")), count(&|k| format!("1,{k}")));
                let names = (1..=a)
                    .flat_map(|x| (1..=b).map(move |y| format!("{x},{y}")))
                    .take(MAX_VARS)
                    .flat_map(|ix| ends(&ix))
                    .collect();
                return (names, vec![a, b]);
            }
        }
        (vec![n.to_string()], vec![])
    }

    /// 端が 1 つの配列の変数 `i[m]` で、相手が `m` 個の要素なら、`i[1]` … `i[m]` にする
    /// `other` は相手の展開した次元（2 次元どうしは行数・列数も合うこと。S372）
    fn split_array_var(&self, x: &[String], n: usize, other: &[usize]) -> Option<Vec<String>> {
        let [x] = x else { return None };
        let [(sf, vi, _)] = self.leaves(x).try_into().ok()?;
        let d = &self.vars[vi].dims;
        // 1 次元 `i[m]`、または 2 次元 `v[Ns, Np]`（部品の 2 次元の配列と行ごとに対応。S367）
        (n > 1
            && sf.is_empty()
            && !x.ends_with(']')
            && matches!(d.len(), 1 | 2)
            && d.iter().product::<usize>() == n
            && (other.len() != 2 || d.len() != 2 || d == other))
            .then(|| {
                idx_names(d)
                    .into_iter()
                    .map(|ix| format!("{x}{ix}"))
                    .collect()
            })
    }

    /// 1 つのインスタンスの `connect` から、potential の等式と flow の和（つながない内側の flow は 0）を作る。
    /// 接続集合は葉の変数（`a.heatPort.Q_flow`）の単位で作る。丸ごとの `connect(c, d)` と、欄だけの `connect(x, d.f)` は同じ集合に入る
    /// （Rumoca の flat も同じ）。`own` はこのインスタンスのコネクタの名前（外側の `connect` の端は、flow の和の符号が逆になる）、
    /// `prot` はそのうち `protected` のもの
    fn gen_connects(
        &mut self,
        pfx: &str,
        lv: &Level,
        inside: &[String],
        own: &[String],
        prot: &[String],
    ) -> Res<()> {
        // 葉の名前 → 番号（出てきた順）。葉ごとに（変数の添字、スカラーの数、外側のコネクタの端か）。集合は union-find
        fn find(p: &mut [usize], mut x: usize) -> usize {
            while p[x] != x {
                p[x] = p[p[x]];
                x = p[x];
            }
            x
        }
        // `n` か、`.`・`[` の手前までの前置きが、このインスタンスのコネクタか（表を引く。`own` の線形探索は O(N²)。S339）
        let own_set: HashSet<&str> = own.iter().map(String::as_str).collect();
        let outside = |n: &str| {
            own_set.contains(n)
                || n.match_indices(['.', '['])
                    .any(|(i, _)| own_set.contains(&n[..i]))
        };
        type Info = (usize, usize, bool);
        type St = (Vec<String>, Vec<Info>, Vec<usize>, BTreeMap<String, usize>);
        let id = |s: &mut St, name: String, i: Info| {
            if let Some(&k) = s.3.get(&name) {
                return k;
            }
            s.3.insert(name.clone(), s.0.len());
            s.0.push(name);
            s.1.push(i);
            s.2.push(s.2.len());
            s.0.len() - 1
        };
        let mut st: St = Default::default();
        // 外側の階層の `inner` につなぐ `connect` は、その `inner` を宣言する階層まで持ち越す。この階層が宣言するものは、ここで集合に入れる
        let (mine, rest): (Vec<_>, Vec<_>) = std::mem::take(&mut self.late_conns)
            .into_iter()
            .partition(|l| l.0 == pfx);
        self.late_conns = rest;
        let all: Vec<_> = lv
            .conns
            .iter()
            .map(|(a, b, o)| (a.clone(), b.clone(), o.clone(), None))
            .chain(mine.into_iter().map(|(_, a, b, o, f)| (a, b, o, Some(f))))
            .collect();
        // 配列のコネクタの一括の接続は、要素ごとの接続にしてから数える（S324）
        let mut conns = Vec::new();
        let mut flags = Vec::new();
        let mut deferred: Vec<(String, String)> = Vec::new();
        for (a, b, o, f) in &all {
            if let Some(owner) = self.late_owner(a, pfx).or_else(|| self.late_owner(b, pfx)) {
                let f = f.unwrap_or((outside(a), outside(b)));
                self.late_conns
                    .push((owner, a.clone(), b.clone(), o.clone(), f));
                deferred.push((a.clone(), b.clone()));
                continue;
            }
            let ((mut xa, da), (mut xb, db)) = (self.expand_end(a, 0), self.expand_end(b, 0));
            // 片方が部品の配列の要素ごとの端で、もう片方が配列の変数 1 つ（`connect(sensor.i, i)`、`i[m]`）なら、変数を要素にする
            if let Some(v) = self.split_array_var(&xa, xb.len(), &db) {
                xa = v;
            } else if let Some(v) = self.split_array_var(&xb, xa.len(), &da) {
                xb = v;
            }
            // 2 次元どうしは（行数, 列数）まで合わせる（全要素数だけでは `[2,3]` と `[3,2]` が通り、行優先で黙って対応づく。S372）
            if xa.len() != xb.len() || (da.len() == 2 && db.len() == 2 && da != db) {
                return un("connect", format!("{a} and {b} differ in size"));
            }
            flags.extend(std::iter::repeat_n(*f, xa.len()));
            conns.extend(xa.into_iter().zip(xb).map(|(x, y)| (x, y, o.clone())));
        }
        let lv = &Level { conns };
        self.conn_pairs
            .extend(lv.conns.iter().map(|(a, b, _)| (a.clone(), b.clone())));
        for (k, (a, b, _)) in lv.conns.iter().enumerate() {
            let la = self.leaves(a);
            if la.is_empty() {
                if self.adds_member(a) {
                    return un("expandable", format!("{a}: connection adds a member"));
                }
                return un(
                    "connect-array",
                    format!("unknown connector (or a whole array of connectors) {a}"),
                );
            }
            let lb = self.leaves(b);
            if lb.is_empty() && self.adds_member(b) {
                return un("expandable", format!("{b}: connection adds a member"));
            }
            let (oa, ob) = flags[k].unwrap_or((outside(a), outside(b)));
            for (suffix, vi, count) in &la {
                let Some((_, vj, _)) = lb.iter().find(|(s, _, _)| s == suffix) else {
                    let kind = if self.exp_vars.contains(vi) {
                        "expandable"
                    } else {
                        "connect"
                    };
                    return un(kind, format!("{a} and {b} differ"));
                };
                let ia = id(&mut st, format!("{a}{suffix}"), (*vi, *count, oa));
                let ib = id(&mut st, format!("{b}{suffix}"), (*vj, *count, ob));
                let (ra, rb) = (find(&mut st.2, ia), find(&mut st.2, ib));
                st.2[rb] = ra;
            }
        }
        let (names, info, mut parent, by_name) = st;
        let mut sets: Vec<Vec<usize>> = Vec::new();
        let mut root_of: HashMap<usize, usize> = HashMap::new();
        for i in 0..names.len() {
            let r = find(&mut parent, i);
            let k = *root_of.entry(r).or_insert_with(|| {
                sets.push(Vec::new());
                sets.len() - 1
            });
            sets[k].push(i);
        }
        // 集合の位置: その集合の端を最初に左辺に書いた `connect` の位置
        let mut origins: Vec<Option<Origin>> = vec![None; sets.len()];
        let mut seen = vec![false; sets.len()];
        for (a, _, o) in &lv.conns {
            // `a` そのもの、`a.…`、`a[…`（名前順の表の連続した範囲。接続数 × 葉の数にしない。S339）
            let hits = by_name.get(a).into_iter().copied().chain(
                [format!("{a}."), format!("{a}[")]
                    .into_iter()
                    .flat_map(|pre| {
                        by_name
                            .range(pre.clone()..)
                            .take_while(|(n, _)| n.starts_with(&pre))
                            .map(|(_, &i)| i)
                            .collect::<Vec<_>>()
                    }),
            );
            for i in hits {
                let k = root_of[&find(&mut parent, i)];
                if !seen[k] {
                    seen[k] = true;
                    origins[k] = o.clone();
                }
            }
        }
        for (k, set) in sets.iter().enumerate() {
            let origin = origins[k].clone();
            let (vi, count, _) = info[set[0]];
            let v = &self.vars[vi];
            let pending = !v.flow
                && !v.stream
                && !matches!(v.kind, VarKind::Parameter | VarKind::Constant)
                && set.iter().all(|&i| self.exp_vars.contains(&info[i].0));
            if !pending && !self.exp_vars.is_empty() {
                self.conn_vars.extend(set.iter().map(|&i| info[i].0));
            }
            // potential の集合は階層ごとに作るので、expandable の欄を含むものは覚えておき、あとで全体の集合に束ねる
            if !v.flow && !v.stream && set.iter().any(|&i| self.exp_vars.contains(&info[i].0)) {
                self.pot_sets.push(set.iter().map(|&i| info[i].0).collect());
            }
            if v.flow {
                let flip = set.iter().all(|&i| info[i].2);
                let mut s = String::new();
                for (j, &i) in set.iter().enumerate() {
                    // 外側のコネクタの端は符号が逆（内側の端が 1 つも無いときは全部そろえる）
                    let plus = !info[i].2 != flip;
                    let t = &names[i];
                    s += &match (j, plus) {
                        (0, true) => t.clone(),
                        (0, false) => format!("-{t}"),
                        (_, true) => format!(" + {t}"),
                        (_, false) => format!(" - {t}"),
                    };
                }
                self.eqs.push(Equation {
                    text: format!("{s} = 0"),
                    count,
                    origin,
                });
            } else {
                if v.stream {
                    // 大きさ 0 の配列の欄（`Xi_outflow[0]`）は端にしない。外側の端ごとに 1 本、`x = <集合の混合>`（MLS 15.2。本文は expand_streams が作る）
                    if count > 0 {
                        self.stream_sets
                            .push(set.iter().map(|&i| (names[i].clone(), info[i].2)).collect());
                        for &i in set.iter().filter(|&&i| info[i].2) {
                            let n = &names[i];
                            self.eqs.push(Equation {
                                text: format!("{n} = {OUT_STREAM}({n})"),
                                count,
                                origin: origin.clone(),
                            });
                        }
                    }
                    continue;
                }
                // コネクタの中の parameter・constant は等式にしない（Rumoca の flat も同じ）
                if matches!(v.kind, VarKind::Parameter | VarKind::Constant) {
                    continue;
                }
                // 端がすべて expandable connector の宣言済みの欄なら、ほかで読まれるまで方程式を保留する（`settle_expandable`）
                if pending {
                    self.exp_sets.push(ExpSet {
                        vars: set.iter().map(|&i| info[i].0).collect(),
                        eqs: set[1..]
                            .iter()
                            .map(|&i| Equation {
                                text: format!("{} = {}", names[set[0]], names[i]),
                                count,
                                origin: origin.clone(),
                            })
                            .collect(),
                    });
                    continue;
                }
                if set.len() > 1 {
                    self.oc_sets.push(graph::OcSet {
                        start: self.eqs.len(),
                        members: set.iter().map(|&i| names[i].clone()).collect(),
                    });
                }
                for &i in &set[1..] {
                    self.eqs.push(Equation {
                        text: format!("{} = {}", names[set[0]], names[i]),
                        count,
                        origin: origin.clone(),
                    });
                }
            }
        }
        // 持ち越した `connect` の端も、つながっているものとして数える（つながない内側の flow = 0 にしない）
        let connected: HashSet<String> = names
            .iter()
            .cloned()
            .chain(deferred.iter().flat_map(|(a, b)| [a, b]).flat_map(|x| {
                self.leaves(x)
                    .into_iter()
                    .map(move |(sf, ..)| format!("{x}{sf}"))
            }))
            .collect();
        // `protected` のコネクタは、丸ごと `connect` の端になっているか、どこにもつながっていない葉の flow が 0
        // （Rumoca の flat: 接続の端が「集合の中の単一の識別子」なら外側の flow、そうでなければ未接続の flow だけ。S312）
        let whole: HashSet<&str> = lv
            .conns
            .iter()
            .flat_map(|(a, b, _)| [a.as_str(), b.as_str()])
            .chain(deferred.iter().flat_map(|(a, b)| [a.as_str(), b.as_str()]))
            .collect();
        for n in prot {
            for (suffix, vi, count) in self.leaves(n) {
                let v = &self.vars[vi];
                if v.flow
                    && (whole.contains(n.as_str())
                        || !connected.contains(format!("{n}{suffix}").as_str()))
                {
                    self.eqs.push(Equation {
                        text: format!("{n}{suffix} = 0"),
                        count,
                        origin: v.origin.clone(),
                    });
                }
            }
        }
        // つながっていない内側のコネクタの flow は 0（葉の単位。一部の欄だけつないだコネクタの残りの flow も 0）
        for n in inside {
            for (suffix, vi, count) in self.leaves(n) {
                let v = &self.vars[vi];
                if v.flow && !connected.contains(format!("{n}{suffix}").as_str()) {
                    self.eqs.push(Equation {
                        text: format!("{n}{suffix} = 0"),
                        count,
                        origin: v.origin.clone(),
                    });
                }
            }
        }
        Ok(())
    }
}

/// 式の文字列に出てくる成分参照（`a.b[1].c`）を、そのままと添字を除いた形（`a.b.c`）で `out` に入れる
/// （shortcut: 引用符つきの名前 `'a b'` は 1 つの名前として見ない。expandable の欄が読まれたかの判定だけに使う）
fn ref_paths(t: &str, out: &mut HashSet<String>) {
    let b = t.as_bytes();
    let ident = |c: u8| c.is_ascii_alphanumeric() || c == b'_';
    let mut i = 0;
    while i < b.len() {
        if !(b[i].is_ascii_alphabetic() || b[i] == b'_') {
            i += 1;
            continue;
        }
        let st = i;
        let mut bare = String::new();
        loop {
            let s = i;
            while i < b.len() && ident(b[i]) {
                i += 1;
            }
            bare += &t[s..i];
            let mut depth = 0;
            while i < b.len() && (b[i] == b'[' || depth > 0) {
                depth += i32::from(b[i] == b'[') - i32::from(b[i] == b']');
                i += 1;
            }
            if i + 1 < b.len()
                && b[i] == b'.'
                && (b[i + 1].is_ascii_alphabetic() || b[i + 1] == b'_')
            {
                bare.push('.');
                i += 1;
            } else {
                break;
            }
        }
        out.insert(t[st..i].to_string());
        out.insert(bare);
    }
}

/// 名前から添字 `[…]` を除く
fn strip_subs(n: &str) -> String {
    let mut depth = 0;
    n.chars()
        .filter(|&c| {
            depth += i32::from(c == '[') - i32::from(c == ']');
            depth == 0 && c != ']'
        })
        .collect()
}

/// 要素がちょうど 1 つのとき、その要素
fn single<T>(mut it: impl Iterator<Item = T>) -> Option<T> {
    let x = it.next()?;
    it.next().is_none().then_some(x)
}

/// 次元の積。溢れたら `None`（宣言の次元は入力なので、検査なしの `product` は debug で panic・release で巻き戻る。S339）
fn dims_product(d: &[usize]) -> Option<usize> {
    d.iter().try_fold(1usize, |a, &x| a.checked_mul(x))
}

/// 数の積。`MAX_SCALARS` を超えたら `MAX_SCALARS + 1` に頭打ちにする（溢れず、あとのスカラーの総数の検査が断る。S365・#137）。
/// 宣言した変数の次元の積は `check_dims` が先に断るので、ここは変数から来ない積（内包の範囲・`vector`・`for` の反復）に使う
fn capped_mul(a: usize, b: usize) -> usize {
    a.saturating_mul(b).min(MAX_SCALARS + 1)
}

fn field_mod(b: Bind) -> Mod {
    Mod {
        val: Some(b),
        ..Mod::default()
    }
}

fn nest(map: &mut ModMap, path: &[&str], b: Bind) {
    let e = map.entry(path[0].to_string()).or_default();
    match path {
        [_] => e.val = Some(b),
        [_, rest @ ..] => nest(&mut e.sub, rest, b),
        [] => {}
    }
}

/// 分けられる値の形: `{…}`（`None` の中身）か、`fill(x, n…)` の `x`（次元が `nd` 個のときだけ）
fn split_kind(n: &SyntaxNode, nd: usize) -> Option<Option<SyntaxNode>> {
    match Expr::cast(n.clone())? {
        Expr::Array(_) => Some(None),
        Expr::Call(c) if c.callee().as_deref() == Some("fill") => {
            let a = c.args();
            (nd == 0 || a.len() == nd + 1).then(|| a.first().map(|e| e.syntax().clone()))
        }
        _ => None,
    }
}

/// 添字だけの接尾辞（`` か `[1]`・`[1,2]`。`.f` の続きは欄の接尾辞）
fn is_index(s: &str) -> bool {
    s.is_empty() || (s.starts_with('[') && s.find(']') == Some(s.len() - 1))
}

/// `Complex` の (実部, 虚部)。虚部 `None` は Real
type CxPart = (String, Option<String>);

/// 演算の被演算子の括弧（名前・数・添字はそのまま）
fn cx_par(s: &str) -> String {
    if s.chars().all(|c| c.is_alphanumeric() || "._[]".contains(c)) {
        s.to_string()
    } else {
        format!("({s})")
    }
}

/// `Complex` の式の長さの上限（実部と虚部の合計）。`*`・`/` は被演算子ごとに式が約 2 倍になるので、超えたら展開しない（S339）
const CX_MAX_LEN: usize = 64 * 1024;

/// MSL の `Complex` の `+ - * /`。Real の被演算子は虚部 0 として、0 の項は出さない（S329）
fn cx_bin(op: K, a: CxPart, b: CxPart) -> Option<CxPart> {
    let (p, q) = (cx_par(&a.0), cx_par(&b.0));
    let (ai, bi) = (a.1.as_deref().map(cx_par), b.1.as_deref().map(cx_par));
    let r = match (op, &ai, &bi) {
        (K::Plus | K::DotPlus, _, _) => (
            format!("{p} + {q}"),
            match (&ai, &bi) {
                (Some(x), Some(y)) => Some(format!("{x} + {y}")),
                (x, y) => x.clone().or(y.clone()),
            },
        ),
        (K::Minus | K::DotMinus, _, _) => (
            format!("{p} - {q}"),
            match (&ai, &bi) {
                (Some(x), Some(y)) => Some(format!("{x} - {y}")),
                (x, None) => x.clone(),
                (None, Some(y)) => Some(format!("-{y}")),
            },
        ),
        (K::Star | K::DotStar, None, None) => (format!("{p}*{q}"), None),
        (K::Star | K::DotStar, Some(x), None) => (format!("{p}*{q}"), Some(format!("{x}*{q}"))),
        (K::Star | K::DotStar, None, Some(y)) => (format!("{p}*{q}"), Some(format!("{p}*{y}"))),
        (K::Star | K::DotStar, Some(x), Some(y)) => (
            format!("{p}*{q} - {x}*{y}"),
            Some(format!("{p}*{y} + {x}*{q}")),
        ),
        (K::Slash | K::DotSlash, None, None) => (format!("{p}/{q}"), None),
        (K::Slash | K::DotSlash, Some(x), None) => (format!("{p}/{q}"), Some(format!("{x}/{q}"))),
        (K::Slash | K::DotSlash, _, Some(y)) => {
            let den = format!("({q}*{q} + {y}*{y})");
            match &ai {
                Some(x) => (
                    format!("({p}*{q} + {x}*{y})/{den}"),
                    Some(format!("(-{p}*{y} + {x}*{q})/{den}")),
                ),
                None => (format!("{p}*{q}/{den}"), Some(format!("-{p}*{y}/{den}"))),
            }
        }
        _ => return None,
    };
    (r.0.len() + r.1.as_ref().map_or(0, String::len) <= CX_MAX_LEN).then_some(r)
}

/// 大小。整数どうしは整数のまま（f64 を通すと 2^53 を超えて値が変わる。S374・#152）、文字列どうしは辞書順（S377）、ほかは f64。
/// 決まらないのは NaN と、文字列と文字列でないものの大小
fn ord(a: Val, b: Val, st: &Strs) -> Option<std::cmp::Ordering> {
    match (a, b) {
        (Val::I(x), Val::I(y)) => Some(x.cmp(&y)),
        (Val::S(x), Val::S(y)) => Some(st.cmp(x, y)),
        (Val::S(_), _) | (_, Val::S(_)) => None,
        _ => a.f().partial_cmp(&b.f()),
    }
}

fn binop(op: K, a: Val, b: Val, st: &Strs) -> Option<Val> {
    use Val::*;
    Some(match (op, a, b) {
        (K::Plus | K::DotPlus, I(x), I(y)) => I(x.checked_add(y)?),
        (K::Minus | K::DotMinus, I(x), I(y)) => I(x.checked_sub(y)?),
        (K::Star | K::DotStar, I(x), I(y)) => I(x.checked_mul(y)?),
        (K::Caret | K::DotCaret, I(x), I(y)) if (0..40).contains(&y) => I(x.checked_pow(y as u32)?),
        // 文字列の `+` は連結。ほかの算術・文字列と数の混ぜは決めない（hash の和の実数を返さない）
        (K::Plus, S(x), S(y)) => st.cat(x, y)?,
        _ if matches!((a, b), (S(_), _) | (_, S(_)))
            && !matches!(op, K::EqEq | K::Ne | K::Lt | K::Le | K::Gt | K::Ge) =>
        {
            return None;
        }
        (K::Plus | K::DotPlus, _, _) => R(a.f() + b.f()),
        (K::Minus | K::DotMinus, _, _) => R(a.f() - b.f()),
        (K::Star | K::DotStar, _, _) => R(a.f() * b.f()),
        (K::Slash | K::DotSlash, _, _) => R(a.f() / b.f()),
        (K::Caret | K::DotCaret, _, _) => R(a.f().powf(b.f())),
        (K::EqEq, S(x), S(y)) => B(x == y),
        (K::Ne, S(x), S(y)) => B(x != y),
        (K::EqEq, E(c, i), E(d, j)) => B(c == d && i == j),
        (K::Ne, E(c, i), E(d, j)) => B(c != d || i != j),
        (K::Lt, _, _) => B(ord(a, b, st)?.is_lt()),
        (K::Le, _, _) => B(ord(a, b, st)?.is_le()),
        (K::Gt, _, _) => B(ord(a, b, st)?.is_gt()),
        (K::Ge, _, _) => B(ord(a, b, st)?.is_ge()),
        (K::EqEq, _, _) => B(ord(a, b, st)?.is_eq()),
        (K::Ne, _, _) => B(ord(a, b, st)?.is_ne()),
        (K::KwAnd, B(x), B(y)) => B(x && y),
        (K::KwOr, B(x), B(y)) => B(x || y),
        _ => return None,
    })
}

const WORDS: &[&str] = &[
    "if", "then", "else", "elseif", "elsewhen", "and", "or", "not", "in", "for", "loop", "when",
    "end", "while",
];

/// 字句を 1 つの空白で結ぶ（括弧の内側・`,`・`.`・関数の呼び出しの `(` の前には空白を置かない）
fn join(l: &[String]) -> String {
    let mut s = String::new();
    for (i, t) in l.iter().enumerate() {
        if i > 0 {
            let p = l[i - 1].as_str();
            let call = matches!(t.as_str(), "(" | "[")
                && p.chars()
                    .last()
                    .is_some_and(|c| c.is_alphanumeric() || matches!(c, '_' | ']' | ')'))
                && !WORDS.contains(&p);
            let tight = matches!(t.as_str(), ")" | "]" | "}" | "," | ";" | ".")
                || matches!(p, "(" | "[" | "{" | ".");
            if !(tight || call) {
                s.push(' ');
            }
        }
        s.push_str(t);
    }
    s
}

#[cfg(test)]
mod tests {
    use crate::Project;

    #[test]
    fn nested_for_equation_count_is_capped() {
        // #126: 入れ子の `for` の積に上限が無く、2000×2000 で 400 万本・3 重で終わらなかった（試験の上限は 1 万本）
        let src = "model M Real x[2];
            equation for i in 1:200 loop for j in 1:200 loop x[1] = x[2] + i + j; end for; end for;
            end M;";
        let e = Project::from_sources([("t.mo", src)])
            .flatten("M")
            .expect_err("too many");
        assert_eq!(e.kind, "too-large");
        // 上限に届かない積は通る
        let src = src.replace("1:200 loop for j in 1:200", "1:50 loop for j in 1:50");
        let m = Project::from_sources([("t.mo", src.as_str())])
            .flatten("M")
            .unwrap();
        assert_eq!(m.equation_count(), 2500);
    }

    #[test]
    fn nested_for_without_equations_is_capped() {
        // #136: 方程式を作らない入れ子の `for`（偽の `if` だけ）は `MAX_EQS` に届かず、1000×1000 で 9 s・10 万の 2 重で終わらなかった（試験の上限は 1 万回）
        let src = "model M Real x;
            equation x = 1;
            for i in 1:200 loop for j in 1:200 loop if false then x = 2; end if; end for; end for;
            end M;";
        let e = Project::from_sources([("t.mo", src)])
            .flatten("M")
            .expect_err("too many iterations");
        assert_eq!(e.kind, "too-large");
        // 空の本体も同じ
        let empty = src.replace("if false then x = 2; end if;", "");
        let e = Project::from_sources([("t.mo", empty.as_str())])
            .flatten("M")
            .expect_err("too many iterations");
        assert_eq!(e.kind, "too-large");
        // 上限に届かない回数は通る
        let src = src.replace("1:200 loop for j in 1:200", "1:50 loop for j in 1:50");
        assert!(
            Project::from_sources([("t.mo", src.as_str())])
                .flatten("M")
                .is_ok()
        );
    }

    #[test]
    fn record_array_comprehension_binding_picks_range_elements_directly() {
        // #138: `R r[n] = {R(k, 2*k) for k in 1:n}` が要素ごとに範囲全体を作り直し、n = 40000 で 22 s（O(n²)）だった
        let rec = "record R Real a; Real b; end R;";
        let flat = |src: &str| Project::from_sources([("t.mo", src)]).flatten("M");
        let bound = |m: &crate::FlatModel| {
            let b = |n: &str| {
                m.vars
                    .iter()
                    .find(|v| v.name == n)
                    .and_then(|v| v.binding.clone())
            };
            (b("r[1].a"), b("r[2].a"), b("r[3].a"), b("r[3].b"))
        };
        // 刻みのある範囲・降順の範囲も、i 番目の値を直接出す
        let m = flat(&format!(
            "{rec} model M R r[3] = {{R(k, 2*k) for k in 1:2:5}}; end M;"
        ))
        .unwrap();
        assert_eq!(
            bound(&m),
            (
                Some("1".into()),
                Some("3".into()),
                Some("5".into()),
                Some("2 * 5".into())
            )
        );
        let m = flat(&format!(
            "{rec} model M R r[3] = {{R(k, 2*k) for k in 5:-2:1}}; end M;"
        ))
        .unwrap();
        assert_eq!(bound(&m).0.as_deref(), Some("5"));
        // 範囲より多い要素は断る
        let e = flat(&format!(
            "{rec} model M R r[4] = {{R(k, 2*k) for k in 1:3}}; end M;"
        ))
        .unwrap_err();
        assert_eq!(e.kind, "composite-binding");
        // 宣言だけ（束縛なし）との比で見る（n = 40000 で前は 8.7 倍、今は 2.1 倍。机の速さによらない）
        let time = |decl: &str| {
            let src = format!("{rec} model M R r[40000]{decl}; end M;");
            let t = std::time::Instant::now();
            assert!(flat(&src).is_ok());
            t.elapsed()
        };
        let plain = time("");
        let with = time(" = {R(k, 2*k) for k in 1:40000}");
        assert!(with * 2 < plain * 9, "{with:?} vs {plain:?}");
    }

    #[test]
    fn expandable_connector_drops_unused_members() {
        // S367: 欄は全部変数に残る（Rumoca の flat と同じ）が、端がすべて宣言済みの欄で、どこからも読まれない集合は等式を作らず、
        // 方程式を作った集合に入らない欄は `unused_expandable`（DAE が数えない）。実の端（`s.y`）につながる欄 `a` は残す
        let src = |extra: &str| {
            format!(
                "connector Out = output Real;
                expandable connector Bus Real a; Real b; Real c; end Bus;
                model Src Out y; equation y = time; end Src;
                model Sub Bus bus; Src s; equation connect(s.y, bus.a); end Sub;
                model M Bus bus; Bus other; Sub p; Real q;
                equation connect(p.bus, bus); connect(bus, other); {extra} end M;"
            )
        };
        let flat = |extra: &str| {
            Project::from_sources([("t.mo", src(extra).as_str())])
                .flatten("M")
                .unwrap()
        };
        let m = flat("q = 1;");
        let unused: Vec<&str> = (m.vars.iter())
            .filter(|v| v.unused_expandable)
            .map(|v| v.name.as_str())
            .collect();
        assert_eq!(
            unused,
            ["bus.b", "bus.c", "other.b", "other.c", "p.bus.b", "p.bus.c"]
        );
        assert_eq!(m.vars.len(), 11); // 3 つの Bus の 9 欄、p.s.y、q
        // 集合 a: p.s.y・p.bus.a・bus.a・other.a の 3 本、y = time、q = 1
        assert_eq!(m.equation_count(), 5);
        // `bus.b` を読む方程式があれば、その集合（bus.b・p.bus.b・other.b）も等式を作る
        let m = flat("q = bus.b;");
        assert_eq!(m.equation_count(), 7);
        assert!(
            !m.vars
                .iter()
                .any(|v| v.name == "bus.b" && v.unused_expandable)
        );
        // 宣言していない欄への接続は欄を足すので断る
        let e = Project::from_sources([(
            "t.mo",
            "connector Out = output Real;
            expandable connector Bus Real a; end Bus;
            model M Bus bus; Out y; equation connect(y, bus.z); end M;",
        )])
        .flatten("M")
        .unwrap_err();
        assert_eq!(e.kind, "expandable");
    }

    #[test]
    fn modelica_functions_are_evaluated_for_sizes() {
        // S370: 配列の大きさを決める関数（`for`・`while`・`if`・`break`・`return`・再帰・配列の入力・実数）を、定数の評価が実行する
        let funs = "function tri input Integer n; output Integer r; algorithm
              r := 0; for i in 1:n loop r := r + i; end for; end tri;
            function collatz input Integer n; output Integer steps; protected Integer m = n; algorithm
              steps := 0;
              while true loop
                if m == 1 then break; elseif mod(m, 2) == 0 then m := div(m, 2); else m := 3*m + 1; end if;
                steps := steps + 1;
              end while;
            end collatz;
            function fact input Integer n; output Integer r; algorithm
              r := 1; if n <= 1 then return; end if; r := n*fact(n - 1); end fact;
            function cnt input Boolean b[:]; output Integer k = 0; algorithm
              for i in 1:size(b, 1) loop if b[i] then k := k + 1; end if; end for; end cnt;
            function up input Real x; input Integer k = 2; output Integer r; protected Real y[2]; algorithm
              y[1] := x*k; y[2] := sqrt(y[1]); r := integer(ceil(y[1] + max(y))); end up;";
        let dims = |body: &str| {
            let src = format!("{funs} model M {body} end M;");
            let m = Project::from_sources([("t.mo", src.as_str())]).flatten("M");
            m.map(|m| m.vars.iter().map(|v| v.dims.clone()).collect::<Vec<_>>())
        };
        assert_eq!(
            dims("Real a[tri(4)]; Real b[collatz(6)]; Real c[fact(4)]; Real d[cnt({true, false, true})];")
                .unwrap(),
            [vec![10], vec![8], vec![24], vec![2]]
        );
        // 実数の入力・既定値・名前つきの引数・出力の最後の `max`: ceil(2.5*2 + max(5, sqrt 5)) = 10、k = 3 なら ceil(7.5 + 7.5) = 15
        assert_eq!(
            dims("Real a[up(2.5)]; Real b[up(2.5, k=3)];").unwrap(),
            [vec![10], vec![15]]
        );
        // 再帰は 16 段まで（深さ 15 の呼び出しは 16 段）
        let rec = "function dep input Integer n; output Integer r; algorithm if n <= 0 then r := 1; else r := dep(n - 1) + 1; end if; end dep;";
        let sized = |n: usize| {
            let src = format!("{rec} model M Real x[dep({n})]; end M;");
            Project::from_sources([("t.mo", src.as_str())])
                .flatten("M")
                .map(|m| m.vars[0].dims[0])
        };
        assert_eq!(sized(15).unwrap(), 16);
        assert_eq!(sized(16).unwrap_err().kind, "non-constant-dim");
        // 評価できない（外部関数・偽の assert・暴走・終わらない再帰）は、今までどおり大きさが定数でないと断る
        let bad = |fun: &str, call: &str| {
            let src = format!("{fun} model M Real x[{call}]; end M;");
            let e = Project::from_sources([("t.mo", src.as_str())])
                .flatten("M")
                .unwrap_err();
            e.kind
        };
        let head = "function f input Integer n; output Integer r;";
        for (fun, call) in [
            (format!("{head} external \"C\"; end f;"), "f(1)"),
            (
                format!("{head} algorithm assert(n > 5, \"small\"); r := n; end f;"),
                "f(1)",
            ),
            (
                format!("{head} algorithm r := n; while true loop r := r + 1; end while; end f;"),
                "f(1)",
            ),
            (format!("{head} algorithm r := f(n + 1); end f;"), "f(1)"),
            (
                format!(
                    "{head} algorithm r := 1; for i in 1:100000000 loop r := r + 1; end for; end f;"
                ),
                "f(1)",
            ),
        ] {
            assert_eq!(bad(&fun, call), "non-constant-dim", "{fun}");
        }
    }

    #[test]
    fn scalar_total_is_capped() {
        // #139: 配列は 1 つと数えるので `Real x[10000000]` が通り、DAE の構築が 4 GB・92 s を使った。#137: 次元の積の溢れは panic していた
        let kind = |body: &str| {
            let src = format!("model M {body} end M;");
            let r = Project::from_sources([("t.mo", src.as_str())]).flatten("M");
            r.err().map(|e| e.kind)
        };
        let big = Some("too-large");
        assert_eq!(
            kind("Real x[10000000]; Real y[10000000]; equation x = y;"),
            big
        );
        // 1 つ 2000000 に収まっても、総数が上限を超える
        assert_eq!(kind("Real x[1500000]; Real y[1500000];"), big);
        assert_eq!(
            kind("Real x[1000, 1000]; equation {1 for i in 1:3000, j in 1:3000} = x;"),
            big
        );
        // 次元の積が usize を溢れる
        assert_eq!(
            kind("Real x[4294967296, 4294967296]; equation der(x) = x;"),
            big
        );
        assert_eq!(
            kind("Real x[4294967296, 4294967296] = zeros(4294967296, 4294967296);"),
            big
        );
        // 上限に届く大きさは通る
        assert_eq!(kind("Real x[1000000]; equation der(x) = -x;"), None);
    }

    #[test]
    fn products_of_sizes_do_not_overflow() {
        // #137: 内包の範囲・`vector`・`for` の反復の積は検査なしで、debug は panic・release は巻き戻っていた（頭打ちにして上限の検査が断る）
        let kind = |body: &str| {
            let src = format!("model M {body} end M;");
            let r = Project::from_sources([("t.mo", src.as_str())]).flatten("M");
            r.err().map(|e| e.kind)
        };
        let big = Some("too-large");
        let r4 = "i in 1:100000, j in 1:100000, k in 1:100000, l in 1:100000";
        assert_eq!(
            kind(&format!("Real x[2]; equation {{1 for {r4}}} = x;")),
            big
        );
        assert_eq!(
            kind("parameter Real y[:] = vector(zeros(4294967296, 4294967296));"),
            big
        );
        let body = "for i in 1:100000 loop for j in 1:100000 loop for k in 1:100000 loop for l in 1:100000 loop x[1] = 1; end for; end for; end for; end for;";
        assert_eq!(
            kind(&format!(
                "Boolean b; Real x[2]; equation when b then {body} end when;"
            )),
            big
        );
        // 小さい積は通る
        assert_eq!(
            kind("Real x[6]; equation {1 for i in 1:2, j in 1:3} = x;"),
            None
        );
    }

    #[test]
    fn range_ending_at_i64_max_does_not_overflow() {
        // #147: 終わりが `i64::MAX` に近い範囲は、最後の要素のあとの `i += s` が溢れて debug は panic だった（release は巻き戻って 10 万個の `too-large`）
        let src = "model M Real x[2]; equation
            for i in 9223372036854775800:9223372036854775807 loop x[1] = 1; end for;
            x[2] = 2; end M;";
        let m = Project::from_sources([("t.mo", src)]).flatten("M").unwrap();
        assert_eq!(m.equation_count(), 9);
        // 刻みが大きくて越える `a:s:b` も同じ
        let src = "model M Real x; equation for i in 1:4611686018427387904:9223372036854775807 loop x = 1; end for; end M;";
        let m = Project::from_sources([("t.mo", src)]).flatten("M").unwrap();
        assert_eq!(m.equation_count(), 2);
    }

    #[test]
    fn function_work_is_budgeted_across_calls() {
        // #148: 出力の束縛だけで再帰する関数は 1 度も tick されず、枝 4 本の `g(15)` は 4^15 回の呼び出しで終わらなかった。
        // #150: 呼び出しの予算は 1 回の flat で共有し（呼び出しごとに戻さない）、同じ引数の結果は覚える
        let kind = |src: &str| {
            let r = Project::from_sources([("t.mo", src)]).flatten("M");
            r.err().map(|e| e.kind)
        };
        let big = Some("non-constant-dim");
        // 引数が毎回違う（覚えても減らない）4 本の枝
        let g = "function g input Integer n; input Integer k; output Integer y = if n <= 0 then 1 else div(g(n - 1, 4*k + 1) + g(n - 1, 4*k + 2) + g(n - 1, 4*k + 3) + g(n - 1, 4*k + 4), 4); end g;";
        assert_eq!(kind(&format!("{g} model M Real x[g(15, 0)]; end M;")), big);
        // 同じ引数の枝は 1 度だけ評価する（16 回で済む）
        let h = "function h input Integer n; output Integer y = if n <= 0 then 1 else div(h(n - 1) + h(n - 1) + h(n - 1) + h(n - 1), 4); end h;";
        assert_eq!(kind(&format!("{h} model M Real x[h(15)]; end M;")), None);
        // 1 回が 1000 反復（約 2100）の関数を、引数を変えて 20 回呼ぶと予算（試験では 1 万）を超える。4 回なら収まる
        let w = "function w input Integer k; output Integer y; algorithm y := 0; for i in 1:1000 loop y := y + 1; end for; y := k; end w;";
        let uses = |n: usize| {
            let xs: String = (1..=n).map(|i| format!("Real x{i}[w({i})]; ")).collect();
            format!("{w} model M {xs} end M;")
        };
        assert_eq!(kind(&uses(4)), None);
        assert_eq!(kind(&uses(20)), big);
        // 同じ `parameter` を 20 か所で使うのは 1 回の評価
        let xs: String = (1..=20).map(|i| format!("Real a{i}[n]; ")).collect();
        assert_eq!(
            kind(&format!(
                "{w} model M parameter Integer n = w(2); {xs} end M;"
            )),
            None
        );
        // 配列の走査（`max(a)`）は要素の数だけ数える（5000 要素を 100 回なめる）
        let m = |body: &str| {
            format!(
                "function f input Integer n; output Integer y; protected Real a[5000]; algorithm y := n; {body} end f; model M Real x[f(1)]; end M;"
            )
        };
        assert_eq!(
            kind(&m(
                "for i in 1:100 loop if max(a) > 5 then y := 2; end if; end for;"
            )),
            big
        );
        assert_eq!(
            kind(&m(
                "for i in 1:100 loop if a[1] > 5 then y := 2; end if; end for;"
            )),
            None
        );
    }

    #[test]
    fn function_for_range_count_does_not_overflow() {
        // #151: 範囲の幅が `i64::MAX` になる `for` の反復の数 `span / |s| + 1` が溢れ、debug は panic・release は 0 回のループにしていた
        for range in [
            "0:9223372036854775807",
            "-9223372036854775807 - 1:-1",
            "0:-1:-9223372036854775807",
        ] {
            let src = format!(
                "function f input Integer n; output Integer y; algorithm y := 0;
                   for i in {range} loop y := y + 1; if y > 3 then break; end if; end for; end f;
                 model M Real x[f(1) + 1]; end M;"
            );
            let e = Project::from_sources([("t.mo", src.as_str())])
                .flatten("M")
                .unwrap_err();
            assert_eq!(e.kind, "non-constant-dim", "{range}");
        }
    }

    /// 変数の（名前, 大きさ）。断られたら種類
    fn sized(src: &str) -> Result<Vec<(String, Vec<usize>)>, String> {
        let m = Project::from_sources([("t.mo", src)])
            .flatten("M")
            .map_err(|e| e.kind)?;
        Ok(m.vars
            .iter()
            .map(|v| (v.name.clone(), v.dims.clone()))
            .collect())
    }

    /// 変数 `name` の大きさ（1 次元）
    fn dim_of(src: &str, name: &str) -> Result<usize, String> {
        let v = sized(src)?;
        Ok(v.iter()
            .find(|(n, _)| n == name)
            .unwrap_or_else(|| panic!("{name}"))
            .1[0])
    }

    #[test]
    fn function_reads_package_constants_of_the_reaching_package() {
        // #156: 関数の本体が読む package の定数は、呼び出しを通した側の package（`extends P(n = 5)`・`redeclare package`）の値
        let pp = "package PP constant Integer n = 1;
              function f input Integer k; output Integer y; algorithm y := k + n; end f;
              constant Integer m = f(1);
            end PP;
            package P5 extends PP(n = 5); end P5;
            package P7 = PP(n = 7);";
        let m = |body: &str| format!("{pp} model M {body} end M;");
        // `P5.n`（関数なし）・`P5.m = f(1)`・`P5.f(1)`・短い形の package。宣言した側の `PP` は 1 のまま
        for (decl, name, want) in [
            ("Real a[PP.m];", "a", 2),
            ("Real a[P5.n];", "a", 5),
            ("Real a[P5.m];", "a", 6),
            ("Real a[P5.f(1)];", "a", 6),
            ("Real a[P7.m];", "a", 8),
            ("Real a[P7.f(2)];", "a", 9),
        ] {
            assert_eq!(dim_of(&m(decl), name), Ok(want), "{decl}");
        }
        // `redeclare package` を通す。`s2`（既定の `PP`）は 1 のまま
        let sub = "model Sub replaceable package P = PP; Real a[P.f(1)]; Real b[P.n + 1]; Real c[P.m]; end Sub;";
        let src = format!(
            "{pp} {sub} model M Sub s1(redeclare package P = P5); Sub s2; Sub s3(redeclare package P = P7); end M;"
        );
        let v = sized(&src).unwrap();
        let got: Vec<_> = v.iter().map(|(n, d)| (n.as_str(), d[0])).collect();
        assert_eq!(
            got,
            [
                ("s1.a", 6),
                ("s1.b", 6),
                ("s1.c", 6),
                ("s2.a", 2),
                ("s2.b", 2),
                ("s2.c", 2),
                ("s3.a", 8),
                ("s3.b", 8),
                ("s3.c", 8)
            ]
        );
    }

    #[test]
    fn redeclared_and_partial_functions_are_not_run_as_declared() {
        // #155: `redeclare function f = f3` と、package ごとの差し替えを引く。本体の無い `partial` の関数・出力が代入されない関数は「分からない」
        let sub = "model Sub replaceable function f input Integer k; output Integer y; algorithm y := k; end f;
              Real x[f(2)]; end Sub;
            function f3 input Integer k; output Integer y; algorithm y := 3*k; end f3;";
        let a = |body: &str, name: &str| dim_of(&format!("{sub} model M {body} end M;"), name);
        assert_eq!(a("Sub a(redeclare function f = f3); Sub b;", "a.x"), Ok(6));
        assert_eq!(a("Sub a(redeclare function f = f3); Sub b;", "b.x"), Ok(2));
        // package の中の関数（`replaceable`・`partial`）
        let pk = "partial package PP replaceable partial function f input Integer k; output Integer y; end f; end PP;
            package P3 extends PP; redeclare function f input Integer k; output Integer y; algorithm y := 3*k; end f; end P3;
            package Q extends PP; end Q;
            model Sub replaceable package P = PP; Real x[P.f(2)]; end Sub;";
        let b = |body: &str| dim_of(&format!("{pk} model M {body} end M;"), "a.x");
        assert_eq!(b("Sub a(redeclare package P = P3);"), Ok(6));
        // 差し替えがない（本体の無い `partial` の `f`）・本体の無い関数を受け継いだだけの package は 0 にせず断る
        assert_eq!(b("Sub a;"), Err("non-constant-dim".into()));
        assert_eq!(
            b("Sub a(redeclare package P = Q);"),
            Err("non-constant-dim".into())
        );
        // 非 `partial` の関数を `P3` が差し替える（`b` は宣言した側）
        let pk = "package PP replaceable function f input Integer k; output Integer y; algorithm y := k; end f; end PP;
            package P3 extends PP; redeclare function f input Integer k; output Integer y; algorithm y := 3*k; end f; end P3;
            model Sub replaceable package P = PP; Real x[P.f(2)]; end Sub;
            model M Sub a(redeclare package P = P3); Sub b; end M;";
        assert_eq!(
            sized(pk).unwrap(),
            [("a.x".into(), vec![6]), ("b.x".into(), vec![2])]
        );
        // 出力が代入されないまま終わった関数
        let g = "function g input Integer k; output Integer y; algorithm if k > 100 then y := 1; end if; end g;";
        assert_eq!(
            sized(&format!("{g} model M Real x[g(2) + 3]; end M;")),
            Err("non-constant-dim".into())
        );
        assert_eq!(
            dim_of(&format!("{g} model M Real x[g(200) + 3]; end M;"), "x"),
            Ok(4)
        );
    }

    #[test]
    fn function_memo_works_with_redeclared_packages_and_tells_contexts_apart() {
        // #154: `redeclare` が 1 つあるだけで結果を覚えず、インスタンスの数だけ関数を走らせて予算（試験では 1 万）が尽きた
        let w = "function w input Integer k; output Integer y; algorithm y := 0; for i in 1:1000 loop y := y + 1; end for; y := k; end w;
            package PP constant Integer c = 1; end PP;
            package P5 extends PP(c = 5); end P5;
            model Sub replaceable package P = PP; parameter Integer n = w(2); Real a[n]; end Sub;";
        let inst = |n: usize, red: &str| -> String {
            let xs: String = (0..n).map(|i| format!("Sub s{i}{red}; ")).collect();
            format!("{w} model M {xs} end M;")
        };
        assert!(sized(&inst(20, "")).is_ok());
        assert!(sized(&inst(20, "(redeclare package P = P5)")).is_ok());
        // 鍵は差し替えの組を区別する: 局所の関数が `P.c` を読み、`P` が違うインスタンスは値が違う（順序によらない）
        let g = "package PP constant Integer c = 1; end PP;
            package P5 extends PP(c = 5); end P5;
            model Sub replaceable package P = PP;
              function g input Integer k; output Integer y; algorithm y := k + P.c; end g;
              Real x[g(1)];
            end Sub;";
        for decls in [
            "Sub s1(redeclare package P = P5); Sub s2;",
            "Sub s2; Sub s1(redeclare package P = P5);",
        ] {
            let v = sized(&format!("{g} model M {decls} end M;")).unwrap();
            let d = |n: &str| v.iter().find(|(k, _)| k == n).unwrap().1[0];
            assert_eq!((d("s1.x"), d("s2.x")), (6, 2), "{decls}");
        }
    }

    #[test]
    fn failed_calls_are_remembered_so_unrelated_calls_are_not_refused() {
        // #153: 評価できない関数の呼び出しは結果を覚えず、同じ式を評価するたびに予算を使って、無関係な `ok(3)` まで断られた
        let n = 40;
        let xs: String = (0..n).map(|i| format!("A a{i}; ")).collect();
        let src = format!(
            "function ext input Real x; output Real y; external \"C\" y = ext_c(x); end ext;
            function slow input Integer n; output Real y; algorithm y := 0; for i in 1:n loop y := y + 1; end for; y := ext(y); end slow;
            function ok input Integer k; output Integer y; algorithm y := k; end ok;
            model A parameter Real p = slow(300); Real w; equation if p > 0 then w = 1; else w = 2; end if; end A;
            model B Real v[ok(3)]; equation for i in 1:3 loop v[i] = time; end for; end B;
            model M {xs} B b; end M;"
        );
        assert_eq!(dim_of(&src, "b.v"), Ok(3));
    }

    #[test]
    fn constant_chains_are_evaluated_once_per_binding() {
        // #158: `c_i = c_{i-1} + c_{i-1}` は共有を覚えず 2^n 回の評価になった。評価の入れ子の上限（約 24 段）を、覚えた値が越える
        // （手前の値を先に評価して覚えておくと、深い段が届く。覚えないと上限で断られる）
        let k = |names: &[usize]| {
            let cs: String = (1..=34)
                .map(|i| format!("constant Integer c{i} = c{} + c{};", i - 1, i - 1))
                .collect();
            let xs: String = names
                .iter()
                .map(|i| format!("Real a{i}[K.c{i} + 1]; "))
                .collect();
            format!("package K constant Integer c0 = 0; {cs} end K; model M {xs} end M;")
        };
        assert!(sized(&k(&[4, 8, 12, 16, 20, 24, 28, 32])).is_ok());
        assert_eq!(sized(&k(&[34])), Err("non-constant-dim".into()));
        // モデルの parameter の連鎖も同じ
        let ps: String = (1..=34)
            .map(|i| format!("parameter Integer c{i} = c{} + c{};", i - 1, i - 1))
            .collect();
        let m = |names: &[usize]| {
            let xs: String = names
                .iter()
                .map(|i| format!("Real a{i}[c{i} + 1]; "))
                .collect();
            format!("model M parameter Integer c0 = 0; {ps} {xs} end M;")
        };
        assert!(sized(&m(&[4, 8, 12, 16, 20, 24, 28, 32])).is_ok());
        assert_eq!(sized(&m(&[34])), Err("non-constant-dim".into()));
    }

    #[test]
    fn evaluations_inside_functions_are_budgeted() {
        // #158: 関数の中で package の定数を読む評価も仕事量に数える（1 反復 2 tick に読みの 1 tick が加わる。予算は試験で 1 万）
        let f = |n: usize| {
            format!(
                "package K constant Integer c = 1; end K;
                function f input Integer n; output Integer y; algorithm y := 0; for i in 1:n loop y := y + K.c; end for; end f;
                model M Real x[f({n})]; end M;"
            )
        };
        assert_eq!(dim_of(&f(2000), "x"), Ok(2000));
        assert_eq!(sized(&f(4000)), Err("non-constant-dim".into()));
    }

    #[test]
    fn string_comparison_concat_and_string_function_are_decided() {
        // S377: 文字列の `<`・`<=`・`>`・`>=` は辞書順（バイト順。omc の checkModel と同じ）、`+` は連結、`String(整数・論理)`、エスケープは解く。以前は大小が「分からない」で、`+` は hash の和の実数になった
        let d = |cond: &str| {
            dim_of(
                &format!(
                    "type Color = enumeration(red, green); model M Real x[if {cond} then 2 else 3]; end M;"
                ),
                "x",
            )
        };
        for (cond, want) in [
            (r#""a" < "b""#, 2),
            (r#""b" < "a""#, 3),
            (r#""Z" < "a""#, 2),
            (r#""ab" < "abc""#, 2),
            (r#""a" <= "a""#, 2),
            (r#""b" > "a""#, 2),
            (r#""a" >= "b""#, 3),
            (r#""" < "a""#, 2),
            (r#""é" > "z""#, 2),
            (r#""a" + "b" == "ab""#, 2),
            (r#""a" + "b" < "ac""#, 2),
            (r#"String(12) == "12""#, 2),
            (r#"String(true) == "true""#, 2),
            (r#"String(-3) < "-2""#, 3),
            (r#"String(Color.green) == "green""#, 2),
            // 比べるのはエスケープを解いた中身（`\t` は 0x09 で `Z` より小さい。字面の `\t` は 0x5C で大きい）
            (r#""\t" < "Z""#, 2),
            (r#""a\"b" == "a\"b""#, 2),
            (r#""a\\" == "a\\""#, 2),
        ] {
            assert_eq!(d(cond), Ok(want), "{cond}");
        }
        // 文字列と数の混ぜ・算術は決めない（hash の和の実数を返さない）
        for cond in [
            r#""a" < 1"#,
            r#"("a" + 1) == 1"#,
            r#"-"a" == 1"#,
            r#""a" - "b" == 1"#,
        ] {
            assert_eq!(d(cond), Err("non-constant-dim".into()), "{cond}");
        }
        // 定数・parameter の文字列も同じ表（宣言が後ろでも）
        let m = |body: &str| {
            format!("package K constant String s = \"b\" + \"c\"; end K; model M {body} end M;")
        };
        assert_eq!(
            dim_of(
                &m("parameter String t = \"bd\"; Real x[if K.s < t then 2 else 3];"),
                "x"
            ),
            Ok(2)
        );
        assert_eq!(
            dim_of(
                &m("parameter String t = \"bb\"; Real x[if K.s < t then 2 else 3];"),
                "x"
            ),
            Ok(3)
        );
        // 関数の本体の中でも同じ表（`String(k)`・文字列どうしの大小）
        let f = |k: usize| {
            format!(
                "function f input Integer k; output Integer y; algorithm
                   if String(k) == \"2\" then y := 7; elseif \"b\" + String(k) > \"b3\" then y := 8; else y := 1; end if;
                 end f; model M Real x[f({k})]; end M;"
            )
        };
        assert_eq!([2, 4, 3].map(|k| dim_of(&f(k), "x")), [Ok(7), Ok(8), Ok(1)]);
        // 文字列の連結が 2 倍ずつ伸びる連鎖は上限で断る（2^34 バイトを作らない）
        let cs: String = (1..=34)
            .map(|i| format!("constant String c{i} = c{} + c{};", i - 1, i - 1))
            .collect();
        let big = format!(
            "package K constant String c0 = \"ab\"; {cs} end K; model M Real x[if K.c34 == K.c34 then 2 else 3]; end M;"
        );
        assert_eq!(sized(&big), Err("non-constant-dim".into()));
    }

    #[test]
    fn nested_package_function_reads_constants_of_the_reaching_package() {
        // S376 の持ち越し: `P5.Inner.f` の本体が読む定数 `n`（`PP` で宣言）は、`P5`（`extends PP(n = 5)`）を通した値
        let pp = "package PP constant Integer n = 1;
              package Inner
                function f input Integer k; output Integer y; algorithm y := k + n; end f;
                function h input Integer k; output Integer y; algorithm y := f(k) * 2; end h;
              end Inner;
            end PP;
            package P5 extends PP(n = 5); end P5;";
        let m = |body: &str| format!("{pp} model M {body} end M;");
        for (decl, want) in [
            ("Real a[PP.Inner.f(1)];", 2),
            ("Real a[P5.Inner.f(1)];", 6),
            ("Real a[P5.Inner.h(1)];", 12),
            ("Real a[PP.Inner.h(1)];", 4),
        ] {
            assert_eq!(dim_of(&m(decl), "a"), Ok(want), "{decl}");
        }
        // `redeclare package` を通した `P.Inner.f`
        let sub = "model Sub replaceable package P = PP; Real a[P.Inner.f(1)]; Real b[P.Inner.h(1)]; end Sub;";
        let v = sized(&format!(
            "{pp} {sub} model M Sub s1(redeclare package P = P5); Sub s2; end M;"
        ))
        .unwrap();
        let got: Vec<_> = v.iter().map(|(n, d)| (n.as_str(), d[0])).collect();
        assert_eq!(got, [("s1.a", 6), ("s1.b", 12), ("s2.a", 2), ("s2.b", 4)]);
    }

    #[test]
    fn function_calls_the_redeclared_sibling_function() {
        // S376 の持ち越し: 関数の中から呼ぶ兄弟の関数 `g`。インスタンスの `redeclare function g = g3` を引く
        let sub = "model Sub replaceable function g input Integer k; output Integer y; algorithm y := k; end g;
              function f input Integer k; output Integer y; algorithm y := g(k) + 1; end f;
              Real x[f(2)];
            end Sub;
            function g3 input Integer k; output Integer y; algorithm y := 3*k; end g3;";
        let v = sized(&format!(
            "{sub} model M Sub a(redeclare function g = g3); Sub b; end M;"
        ))
        .unwrap();
        assert_eq!(v, [("a.x".into(), vec![7]), ("b.x".into(), vec![3])]);
        // 別のインスタンスから静的に呼ぶ `Sub.f`（`Other` の `g` の差し替えは `Sub.g` に届かない）
        let other = "model Other replaceable function g input Integer k; output Integer y; algorithm y := 5*k; end g;
              Real z[Sub.f(2)]; end Other;";
        let v = sized(&format!(
            "{sub} {other} model M Other o(redeclare function g = g3); end M;"
        ))
        .unwrap();
        assert_eq!(v, [("o.z".into(), vec![3])]);
    }

    #[test]
    fn parenthesized_package_redeclare_changes_the_constants_it_reaches() {
        // S376 の持ち越し: `redeclare package P = PP(n = 5)` の `P.n` は 5（以前は宣言した側の 1）。`n` を使う `m` は決めない
        let pp = "package PP constant Integer n = 1; constant Integer m = n + 1; constant Integer k = 4; end PP;
            model Sub replaceable package P = PP; Real a[P.NAME]; end Sub;";
        let a = |name: &str, red: &str| {
            dim_of(
                &format!("{} model M Sub s{red}; end M;", pp.replace("NAME", name)),
                "s.a",
            )
        };
        let red = "(redeclare package P = PP(n = 5))";
        assert_eq!(a("n", red), Ok(5));
        assert_eq!(a("k", red), Ok(4));
        assert_eq!(a("m", red), Err("non-constant-dim".into()));
        assert_eq!((a("n", ""), a("m", ""), a("k", "")), (Ok(1), Ok(2), Ok(4)));
    }
}

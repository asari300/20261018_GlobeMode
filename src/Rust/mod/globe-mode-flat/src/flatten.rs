//! 平らにする本体。クラスをインスタンス化し（`extends` を取り込み、修飾を外側から当てる）、変数と方程式を集める。

use crate::{Causality, Equation, FlatModel, Project, Res, Var, VarKind, un};
use globe_mode_resolve::{ClassId, Reason, Resolution};
use globe_mode_syntax::ast::{self, Element, Expr};
use globe_mode_syntax::{SyntaxKind as K, SyntaxNode};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::rc::Rc;

/// 修飾 `x(start = 1) = 2`。値は書いた場所の文脈（`Bind::ctx`）で名前を付け替える
#[derive(Clone, Default)]
struct Mod {
    val: Option<Bind>,
    sub: ModMap,
    each: bool,
}
type ModMap = BTreeMap<String, Mod>;

#[derive(Clone)]
struct Bind {
    node: SyntaxNode,
    file: u32,
    ctx: Rc<Ctx>,
    /// レコードの束縛 `r = other` を欄に分けたときの `.f.g`（印字だけに足す。評価はしない）
    suffix: String,
}

/// 式を書いた場所: インスタンスの名前の前置き（`a.b.`）、そのクラスと基底の全部、`for` の添字の値
struct Ctx {
    pfx: String,
    lineage: Rc<Vec<ClassId>>,
    locals: Vec<(String, i64)>,
}

impl Ctx {
    fn local(&self, n: &str) -> Option<i64> {
        self.locals
            .iter()
            .rev()
            .find(|(k, _)| k == n)
            .map(|(_, v)| *v)
    }
}

fn merge_mod(o: &Mod, i: &Mod) -> Mod {
    Mod {
        val: o.val.clone().or_else(|| i.val.clone()),
        sub: merge(&o.sub, &i.sub),
        each: o.each || i.each,
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
            K::KwReplaceable | K::KwRedeclare => {
                return un("replaceable", "replaceable / redeclare");
            }
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

#[derive(Clone, Copy)]
enum Val {
    I(i64),
    R(f64),
    B(bool),
    /// 列挙の要素（列挙のクラスと、その中の順番）
    E(ClassId, usize),
}

impl Val {
    fn f(self) -> f64 {
        match self {
            Val::I(i) => i as f64,
            Val::R(r) => r,
            Val::B(b) => f64::from(u8::from(b)),
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

enum Item {
    Decl {
        cc: ast::ComponentClause,
        file: u32,
        mods: Rc<ModMap>,
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
    conns: Vec<(String, String)>,
}

const MAX_VARS: usize = 200_000;

struct F<'a> {
    p: &'a Project,
    vars: Vec<Var>,
    var_idx: BTreeMap<String, usize>,
    eqs: Vec<Equation>,
    whens: Vec<Equation>,
    ieqs: Vec<Equation>,
    algs: Vec<String>,
    env: HashMap<String, Bind>,
    /// `inner` で宣言した部品の完全な名前
    inners: HashSet<String>,
    /// `outer` の部品 → 結びつけた `inner` の完全な名前
    alias: HashMap<String, String>,
    /// 条件が偽で落とした部品の完全な名前（これにつながる `connect` も落とす）
    removed: Vec<String>,
    /// まだ平らにしていない宣言（`size(x, 1)` が後ろで宣言した配列の大きさを引くため）
    pending: HashMap<String, Pending>,
    depth: std::cell::Cell<u32>,
}

struct Pending {
    cc: ast::ComponentClause,
    d: ast::Declaration,
    file: u32,
    ctx: Rc<Ctx>,
    m: Mod,
}

/// 「パッケージの定数を評価する文脈」の印（`Ctx::pfx`）
const PKG: &str = "\u{0}";

pub(crate) fn run(p: &Project, cid: ClassId, model: &str) -> Res<FlatModel> {
    let mut f = F {
        p,
        vars: Vec::new(),
        var_idx: BTreeMap::new(),
        eqs: Vec::new(),
        whens: Vec::new(),
        ieqs: Vec::new(),
        algs: Vec::new(),
        env: HashMap::new(),
        inners: HashSet::new(),
        alias: HashMap::new(),
        removed: Vec::new(),
        pending: HashMap::new(),
        depth: std::cell::Cell::new(0),
    };
    let own = f.inst_class(cid, "", &ModMap::new(), Flags::default())?;
    // 最上位のモデル自身のコネクタの flow は 0（外へ流れ出ない。Rumoca の flat も同じ）
    for n in own {
        for (suffix, vi, count) in f.leaves(&n) {
            if f.vars[vi].flow {
                f.eqs.push(Equation {
                    text: format!("{n}{suffix} = 0"),
                    count,
                });
            }
        }
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

    /// 戻り値は、このインスタンスが直接持つコネクタの名前（親が「内側のコネクタ」として使う）
    fn inst_class(
        &mut self,
        cid: ClassId,
        pfx: &str,
        mods: &ModMap,
        flags: Flags,
    ) -> Res<Vec<String>> {
        let Some(cd) = self.p.class(cid).cloned() else {
            return un("unresolved", "class without syntax");
        };
        if cd.is_class_extends() {
            return un("class-extends", self.p.lib.qualified_name(cid));
        }
        let prefix: Vec<K> = tokens(cd.syntax())
            .map(|t| t.kind())
            .take_while(|k| *k != K::Ident)
            .collect();
        if prefix.contains(&K::KwExpandable) {
            return un("expandable", self.p.lib.qualified_name(cid));
        }
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
        });
        let mut items = Vec::new();
        self.collect(cid, mods, &ctx, &mut items, 0)?;
        // 束縛を先に登録する（大きさ・添字が同じインスタンスの parameter を使うので）
        let mut decls = Vec::new();
        for it in &items {
            let Item::Decl { cc, file, mods } = it else {
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
                let m = match mods.get(&name) {
                    Some(o) => merge_mod(o, &own),
                    None => own,
                };
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
                decls.push((cc.clone(), *file, d, name, m));
            }
        }
        let mut own_conns = Vec::new();
        let mut inside = Vec::new();
        for (cc, file, d, name, m) in decls {
            self.inst_decl(
                &ctx,
                file,
                &cc,
                &d,
                &name,
                m,
                inherit(flags),
                &mut own_conns,
                &mut inside,
            )?;
        }
        let mut lv = Level::default();
        for it in &items {
            if let Item::Sec { node, file } = it {
                self.section(node, *file, &ctx, &mut lv)?;
            }
        }
        self.gen_connects(pfx, &lv, &inside)?;
        Ok(own_conns)
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
        &mut self,
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
            return match self.p.res_of(file, base.range()) {
                Some(Resolution::Class(b)) => self.collect(*b, &m2, ctx, items, depth + 1),
                _ => un("extends-builtin", base.parts().join(".")),
            };
        }
        let shared = Rc::new(mods.clone());
        for e in cd.elements() {
            match e {
                Element::Extends(x) => {
                    let Some(n) = x.name() else { continue };
                    if x.has_redeclare() {
                        return un("redeclare", format!("extends {}", n.parts().join(".")));
                    }
                    let m2 = match x
                        .syntax()
                        .children()
                        .find(|c| c.kind() == K::ClassModification)
                    {
                        Some(cm) => merge(mods, &self.mods_of(&cm, file, ctx)?),
                        None => mods.clone(),
                    };
                    match self.p.res_of(file, n.range()) {
                        Some(Resolution::Class(b)) => {
                            self.collect(*b, &m2, ctx, items, depth + 1)?
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
        let own_flags = flags_of(
            tokens(cc.syntax())
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
        if cc.constraint().is_some() {
            return un("replaceable", name);
        }
        let Some(tn) = cc.type_name() else {
            return un("unresolved", "no type name");
        };
        let tres = self.p.res_of(file, tn.range()).cloned();
        let ty = self.type_of(tres, &tn.parts())?;
        let dims = self.decl_dims(ctx, file, cc, d, name, &m)?;
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
                self.var_idx.insert(full.clone(), self.vars.len());
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
                // `R r = R(a = 1)`・`r = other` は欄ごとの修飾にする。明示した修飾 > 束縛 > 型の修飾
                let bound = match &m.val {
                    Some(b) if dims.is_empty() => self.record_value(b, name)?,
                    Some(b) => {
                        let t = b.node.text();
                        return un("composite-binding", format!("{name}[] = {t}"));
                    }
                    None => ModMap::new(),
                };
                let eff = fill(parent, fill(own_flags, flags));
                let mods = merge(&m.sub, &merge(&bound, &mods));
                if !dims.is_empty() && has_array_value(&m.sub) {
                    return un("array-modification", name);
                }
                for ix in idx_names(&dims) {
                    let el = format!("{full}{ix}");
                    let sub = self.inst_class(cid, &format!("{el}."), &mods, inherit(eff))?;
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

    /// レコードの束縛（コンストラクタの呼び出しか、別のレコードの部品）を、欄ごとの修飾にする
    fn record_value(&self, b: &Bind, name: &str) -> Res<ModMap> {
        let bind = |e: &SyntaxNode, suffix: String| Bind {
            node: e.clone(),
            file: b.file,
            ctx: b.ctx.clone(),
            suffix,
        };
        let mut out = ModMap::new();
        match Expr::cast(b.node.clone()) {
            Some(Expr::Call(c)) => {
                let rid = c
                    .syntax()
                    .children()
                    .find_map(ast::ComponentRef::cast)
                    .and_then(|r| match self.p.res_of(b.file, r.range()) {
                        Some(Resolution::Class(rid)) => Some(*rid),
                        _ => None,
                    })
                    .filter(|rid| {
                        matches!(self.p.kind_of(*rid), Some(K::KwRecord | K::KwOperator))
                    });
                let Some(rid) = rid else {
                    return un("composite-binding", format!("{name} = {}", b.node.text()));
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
                let pre = format!("{t}.");
                let leaves: Vec<&str> = self
                    .var_idx
                    .range(pre.clone()..)
                    .take_while(|(k, _)| k.starts_with(&pre))
                    .map(|(k, _)| &k[pre.len()..])
                    .collect();
                if leaves.is_empty() || leaves.iter().any(|l| l.contains('[')) {
                    return un(
                        "composite-binding",
                        format!("{name} = {t} (欄が分からない)"),
                    );
                }
                for l in leaves {
                    let path: Vec<&str> = l.split('.').collect();
                    nest(&mut out, &path, bind(&b.node, format!(".{l}")));
                }
            }
            _ => return un("composite-binding", format!("{name} = {}", b.node.text())),
        }
        Ok(out)
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
                self.record_fields(*b, out, depth + 1);
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
                    && self.is_enum(*cid)
                {
                    dims.push(self.p.lib.classes[*cid].components.len());
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
            // shortcut: 大きさを決められるのは束縛の式の形が分かるときだけ（`dims_of`。ユーザー定義の関数の戻り値は断る）
            match m
                .val
                .as_ref()
                .and_then(|b| self.dims_of(&b.node, b.file, &b.ctx))
            {
                Some(v) if !v.is_empty() => dims = v,
                _ => {
                    let b = m.val.as_ref().map(|b| b.node.text().to_string());
                    return un("unsized-array", format!("{name} = {b:?}"));
                }
            }
        }
        Ok(dims)
    }

    /// 型名を、組み込みの型（短いクラスの鎖をたどった属性の修飾つき）か、部品をもつクラスにする
    fn type_of(&self, mut res: Option<Resolution>, parts: &[String]) -> Res<Ty> {
        let mut acc = ModMap::new();
        let mut flags = Flags::default();
        let mut conn = false;
        let mut last = parts.last().cloned().unwrap_or_default();
        let tctx = Rc::new(Ctx {
            pfx: String::new(),
            lineage: Rc::new(Vec::new()),
            locals: Vec::new(),
        });
        for _ in 0..64 {
            match res {
                Some(Resolution::Builtin) => {
                    return Ok(Ty::Prim {
                        ty: last,
                        mods: acc,
                        flags,
                        conn,
                    });
                }
                Some(Resolution::Class(cid)) => {
                    let cd = self.p.class(cid).ok_or_else(|| crate::Unsupported {
                        kind: "unresolved",
                        detail: "class without syntax".into(),
                    })?;
                    conn |= cd.kind_token().map(|t| t.kind()) == Some(K::KwConnector);
                    let Some(sb) = cd.short_body() else {
                        return Ok(Ty::Comp {
                            cid,
                            mods: acc,
                            flags,
                            conn,
                        });
                    };
                    let file = self.p.lib.classes[cid].file;
                    let toks: Vec<K> = tokens(sb.syntax()).map(|t| t.kind()).collect();
                    if toks.contains(&K::KwEnumeration) {
                        let q = self.p.lib.qualified_name(cid);
                        return Ok(Ty::Prim {
                            ty: q,
                            mods: acc,
                            flags,
                            conn,
                        });
                    }
                    if toks.contains(&K::KwDer) {
                        return un("der-class", self.p.lib.qualified_name(cid));
                    }
                    if sb.syntax().children().any(|c| c.kind() == K::Subscripts) {
                        return un("array-type", self.p.lib.qualified_name(cid));
                    }
                    if sb.has_redeclare() {
                        return un("redeclare", self.p.lib.qualified_name(cid));
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
                    res = self.p.res_of(file, base.range()).cloned();
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
                K::Redeclaration => return un("redeclare", ch.text().to_string()),
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
            });
        Ok(Mod { val, sub, each })
    }

    fn print_bind(&self, b: &Bind) -> Res<String> {
        Ok(self.text(&b.node, b.file, &b.ctx)? + &b.suffix)
    }

    // ---------------------------------------------------------------- 式の印字

    fn text(&self, n: &SyntaxNode, file: u32, ctx: &Rc<Ctx>) -> Res<String> {
        let mut out = Vec::new();
        self.pr(n, file, ctx, &mut out)?;
        Ok(join(&out))
    }

    fn pr(&self, n: &SyntaxNode, file: u32, ctx: &Rc<Ctx>, out: &mut Vec<String>) -> Res<()> {
        if let Some(c) = ast::ComponentRef::cast(n.clone()) {
            out.push(self.cref(&c, file, ctx)?);
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
        match r {
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
                    _ => None,
                }
            }
            Expr::Paren(p) => match p.items().as_slice() {
                [e] => self.eval(e.syntax(), file, ctx, d),
                _ => None,
            },
            Expr::Unary(u) => {
                let v = self.eval(u.operand()?.syntax(), file, ctx, d)?;
                match u.op()?.kind() {
                    K::Minus => Some(match v {
                        Val::I(i) => Val::I(-i),
                        v => Val::R(-v.f()),
                    }),
                    K::Plus => Some(v),
                    K::KwNot => match v {
                        Val::B(b) => Some(Val::B(!b)),
                        _ => None,
                    },
                    _ => None,
                }
            }
            Expr::Binary(b) => {
                let operands = b.operands();
                let ops = b.ops();
                let mut acc = self.eval(operands.first()?.syntax(), file, ctx, d)?;
                for (o, op) in operands[1..].iter().zip(ops.iter()) {
                    let r = self.eval(o.syntax(), file, ctx, d)?;
                    acc = binop(op.kind(), acc, r)?;
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
                if n.children().any(|x| x.kind() == K::Subscripts) {
                    return None;
                }
                let parts = c.parts();
                match self.p.res_of(file, c.range())? {
                    Resolution::Local if parts.len() == 1 => ctx.local(&parts[0]).map(Val::I),
                    Resolution::Component(cid, idx) => {
                        if ctx.pfx != PKG && ctx.lineage.contains(cid) {
                            let key = format!("{}{}", ctx.pfx, parts.join("."));
                            let b = self.env.get(self.alias.get(&key).unwrap_or(&key))?;
                            if !b.suffix.is_empty() {
                                return None;
                            }
                            self.eval(&b.node, b.file, &b.ctx, d)
                        } else {
                            // 列挙の要素、またはパッケージの定数
                            let comp = &self.p.lib.classes[*cid].components[*idx].name;
                            if parts.last() != Some(comp) {
                                return None;
                            }
                            if self.is_enum(*cid) {
                                return Some(Val::E(*cid, *idx));
                            }
                            let (node, f, c2, _) = self.pkg_const(*cid, comp)?;
                            self.eval(&node, f, &c2, d)
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
                        let name = self.cref(r, file, ctx).ok()?;
                        let k = v(1)?.int()? as usize;
                        let dims = self.var_dims(&name)?;
                        dims.get(k.checked_sub(1)?).map(|&x| Val::I(x as i64))
                    }
                    "abs" => Some(match v(0)? {
                        Val::I(i) => Val::I(i.abs()),
                        x => Val::R(x.f().abs()),
                    }),
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
                        let mut it = xs.into_iter();
                        let first = it.next()?;
                        Some(it.fold(first, |a, b| if (a.f() >= b.f()) == is_max { a } else { b }))
                    }
                    "floor" => Some(Val::I(v(0)?.f().floor() as i64)),
                    "ceil" => Some(Val::I(v(0)?.f().ceil() as i64)),
                    "integer" => Some(Val::I(v(0)?.f().floor() as i64)),
                    "div" => Some(Val::I((v(0)?.f() / v(1)?.f()).trunc() as i64)),
                    "mod" => {
                        let (a, b) = (v(0)?.int()?, v(1)?.int()?);
                        (b != 0).then(|| Val::I(a - (a as f64 / b as f64).floor() as i64 * b))
                    }
                    "sqrt" => Some(Val::R(v(0)?.f().sqrt())),
                    _ => None,
                }
            }
            _ => None,
        }
    }

    /// 変数の大きさ。まだ平らにしていなければ、宣言から出す（循環は深さで止める）
    fn var_dims(&self, name: &str) -> Option<Vec<usize>> {
        if let Some(&i) = self.var_idx.get(name) {
            return Some(self.vars[i].dims.clone());
        }
        let p = self.pending.get(name)?;
        if self.depth.get() > 16 {
            return None;
        }
        self.depth.set(self.depth.get() + 1);
        let r = self.decl_dims(&p.ctx, p.file, &p.cc, &p.d, name, &p.m);
        self.depth.set(self.depth.get() - 1);
        r.ok()
    }

    fn is_enum(&self, cid: ClassId) -> bool {
        self.p
            .class(cid)
            .and_then(|c| c.short_body())
            .is_some_and(|sb| tokens(sb.syntax()).any(|t| t.kind() == K::KwEnumeration))
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

    /// 式の配列の形（スカラーは空）。分からなければ `None`
    // shortcut: 行列の積・関数の戻り値の形は見ない（要素ごとの関数だけ）。必要になったら足す
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
                let mut out: Vec<usize> = Vec::new();
                for o in b.operands() {
                    let d = self.dims_of(o.syntax(), file, ctx)?;
                    if out.is_empty() {
                        out = d;
                    } else if !d.is_empty() && d != out {
                        return None;
                    }
                }
                Some(out)
            }
            Expr::If(i) => self.dims_of(i.branches().first()?.1.syntax(), file, ctx),
            Expr::Array(a) => {
                let items = a.items();
                let mut v = vec![items.len()];
                v.extend(self.dims_of(items.first()?.syntax(), file, ctx)?);
                Some(v)
            }
            Expr::Matrix(m) => {
                let rows = 1 + tokens(m.syntax()).filter(|t| t.kind() == K::Semi).count();
                let items: Vec<Expr> = m.syntax().children().filter_map(Expr::cast).collect();
                for e in &items {
                    if !self.dims_of(e.syntax(), file, ctx)?.is_empty() {
                        return None; // 要素が配列の連結は断る
                    }
                }
                (items.len().is_multiple_of(rows)).then(|| vec![rows, items.len() / rows])
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
                    "from_rpm" | "from_deg" | "to_deg" | "sqrt" | "abs" | "sin" | "cos" | "exp"
                    | "log" => self.dims_of(args.first()?.syntax(), file, ctx),
                    _ => None,
                }
            }
            Expr::ComponentRef(c) => {
                if n.children().any(|x| x.kind() == K::Subscripts) {
                    return None;
                }
                match self.p.res_of(file, c.range())? {
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
                        let (node, f, c2, scalar) = self.pkg_const(*cid, comp)?;
                        if scalar {
                            return Some(Vec::new());
                        }
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
        let e = Equation { text, count };
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
                    self.whens.push(Equation { text: t, count: c });
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
                        let (x, y) = (self.cref(x, file, ctx)?, self.cref(y, file, ctx)?);
                        if !(self.is_removed(&x) || self.is_removed(&y)) {
                            lv.conns.push((x, y));
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
                        let c = self
                            .shape(l, file, ctx)
                            .or_else(|| self.shape(r, file, ctx))
                            .unwrap_or(1);
                        self.push(initial, t, c);
                    }
                    _ => return un("equation-shape", node.text().to_string()),
                }
                Ok(())
            }
        }
    }

    /// `if` の方程式の条件を先頭から評価し、真になる枝（無ければ `else`、それも無ければ空）の方程式を返す。
    /// 途中で評価できない条件があれば `None`
    fn const_branch(&self, node: &SyntaxNode, file: u32, ctx: &Rc<Ctx>) -> Option<Vec<SyntaxNode>> {
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
                    branches.last_mut()?.0 = Some(c);
                    want_cond = false;
                }
                rowan::NodeOrToken::Node(c) if c.kind() == K::Equation => {
                    branches.last_mut()?.1.push(c);
                }
                _ => {}
            }
        }
        for (cond, eqs) in branches {
            let Some(c) = cond else { return Some(eqs) }; // else
            match self.eval(&c, file, ctx, 0)? {
                Val::B(true) => return Some(eqs),
                Val::B(false) => {}
                _ => return None,
            }
        }
        Some(Vec::new())
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
            let mut locals = ctx.locals.clone();
            locals.push((name.clone(), v));
            let c2 = Rc::new(Ctx {
                pfx: ctx.pfx.clone(),
                lineage: ctx.lineage.clone(),
                locals,
            });
            self.for_eq(node, rest, file, &c2, initial, lv)?;
        }
        Ok(())
    }

    fn range_vals(&self, r: &SyntaxNode, file: u32, ctx: &Rc<Ctx>) -> Res<Vec<i64>> {
        let ev = |n: &SyntaxNode| {
            self.eval(n, file, ctx, 0)
                .and_then(Val::int)
                .ok_or_else(|| crate::Unsupported {
                    kind: "non-constant-for",
                    detail: r.text().to_string(),
                })
        };
        let v: Vec<i64> = match Expr::cast(r.clone()) {
            Some(Expr::Range(x)) => {
                let p = x.parts();
                let n: Vec<i64> = p.iter().map(|e| ev(e.syntax())).collect::<Res<_>>()?;
                let (a, s, b) = match n.as_slice() {
                    [a, b] => (*a, 1, *b),
                    [a, s, b] if *s != 0 => (*a, *s, *b),
                    _ => return un("non-constant-for", r.text().to_string()),
                };
                let mut out = Vec::new();
                let mut i = a;
                while (s > 0 && i <= b) || (s < 0 && i >= b) {
                    out.push(i);
                    i += s;
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
                // `(a, b) = f(x)` は出力の数だけ
                items => Some(items.len()),
            },
            Expr::Call(c) if c.callee().as_deref() == Some("der") => {
                self.shape(c.args().first()?, file, ctx)
            }
            Expr::Array(a) => Some(a.items().len()),
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
                let dims = &self.vars[*self.var_idx.get(&base)?].dims;
                if t.contains(':') {
                    return None;
                }
                (dims.len() > nsub).then(|| dims[nsub..].iter().product())
            }
            _ => None,
        }
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
                            [l, r] => self
                                .shape(l, file, ctx)
                                .or_else(|| self.shape(r, file, ctx))
                                .unwrap_or(1),
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
                        .product::<usize>()
                })
                .unwrap_or(1);
            return total * n;
        }
        total
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

    /// 1 つのインスタンスの `connect` から、potential の等式と flow の和（つながない内側の flow は 0）を作る
    fn gen_connects(&mut self, pfx: &str, lv: &Level, inside: &[String]) -> Res<()> {
        // 集合（union-find）。出てきた順に並べる
        let mut names: Vec<String> = Vec::new();
        let mut parent: Vec<usize> = Vec::new();
        let id = |s: &String, names: &mut Vec<String>, parent: &mut Vec<usize>| {
            names.iter().position(|x| x == s).unwrap_or_else(|| {
                names.push(s.clone());
                parent.push(parent.len());
                names.len() - 1
            })
        };
        fn find(p: &mut [usize], mut x: usize) -> usize {
            while p[x] != x {
                p[x] = p[p[x]];
                x = p[x];
            }
            x
        }
        for (a, b) in &lv.conns {
            let (ia, ib) = (
                id(a, &mut names, &mut parent),
                id(b, &mut names, &mut parent),
            );
            let (ra, rb) = (find(&mut parent, ia), find(&mut parent, ib));
            parent[rb] = ra;
        }
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
        for set in &sets {
            let members: Vec<&String> = set.iter().map(|&i| &names[i]).collect();
            let first = self.leaves(members[0]);
            if first.is_empty() {
                return un(
                    "connect-array",
                    format!(
                        "unknown connector (or a whole array of connectors) {}",
                        members[0]
                    ),
                );
            }
            for (suffix, vi, count) in &first {
                let (v, count) = (&self.vars[*vi], *count);
                if v.flow {
                    let mut terms = Vec::new();
                    for m in &members {
                        let inside = m.strip_prefix(pfx).is_some_and(|r| r.contains('.'));
                        terms.push((inside, format!("{m}{suffix}")));
                    }
                    let flip = terms.iter().all(|(i, _)| !i);
                    let mut s = String::new();
                    for (i, (ins, t)) in terms.iter().enumerate() {
                        let plus = *ins != flip;
                        s += &match (i, plus) {
                            (0, true) => t.clone(),
                            (0, false) => format!("-{t}"),
                            (_, true) => format!(" + {t}"),
                            (_, false) => format!(" - {t}"),
                        };
                    }
                    self.eqs.push(Equation {
                        text: format!("{s} = 0"),
                        count,
                    });
                } else {
                    if v.stream {
                        return un("stream", format!("{}{suffix}", members[0]));
                    }
                    // コネクタの中の parameter・constant は等式にしない（Rumoca の flat も同じ）
                    if matches!(v.kind, VarKind::Parameter | VarKind::Constant) {
                        continue;
                    }
                    for m in &members[1..] {
                        let other = self.leaves(m);
                        if !other.iter().any(|(s, _, _)| s == suffix) {
                            return un("connect", format!("{} and {m} differ", members[0]));
                        }
                        self.eqs.push(Equation {
                            text: format!("{}{suffix} = {m}{suffix}", members[0]),
                            count,
                        });
                    }
                }
            }
        }
        // つながっていない内側のコネクタの flow は 0
        for n in inside {
            let used = names.iter().any(|m| {
                m == n || m.starts_with(&format!("{n}.")) || n.starts_with(&format!("{m}."))
            });
            if used {
                continue;
            }
            for (suffix, vi, count) in self.leaves(n) {
                let v = &self.vars[vi];
                if v.stream {
                    return un("stream", n.clone());
                }
                if v.flow {
                    self.eqs.push(Equation {
                        text: format!("{n}{suffix} = 0"),
                        count,
                    });
                }
            }
        }
        Ok(())
    }
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

fn has_array_value(m: &ModMap) -> bool {
    m.values().any(|x| {
        (!x.each
            && x.val
                .as_ref()
                .is_some_and(|b| b.node.kind() == K::ArrayExpr))
            || has_array_value(&x.sub)
    })
}

fn binop(op: K, a: Val, b: Val) -> Option<Val> {
    use Val::*;
    Some(match (op, a, b) {
        (K::Plus | K::DotPlus, I(x), I(y)) => I(x.checked_add(y)?),
        (K::Minus | K::DotMinus, I(x), I(y)) => I(x.checked_sub(y)?),
        (K::Star | K::DotStar, I(x), I(y)) => I(x.checked_mul(y)?),
        (K::Caret | K::DotCaret, I(x), I(y)) if (0..40).contains(&y) => I(x.checked_pow(y as u32)?),
        (K::Plus | K::DotPlus, _, _) => R(a.f() + b.f()),
        (K::Minus | K::DotMinus, _, _) => R(a.f() - b.f()),
        (K::Star | K::DotStar, _, _) => R(a.f() * b.f()),
        (K::Slash | K::DotSlash, _, _) => R(a.f() / b.f()),
        (K::Caret | K::DotCaret, _, _) => R(a.f().powf(b.f())),
        (K::EqEq, E(c, i), E(d, j)) => B(c == d && i == j),
        (K::Ne, E(c, i), E(d, j)) => B(c != d || i != j),
        (K::Lt, _, _) => B(a.f() < b.f()),
        (K::Le, _, _) => B(a.f() <= b.f()),
        (K::Gt, _, _) => B(a.f() > b.f()),
        (K::Ge, _, _) => B(a.f() >= b.f()),
        (K::EqEq, _, _) => B(a.f() == b.f()),
        (K::Ne, _, _) => B(a.f() != b.f()),
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

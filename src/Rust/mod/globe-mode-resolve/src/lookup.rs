//! Modelica 3.6 の 5.3 の名前の探索。

use crate::{ClassId, Import, Library, PathRef, Reason, Redecl, Ref, RefKind, Resolution, St};

/// 組み込みの名前（型・関数・演算子・変数）。ここにあるものは、探索で見つからなかったときだけ使う
const BUILTINS: &[&str] = &[
    "Real",
    "Integer",
    "Boolean",
    "String",
    "StateSelect",
    "ExternalObject",
    "Clock",
    "AssertionLevel",
    "time",
    "Connections",
    "abs",
    "sign",
    "sqrt",
    "min",
    "max",
    "sin",
    "cos",
    "tan",
    "asin",
    "acos",
    "atan",
    "atan2",
    "sinh",
    "cosh",
    "tanh",
    "exp",
    "log",
    "log10",
    "floor",
    "ceil",
    "integer",
    "mod",
    "rem",
    "div",
    "delay",
    "cardinality",
    "semiLinear",
    "smooth",
    "noEvent",
    "pre",
    "edge",
    "change",
    "reinit",
    "sample",
    "terminal",
    "assert",
    "terminate",
    "ndims",
    "size",
    "scalar",
    "vector",
    "matrix",
    "array",
    "zeros",
    "ones",
    "fill",
    "linspace",
    "identity",
    "diagonal",
    "cat",
    "transpose",
    "outerProduct",
    "symmetric",
    "cross",
    "skew",
    "sum",
    "product",
    "homotopy",
    "inStream",
    "actualStream",
    "spatialDistribution",
    "getInstanceName",
    "previous",
    "hold",
    "subSample",
    "superSample",
    "shiftSample",
    "backSample",
    "firstTick",
    "interval",
    "initial",
    "der",
    "isPresent",
    "rooted",
    "analysisType",
];

#[derive(Clone, Copy, PartialEq)]
enum Mode {
    Normal,
    /// 自分の受け継いだ要素は見ない（`extends` の基底の名前）
    Extends,
    /// 最上位から
    Import,
}

#[derive(Clone, Copy, PartialEq)]
enum Want {
    /// 全部の部分がクラス
    Class,
    /// 最後がクラスでも部品でもよい（式・`import`）。途中で部品に当たったらそこで止める
    Any,
    /// `Any` に加え、途中の部品は型の中へたどる（式の成分参照）
    Path,
}

#[derive(Default)]
struct Walk {
    stack: Vec<ClassId>,
    cycle: bool,
    redecl: bool,
    base_fail: bool,
    /// 継承元が組み込みの型だった（`type V = Real` の `start` など、属性は探索の外）
    builtin: bool,
}

impl Library {
    /// 成分参照は続きまで（`a.b.c` の `c`）
    pub fn resolve(&self, r: &Ref) -> Resolution {
        self.resolve_with(r, Want::Path)
    }

    /// 成分参照は先頭の部品で止める（S272 の解決。続きは調べない）
    pub fn resolve_head(&self, r: &Ref) -> Resolution {
        self.resolve_with(r, Want::Any)
    }

    fn resolve_with(&self, r: &Ref, expr: Want) -> Resolution {
        if r.local {
            return Resolution::Local;
        }
        let (mode, want) = match r.kind {
            RefKind::Type => (Mode::Normal, Want::Class),
            RefKind::Extends => (Mode::Extends, Want::Class),
            RefKind::Import => (Mode::Import, Want::Any),
            RefKind::Expr => (Mode::Normal, expr),
        };
        self.resolve_path(Some(r.scope), &r.parts, r.global, mode, want)
    }

    /// `A.B.C` から探す最上位のクラス
    pub fn class_by_name(&self, name: &str) -> Option<ClassId> {
        let mut it = name.split('.');
        let mut cur = *self.roots.get(it.next()?)?;
        for p in it {
            cur = *self.classes[cur].classes.get(p)?;
        }
        Some(cur)
    }

    pub fn qualified_name(&self, mut c: ClassId) -> String {
        let mut v = vec![self.classes[c].name.as_str()];
        while let Some(p) = self.classes[c].parent {
            v.push(&self.classes[p].name);
            c = p;
        }
        v.reverse();
        v.join(".")
    }

    fn resolve_path(
        &self,
        scope: Option<ClassId>,
        parts: &[String],
        global: bool,
        mode: Mode,
        want: Want,
    ) -> Resolution {
        let mut w = Walk::default();
        let Some(head) = parts.first() else {
            return Resolution::Unresolved(Reason::NotFound);
        };
        // `.sqrt` のように組み込みにも先頭の `.` がつく
        let root = |n: &str| {
            let c = self.roots.get(n).map(|&c| Resolution::Class(c));
            c.or_else(|| (global && BUILTINS.contains(&n)).then_some(Resolution::Builtin))
        };
        let found = match (scope, global || mode == Mode::Import) {
            (Some(s), false) => self.lookup(s, head, mode == Mode::Extends, &mut w),
            _ => root(head),
        };
        let Some(mut hit) = found else {
            return Resolution::Unresolved(reason(&w, false));
        };
        for p in &parts[1..] {
            match hit {
                Resolution::Class(c) => {
                    hit = match self.member(c, p, &mut w) {
                        Some(h) => h,
                        None => {
                            return Resolution::Unresolved(reason(&w, self.classes[c].replaceable));
                        }
                    }
                }
                Resolution::Component(c, k) if want == Want::Path => {
                    hit = match self.step(c, k, p, &mut w) {
                        Ok(h) => h,
                        Err(r) => return Resolution::Unresolved(r),
                    }
                }
                Resolution::Component(..) if want == Want::Any => return hit,
                Resolution::Builtin => return hit,
                _ => return Resolution::Unresolved(Reason::NotAClass),
            }
        }
        match (&hit, want) {
            (Resolution::Component(..), Want::Class) => Resolution::Unresolved(Reason::NotAClass),
            _ => hit,
        }
    }

    /// `scope` から外へ。`first_declared_only` は最初のクラスの受け継ぎを見ない
    fn lookup(
        &self,
        scope: ClassId,
        name: &str,
        first_declared_only: bool,
        w: &mut Walk,
    ) -> Option<Resolution> {
        let mut cur = Some(scope);
        let mut first = first_declared_only;
        while let Some(c) = cur {
            let cl = &self.classes[c];
            let hit = if first {
                cl.classes.get(name).map(|&id| Resolution::Class(id))
            } else {
                self.member(c, name, w)
            };
            if hit.is_some() {
                return hit;
            }
            if let Some(h) = self.import_hit(c, name, w) {
                return Some(h);
            }
            first = false;
            if cl.encapsulated {
                break;
            }
            cur = cl.parent;
        }
        self.roots
            .get(name)
            .map(|&c| Resolution::Class(c))
            .or_else(|| BUILTINS.contains(&name).then_some(Resolution::Builtin))
    }

    /// 宣言した要素と、受け継いだ要素（`import` は含まない）
    fn member(&self, c: ClassId, name: &str, w: &mut Walk) -> Option<Resolution> {
        let cl = &self.classes[c];
        if let Some(&id) = cl.classes.get(name) {
            return Some(Resolution::Class(id));
        }
        if let Some(&i) = cl.comp_index.get(name) {
            return Some(Resolution::Component(c, i));
        }
        if w.stack.contains(&c) {
            w.cycle = true;
            return None;
        }
        w.stack.push(c);
        let mut found = None;
        for (i, b) in cl.bases.iter().enumerate() {
            w.redecl |= b.redeclare;
            if let Some(id) = self.redecl_hit(c, &b.redecls, name) {
                found = Some(Resolution::Class(id));
                break;
            }
            match self.base(c, i) {
                Ok(bid) => {
                    if let Some(h) = self.member(bid, name, w) {
                        found = Some(h);
                        break;
                    }
                }
                Err(Reason::Cycle) => w.cycle = true,
                Err(Reason::Redeclare) => w.redecl = true,
                Err(_) if matches!(b.state.get(), St::Builtin) => w.builtin = true,
                Err(_) => w.base_fail = true,
            }
        }
        w.stack.pop();
        found
    }

    /// `c` の `i` 番目の継承元。結果は覚える（解決中にまた来たら輪）
    fn base(&self, c: ClassId, i: usize) -> Result<ClassId, Reason> {
        let b = &self.classes[c].bases[i];
        match b.state.get() {
            St::Done(id) => return Ok(id),
            St::Fail(r) => return Err(r),
            St::Busy => return Err(Reason::Cycle),
            St::Builtin => return Err(Reason::NotAClass),
            St::Todo => {}
        }
        b.state.set(St::Busy);
        let r = if b.class_ext {
            self.class_extends_base(c)
        } else {
            self.resolve_path(Some(c), &b.path, b.global, Mode::Extends, Want::Class)
        };
        let st = match r {
            Resolution::Class(id) => St::Done(id),
            Resolution::Builtin => St::Builtin,
            Resolution::Unresolved(r) => St::Fail(r),
            _ => St::Fail(Reason::NotAClass),
        };
        b.state.set(st);
        match st {
            St::Done(id) => Ok(id),
            St::Fail(r) => Err(r),
            _ => Err(Reason::NotAClass),
        }
    }

    /// `scope` の中の名前 `p` が指すクラス（結果は覚える。解決中にまた来たら `Busy`）
    fn target_state(&self, scope: ClassId, p: &PathRef) -> St {
        if let St::Todo = p.state.get() {
            p.state.set(St::Busy);
            let r = self.resolve_path(Some(scope), &p.path, p.global, Mode::Normal, Want::Class);
            p.state.set(match r {
                Resolution::Class(id) => St::Done(id),
                Resolution::Builtin => St::Builtin,
                Resolution::Unresolved(r) => St::Fail(r),
                _ => St::Fail(Reason::NotAClass),
            });
        }
        p.state.get()
    }

    /// 修飾の `redeclare package name = X` があれば `X`（`scope` の中で解決）
    fn redecl_hit(&self, scope: ClassId, redecls: &[Redecl], name: &str) -> Option<ClassId> {
        let r = redecls.iter().find(|r| r.name == name)?;
        r.targets
            .iter()
            .find_map(|t| match self.target_state(scope, t) {
                St::Done(id) => Some(id),
                _ => None,
            })
    }

    /// 部品 `c` の `k` 番目の型の中で、続きの名前 `name` を探す
    fn step(&self, c: ClassId, k: usize, name: &str, w: &mut Walk) -> Result<Resolution, Reason> {
        let comp = &self.classes[c].components[k];
        if let Some(id) = self.redecl_hit(c, &comp.redecls, name) {
            return Ok(Resolution::Class(id));
        }
        let mut why = Reason::NotAClass;
        for t in &comp.tys {
            match self.target_state(c, t) {
                St::Done(tc) => match self.member(tc, name, w) {
                    Some(h) => return Ok(h),
                    None if w.builtin => return Ok(Resolution::Builtin),
                    None => why = reason(w, self.classes[tc].replaceable),
                },
                St::Builtin => return Ok(Resolution::Builtin),
                St::Fail(r) => why = r,
                _ => {}
            }
        }
        Err(why)
    }

    /// `class extends X` の `X`: 外側のクラスの継承元から探す（外側自身の宣言は自分）
    fn class_extends_base(&self, c: ClassId) -> Resolution {
        let name = &self.classes[c].name;
        let Some(p) = self.classes[c].parent else {
            return Resolution::Unresolved(Reason::NotFound);
        };
        let mut w = Walk::default();
        w.stack.push(p);
        for i in 0..self.classes[p].bases.len() {
            match self.base(p, i) {
                Ok(bid) => {
                    if let Some(h @ Resolution::Class(_)) = self.member(bid, name, &mut w) {
                        return h;
                    }
                }
                Err(r) if r != Reason::NotFound => return Resolution::Unresolved(r),
                Err(_) => {}
            }
        }
        Resolution::Unresolved(reason(&w, true))
    }

    /// `import A.B.*` / `A.B.{C}` の `A.B`（クラス）
    fn import_base(&self, p: &PathRef, owner: ClassId) -> Option<ClassId> {
        if let St::Todo = p.state.get() {
            let r = self.resolve_path(Some(owner), &p.path, true, Mode::Import, Want::Class);
            p.state.set(match r {
                Resolution::Class(id) => St::Done(id),
                _ => St::Fail(Reason::NotFound),
            });
        }
        match p.state.get() {
            St::Done(id) => Some(id),
            _ => None,
        }
    }

    fn import_hit(&self, c: ClassId, name: &str, w: &mut Walk) -> Option<Resolution> {
        let imports = &self.classes[c].imports;
        let via =
            |path: &[String]| match self.resolve_path(None, path, true, Mode::Import, Want::Any) {
                h @ (Resolution::Class(_) | Resolution::Component(..)) => Some(h),
                _ => None,
            };
        for i in imports {
            let hit = match i {
                Import::Alias(a, p) if a == name => via(p),
                Import::Qualified(p) if p.last().is_some_and(|l| l == name) => via(p),
                Import::Multi(p, names) if names.iter().any(|n| n == name) => {
                    self.import_base(p, c).and_then(|b| self.member(b, name, w))
                }
                _ => None,
            };
            if hit.is_some() {
                return hit;
            }
        }
        // `A.*` は最後（名前つきの import が先）
        imports.iter().find_map(|i| match i {
            Import::Unqualified(p) => self.import_base(p, c).and_then(|b| self.member(b, name, w)),
            _ => None,
        })
    }
}

fn reason(w: &Walk, replaceable: bool) -> Reason {
    if w.cycle {
        Reason::Cycle
    } else if w.redecl || replaceable {
        Reason::Redeclare
    } else if w.base_fail {
        Reason::Inherited
    } else {
        Reason::NotFound
    }
}

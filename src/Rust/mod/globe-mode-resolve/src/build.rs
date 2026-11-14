//! 構文の木から、クラスの木と名前の使われ方（`Ref`）を取り出す。ファイルごとに木を捨てるので、全体の木は持たない。

use crate::{Class, ClassId, Component, Import, PathRef, Ref, RefKind, St};
use globe_mode_syntax::ast::{self, Element, ImportKind};
use globe_mode_syntax::{SyntaxKind, SyntaxNode, parse};
use rowan::TextRange;
use std::cell::Cell;
use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::path::{Path, PathBuf};

#[derive(Debug, Default)]
pub struct Library {
    pub classes: Vec<Class>,
    pub files: Vec<String>,
    pub refs: Vec<Ref>,
    /// 重複・`within` の親が無い・構文の誤りの数（`(ファイル, 文言)`）
    pub diagnostics: Vec<(u32, String)>,
    pub(crate) roots: HashMap<String, ClassId>,
}

struct RawRef {
    range: TextRange,
    kind: RefKind,
    parts: Vec<String>,
    global: bool,
    local: bool,
}

struct Raw {
    class: Class,
    children: Vec<Raw>,
    refs: Vec<RawRef>,
}

impl PathRef {
    fn new(path: Vec<String>, global: bool, redeclare: bool) -> Self {
        PathRef {
            path,
            global,
            redeclare,
            class_ext: false,
            state: Cell::new(St::Todo),
        }
    }
}

fn mo_files(dir: &Path, out: &mut Vec<PathBuf>) -> std::io::Result<()> {
    for e in std::fs::read_dir(dir)? {
        let p = e?.path();
        if p.is_dir() {
            mo_files(&p, out)?;
        } else if p.extension().is_some_and(|x| x == "mo") {
            out.push(p);
        }
    }
    Ok(())
}

impl Library {
    /// ディレクトリの下の `.mo` をすべて読む（`MODELICAPATH` の 1 つぶん。複数なら `from_dirs`）
    pub fn from_dir(dir: &Path) -> std::io::Result<Library> {
        Self::from_dirs(&[dir])
    }

    pub fn from_dirs(dirs: &[&Path]) -> std::io::Result<Library> {
        let mut files = Vec::new();
        for d in dirs {
            let before = files.len();
            mo_files(d, &mut files)?;
            files[before..].sort();
        }
        let mut lib = Library::default();
        let mut raws = Vec::new();
        for f in files {
            // 読めない・UTF-8 でないファイルは省く
            if let Ok(text) = std::fs::read_to_string(&f) {
                lib.add(f.display().to_string(), &text, &mut raws);
            }
        }
        lib.link(raws);
        Ok(lib)
    }

    /// 試験・エディタの文書用: `(名前, 本文)` の列から作る
    pub fn from_sources<'a>(sources: impl IntoIterator<Item = (&'a str, &'a str)>) -> Library {
        let mut lib = Library::default();
        let mut raws = Vec::new();
        for (name, text) in sources {
            lib.add(name.to_string(), text, &mut raws);
        }
        lib.link(raws);
        lib
    }

    fn add(&mut self, name: String, text: &str, raws: &mut Vec<(Vec<String>, Raw)>) {
        let file = self.files.len() as u32;
        self.files.push(name);
        let p = parse(text);
        if let Some(e) = p.errors.first() {
            self.diagnostics.push((
                file,
                format!("syntax error: {} ({} total)", e.message, p.errors.len()),
            ));
        }
        let Some(sd) = ast::StoredDefinition::cast(p.syntax()) else {
            return;
        };
        let within = sd.within().unwrap_or_default();
        for c in sd.classes() {
            if let Some(r) = extract(&c, file) {
                raws.push((within.clone(), r));
            }
        }
    }

    /// `within` の浅い順に木へ付ける（`within A.B;` の `A.B` は先に付いている）
    fn link(&mut self, mut raws: Vec<(Vec<String>, Raw)>) {
        raws.sort_by_key(|(w, _)| w.len());
        for (within, raw) in raws {
            let file = raw.class.file;
            let parent = if within.is_empty() {
                None
            } else {
                let mut cur = self.roots.get(&within[0]).copied();
                for p in &within[1..] {
                    cur = cur.and_then(|c| self.classes[c].classes.get(p).copied());
                }
                if cur.is_none() {
                    self.diagnostics
                        .push((file, format!("within {} is not defined", within.join("."))));
                    continue;
                }
                cur
            };
            let name = raw.class.name.clone();
            let id = self.alloc(raw, parent);
            self.attach(file, parent, name, id);
        }
    }

    fn attach(&mut self, file: u32, parent: Option<ClassId>, name: String, id: ClassId) {
        let map = match parent {
            Some(p) => &mut self.classes[p].classes,
            None => &mut self.roots,
        };
        match map.entry(name) {
            Entry::Occupied(e) => {
                let msg = format!("duplicate class {}", e.key());
                self.diagnostics.push((file, msg));
            }
            Entry::Vacant(e) => {
                e.insert(id);
            }
        }
    }

    fn alloc(&mut self, raw: Raw, parent: Option<ClassId>) -> ClassId {
        let id = self.classes.len();
        let mut class = raw.class;
        class.parent = parent;
        let file = class.file;
        self.classes.push(class);
        for r in raw.refs {
            self.refs.push(Ref {
                file,
                range: r.range,
                scope: id,
                kind: r.kind,
                parts: r.parts,
                global: r.global,
                local: r.local,
            });
        }
        for ch in raw.children {
            let name = ch.class.name.clone();
            let cid = self.alloc(ch, Some(id));
            self.attach(file, Some(id), name, cid);
        }
        id
    }
}

fn extract(c: &ast::ClassDef, file: u32) -> Option<Raw> {
    let name = c.name()?;
    let mut raw = Raw {
        class: Class {
            name,
            parent: None,
            file,
            name_range: c.name_range()?,
            encapsulated: c.is_encapsulated(),
            replaceable: c.is_replaceable(),
            classes: HashMap::new(),
            components: Vec::new(),
            comp_index: HashMap::new(),
            bases: Vec::new(),
            imports: Vec::new(),
        },
        children: Vec::new(),
        refs: Vec::new(),
    };
    let mut locals = Vec::new();
    if c.is_class_extends() {
        let mut b = PathRef::new(vec![raw.class.name.clone()], false, false);
        b.class_ext = true;
        raw.class.bases.push(b);
    }
    if let Some(sb) = c.short_body() {
        for (i, n) in sb
            .syntax()
            .children()
            .filter_map(ast::Name::cast)
            .enumerate()
        {
            let kind = if i == 0 && sb.base().is_some() {
                raw.class
                    .bases
                    .push(PathRef::new(n.parts(), n.is_global(), sb.has_redeclare()));
                RefKind::Extends
            } else {
                RefKind::Type
            };
            raw.refs.push(name_ref(&n, kind));
        }
        for (l, r) in sb.enum_literals() {
            add_component(&mut raw.class, l, r, true);
        }
    }
    let mut public = true;
    for e in c.elements() {
        match e {
            Element::Visibility(p) => public = p,
            Element::Class(ch) => raw.children.extend(extract(&ch, file)),
            Element::Extends(x) => {
                if let Some(n) = x.name() {
                    raw.class
                        .bases
                        .push(PathRef::new(n.parts(), n.is_global(), x.has_redeclare()));
                    raw.refs.push(name_ref(&n, RefKind::Extends));
                }
            }
            Element::Import(i) => {
                if let Some(n) = i.name() {
                    raw.refs.push(name_ref(&n, RefKind::Import));
                }
                match i.kind() {
                    Some(ImportKind::Alias(a, p)) => raw.class.imports.push(Import::Alias(a, p)),
                    Some(ImportKind::Qualified(p)) => raw.class.imports.push(Import::Qualified(p)),
                    Some(ImportKind::Unqualified(p)) => raw
                        .class
                        .imports
                        .push(Import::Unqualified(PathRef::new(p, true, false))),
                    Some(ImportKind::Multi(p, names)) => raw
                        .class
                        .imports
                        .push(Import::Multi(PathRef::new(p, true, false), names)),
                    None => {}
                }
            }
            Element::Component(cc) => {
                for n in cc.syntax().children().filter_map(ast::Name::cast) {
                    raw.refs.push(name_ref(&n, RefKind::Type));
                }
                for d in cc.declarations() {
                    if let Some((n, r)) = d.name() {
                        add_component(&mut raw.class, n, r, public);
                    }
                }
            }
        }
    }
    // 式を持つ所: 宣言の修飾・添字・条件、`extends` の修飾、節（`annotation` は読まない）
    for ch in c.syntax().children() {
        use SyntaxKind::*;
        if matches!(
            ch.kind(),
            ComponentClause
                | ExtendsClause
                | ShortBody
                | ClassModification
                | EquationSection
                | AlgorithmSection
        ) {
            collect(&ch, &mut raw.refs, &mut locals);
        }
    }
    Some(raw)
}

fn add_component(c: &mut Class, name: String, range: TextRange, public: bool) {
    c.comp_index
        .entry(name.clone())
        .or_insert(c.components.len());
    c.components.push(Component {
        name,
        range,
        public,
    });
}

fn name_ref(n: &ast::Name, kind: RefKind) -> RawRef {
    RawRef {
        range: n.range(),
        kind,
        parts: n.parts(),
        global: n.is_global(),
        local: false,
    }
}

/// 式の中の成分参照を集める。`for` の添字はその節点の中だけ有効
fn collect(n: &SyntaxNode, out: &mut Vec<RawRef>, locals: &mut Vec<String>) {
    use SyntaxKind::*;
    if matches!(n.kind(), Annotation | ClassDef) {
        return;
    }
    let mark = locals.len();
    for ch in n.children().filter(|c| c.kind() == ForIndices) {
        for fi in ch.children().filter(|c| c.kind() == ForIndex) {
            let first = fi
                .children_with_tokens()
                .filter_map(|e| e.into_token())
                .find(|t| t.kind() == Ident);
            locals.extend(first.map(|t| t.text().to_string()));
        }
    }
    if let Some(c) = ast::ComponentRef::cast(n.clone()) {
        let parts = c.parts();
        let global = c.is_global();
        out.push(RawRef {
            range: c.range(),
            kind: RefKind::Expr,
            local: !global && parts.first().is_some_and(|h| locals.contains(h)),
            parts,
            global,
        });
    }
    for ch in n.children() {
        collect(&ch, out, locals);
    }
    locals.truncate(mark);
}

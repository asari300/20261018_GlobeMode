//! Track B の B3 の最初の一切れ（S278）: モデルを平らな変数と方程式の列にする。
//!
//! - 入力は `globe-mode-syntax` の CST と `globe-mode-resolve` の名前解決だけ（`Project` が両方を持つ）。
//! - 範囲: 部品の入れ子（`a.b.x`）、`extends`、修飾（外側が勝つ）、parameter の束縛、方程式の名前の付け替え、
//!   `connect`（potential は等式、flow は和が 0、つながない内側の flow は 0。`stream` は断る）、
//!   大きさが定数の配列（部品の配列は要素ごとに展開、`for` の方程式も展開）。
//! - S284: 条件つきの宣言（条件が定数に評価できるとき。偽なら宣言と、それにつながる `connect` を落とす）、
//!   定数の条件の `if` の方程式（選んだ枝だけを平らにする。`connect` も入る）、`inner`/`outer`（`outer` は部品を作らず、
//!   外側へたどって最初の同じ名前の `inner` へ参照を付け替える）、レコードの束縛（`R r = R(a = 1)`・`r = other` は欄ごとの修飾）、
//!   `[:]` と添字の大きさ（行列・範囲・`fill`/`zeros`・パッケージの定数・列挙の型・後ろで宣言した配列の `size`）。
//! - 式は評価せず、正規化した印字として残す（評価できない条件の `if` の方程式・`when`・関数の呼び出し）。
//!   大きさ・`for` の範囲・添字・定数の条件だけ、定数の式（リテラル・parameter・列挙・`size`・四則・`max`/`min` など）を評価する。
//! - 断るもの（`Unsupported::kind`）: `redeclare`・`replaceable`、`inner` の無い `outer`、評価できない条件の宣言、外部関数、
//!   `stream`、`expandable`、`class extends`、形の決まらない `[:]`、関数の戻り値のレコード、評価できない条件の `if`/`when` の中の `connect` など。
//!
//! 既存のフロントエンド（Rumoca）にはつながない（doc/adr/0039-native-cst-parser.md の付記）。

mod flatten;

use globe_mode_resolve::{ClassId, Library, Resolution};
use globe_mode_syntax::ast::{self, Element};
use globe_mode_syntax::{SyntaxKind, parse};
use std::cell::RefCell;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::rc::Rc;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum VarKind {
    Constant,
    Parameter,
    /// `discrete` と、Real 以外の型（Integer・Boolean・String・列挙）。Real を `when` で代入するものは区別しない
    Discrete,
    Continuous,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Causality {
    #[default]
    Acausal,
    Input,
    Output,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Var {
    /// `a.b[2].x`（配列の変数は添字なしの 1 個。部品の配列は要素ごと）
    pub name: String,
    pub kind: VarKind,
    pub causality: Causality,
    pub flow: bool,
    pub stream: bool,
    /// `Real`・`Integer`・`Boolean`・`String`、列挙は完全修飾名
    pub ty: String,
    pub dims: Vec<usize>,
    pub start: Option<String>,
    /// 束縛 `= 式`（parameter・constant・input 以外の束縛は、DAE では方程式になる。`FlatModel::binding_equations`）
    pub binding: Option<String>,
    /// `start` 以外の属性の修飾（`unit`・`min`・`fixed` など。名前順）
    pub attrs: Vec<(String, String)>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Equation {
    pub text: String,
    /// スカラーの方程式の数。配列の方程式は配列の大きさ、`if`/`when` は最初の枝、関数の呼び出しの文は 0
    pub count: usize,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FlatModel {
    pub name: String,
    pub vars: Vec<Var>,
    /// 方程式（`if` を含む。`when` と変数の束縛は含まない。Rumoca の flat の `equations` と同じ分け方）
    pub equations: Vec<Equation>,
    /// `when` の方程式（初期でないもの）
    pub when_equations: Vec<Equation>,
    pub initial_equations: Vec<Equation>,
    /// `algorithm` 節の印字（数えない）
    pub algorithms: Vec<String>,
}

impl FlatModel {
    pub fn equation_count(&self) -> usize {
        self.equations.iter().map(|e| e.count).sum()
    }
    /// parameter・constant・input 以外の変数の束縛（`Real y = x + 1;`）が作る方程式のスカラーの数
    pub fn binding_equations(&self) -> usize {
        self.vars
            .iter()
            .filter(|v| {
                v.binding.is_some()
                    && matches!(v.kind, VarKind::Continuous | VarKind::Discrete)
                    && v.causality != Causality::Input
            })
            .map(|v| v.dims.iter().product::<usize>())
            .sum()
    }
    pub fn var_names(&self) -> Vec<&str> {
        self.vars.iter().map(|v| v.name.as_str()).collect()
    }
}

/// 平らにできなかった理由。`kind` は数えるための短い分類
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Unsupported {
    pub kind: &'static str,
    pub detail: String,
}

impl std::fmt::Display for Unsupported {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.kind, self.detail)
    }
}

pub(crate) type Res<T> = Result<T, Unsupported>;

pub(crate) fn un<T>(kind: &'static str, detail: impl Into<String>) -> Res<T> {
    Err(Unsupported {
        kind,
        detail: detail.into(),
    })
}

/// ライブラリ全体の構文と名前解決。`flatten` は何度でも呼べる
#[derive(Debug)]
pub struct Project {
    pub lib: Library,
    classes: Vec<Option<ast::ClassDef>>,
    bases: Vec<Vec<ClassId>>,
    /// `(ファイル, 名前の始まり, 終わり)` → 解決の結果（`Library::refs` のすべて）
    res: HashMap<(u32, u32, u32), Resolution>,
    lineage: RefCell<HashMap<ClassId, Rc<Vec<ClassId>>>>,
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

impl Project {
    /// ディレクトリの下の `.mo` をすべて読む（`Library::from_dirs` と同じ順）
    pub fn from_dirs(dirs: &[&Path]) -> std::io::Result<Project> {
        Self::from_dirs_with(dirs, &[])
    }

    /// `from_dirs` に、メモリ上のファイル（名前と本文。ディレクトリの後ろに並ぶ）を足す（S305。`globe check` の入力）
    pub fn from_dirs_with(dirs: &[&Path], extra: &[(&str, &str)]) -> std::io::Result<Project> {
        let mut files = Vec::new();
        for d in dirs {
            let before = files.len();
            mo_files(d, &mut files)?;
            files[before..].sort();
        }
        let texts: Vec<(String, String)> = files
            .iter()
            .filter_map(|f| Some((f.display().to_string(), std::fs::read_to_string(f).ok()?)))
            .collect();
        Ok(Self::from_sources(
            texts
                .iter()
                .map(|(n, t)| (n.as_str(), t.as_str()))
                .chain(extra.iter().copied()),
        ))
    }

    pub fn from_sources<'a>(sources: impl IntoIterator<Item = (&'a str, &'a str)>) -> Project {
        let sources: Vec<_> = sources.into_iter().collect();
        let lib = Library::from_sources(sources.iter().copied());
        // クラスごとの節点を、ファイルを読み直して `(ファイル, 名前の位置)` で引く
        let mut by_pos: HashMap<(u32, u32), ast::ClassDef> = HashMap::new();
        for (file, (_, text)) in sources.iter().enumerate() {
            for n in parse(text).syntax().descendants() {
                if let Some(c) = ast::ClassDef::cast(n)
                    && let Some(r) = c.name_range()
                {
                    by_pos.insert((file as u32, u32::from(r.start())), c);
                }
            }
        }
        let classes: Vec<_> = lib
            .classes
            .iter()
            .map(|c| {
                by_pos
                    .get(&(c.file, u32::from(c.name_range.start())))
                    .cloned()
            })
            .collect();
        let res: HashMap<_, _> = lib
            .refs
            .iter()
            .map(|r| {
                (
                    (r.file, u32::from(r.range.start()), u32::from(r.range.end())),
                    lib.resolve_head(r), // 先頭の部品で止める（S272 の解決。S275 の resolve は最後の名前）
                )
            })
            .collect();
        let mut p = Project {
            lib,
            classes,
            bases: Vec::new(),
            res,
            lineage: RefCell::default(),
        };
        p.bases = (0..p.classes.len()).map(|c| p.base_ids(c)).collect();
        p
    }

    fn base_ids(&self, cid: ClassId) -> Vec<ClassId> {
        let Some(cd) = &self.classes[cid] else {
            return Vec::new();
        };
        let file = self.lib.classes[cid].file;
        let mut names = Vec::new();
        names.extend(cd.short_body().and_then(|s| s.base()));
        for e in cd.elements() {
            if let Element::Extends(x) = e {
                names.extend(x.name());
            }
        }
        names
            .iter()
            .filter_map(|n| match self.res_of(file, n.range()) {
                Some(Resolution::Class(b)) => Some(*b),
                _ => None,
            })
            .collect()
    }

    pub(crate) fn res_of(&self, file: u32, r: rowan::TextRange) -> Option<&Resolution> {
        self.res
            .get(&(file, u32::from(r.start()), u32::from(r.end())))
    }

    pub(crate) fn class(&self, cid: ClassId) -> Option<&ast::ClassDef> {
        self.classes.get(cid)?.as_ref()
    }

    /// `cid` と、`extends`・short class の基底を再帰でたどった全部（輪は 1 回で止める）
    pub(crate) fn lineage(&self, cid: ClassId) -> Rc<Vec<ClassId>> {
        if let Some(l) = self.lineage.borrow().get(&cid) {
            return l.clone();
        }
        let mut out = Vec::new();
        let mut stack = vec![cid];
        while let Some(c) = stack.pop() {
            if !out.contains(&c) {
                out.push(c);
                stack.extend(self.bases[c].iter().copied());
            }
        }
        let l = Rc::new(out);
        self.lineage.borrow_mut().insert(cid, l.clone());
        l
    }

    /// 完全修飾名のクラスを平らにする
    pub fn flatten(&self, model: &str) -> Result<FlatModel, Unsupported> {
        let Some(cid) = self.lib.class_by_name(model) else {
            return un("not-found", model);
        };
        flatten::run(self, cid, model)
    }

    /// クラスの種類の字句（`model`・`package` など）
    pub fn kind_of(&self, cid: ClassId) -> Option<SyntaxKind> {
        self.class(cid)?.kind_token().map(|t| t.kind())
    }

    pub fn is_partial(&self, cid: ClassId) -> bool {
        self.class(cid).is_some_and(|c| {
            c.syntax()
                .children_with_tokens()
                .filter_map(|e| e.into_token())
                .take_while(|t| t.kind() != SyntaxKind::Ident)
                .any(|t| t.kind() == SyntaxKind::KwPartial)
        })
    }
}

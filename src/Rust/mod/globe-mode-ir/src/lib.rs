//! フロントエンドに依存しない中立 IR。
//!
//! Rumoca（Phase A）や自前コア（Track B）の型は、ここに変換してから外に出す。
//! 型はすべて `Eq` を実装する。salsa のクエリの「防火壁」として、
//! 空白やコメントだけの編集では値が変わらないようにするため（ADR-0005）。
//! そのため、ソース上の位置（TextRange）は持たせるが、比較から外したい場合は
//! 呼び出し側で位置を落とした射影を使う（P2 で `outline` クエリを作るときに決める）。

use std::collections::BTreeMap;

use globe_mode_core::TextRange;
use serde::Serialize;

/// 1 ファイル分のアウトライン。
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize)]
pub struct Outline {
    /// `within` 句（無ければ None）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub within: Option<String>,
    pub classes: Vec<ClassOutline>,
}

impl Outline {
    /// ソース上の位置をすべて 0 にした写し。
    ///
    /// salsa の防火壁（ADR-0005）に使う。空白やコメントだけの編集では位置だけが変わるので、
    /// この写しは前と等しくなり、下流（クラスの索引、connect グラフ）は再計算されない。
    pub fn without_ranges(&self) -> Outline {
        fn strip(c: &ClassOutline) -> ClassOutline {
            ClassOutline {
                range: TextRange::default(),
                components: c
                    .components
                    .iter()
                    .map(|x| ComponentOutline {
                        range: TextRange::default(),
                        ..x.clone()
                    })
                    .collect(),
                connects: c
                    .connects
                    .iter()
                    .map(|x| Connect {
                        range: TextRange::default(),
                        ..x.clone()
                    })
                    .collect(),
                classes: c.classes.iter().map(strip).collect(),
                ..c.clone()
            }
        }
        Outline {
            within: self.within.clone(),
            classes: self.classes.iter().map(strip).collect(),
        }
    }

    /// 入れ子を含めたすべてのクラスを、`A.B.C` 形式の名前とともに返す。
    pub fn qualified_classes(&self) -> Vec<(String, &ClassOutline)> {
        fn walk<'a>(
            prefix: &str,
            cs: &'a [ClassOutline],
            out: &mut Vec<(String, &'a ClassOutline)>,
        ) {
            for c in cs {
                let name = if prefix.is_empty() {
                    c.name.clone()
                } else {
                    format!("{prefix}.{}", c.name)
                };
                out.push((name.clone(), c));
                walk(&name, &c.classes, out);
            }
        }
        let mut out = Vec::new();
        walk("", &self.classes, &mut out);
        out
    }
}

/// クラスの種類。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ClassKind {
    Model,
    Class,
    Block,
    Connector,
    Record,
    Type,
    Package,
    Function,
    Operator,
}

/// クラス 1 つ分のアウトライン。
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
pub struct ClassOutline {
    pub name: String,
    pub kind: ClassKind,
    pub partial: bool,
    /// 短いクラス定義の向き（例: `connector RealInput = input Real;` は Input）
    pub causality: Causality,
    pub range: TextRange,
    pub extends: Vec<String>,
    pub components: Vec<ComponentOutline>,
    /// このクラスの equation 節に直接書かれた connect 文（for や if の中は P2 で扱う）。
    pub connects: Vec<Connect>,
    pub classes: Vec<ClassOutline>,
}

/// コンポーネント（変数・部品）の宣言。
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
pub struct ComponentOutline {
    pub name: String,
    pub type_name: String,
    /// `flow` / `stream`（コネクタの中の変数）
    pub prefix: ConnectPrefix,
    /// `input` / `output`
    pub causality: Causality,
    pub range: TextRange,
}

/// コネクタの中の変数の種類（MLS 9.1）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ConnectPrefix {
    /// 何も付いていない（potential）
    #[default]
    Potential,
    Flow,
    Stream,
}

/// 向き。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Causality {
    #[default]
    None,
    Input,
    Output,
}

/// `connect(lhs, rhs)`。connect グラフの辺になる。
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
pub struct Connect {
    pub lhs: String,
    pub rhs: String,
    pub range: TextRange,
}

/// DAE の変数を、配列を 1 個として数えたもの（Rumoca の CLI の `-v` と同じ粒度）。
///
/// [`BalanceCount`] は配列をスカラーに展開して数えるので、配列を含むモデルでは数が合わない。
/// Rumoca の CLI との突き合わせにはこちらを使う。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize)]
pub struct DaeCounts {
    pub states: usize,
    pub algebraics: usize,
    pub outputs: usize,
    pub inputs: usize,
    pub parameters: usize,
    pub continuous_equations: usize,
}

/// 方程式数と変数数（S5 の検査に使う）。配列はスカラーに展開して数える（MLS 4.7 の釣り合い）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize)]
pub struct BalanceCount {
    pub equations: usize,
    pub unknowns: usize,
    pub states: usize,
    pub algebraics: usize,
    pub outputs: usize,
    pub discrete: usize,
}

impl BalanceCount {
    pub fn is_balanced(&self) -> bool {
        self.equations == self.unknowns
    }
}

/// 組み込みの型（MLS 4.9）。
pub const BUILTIN_TYPES: [&str; 5] = ["Real", "Integer", "Boolean", "String", "Clock"];

/// クラスの索引: 完全修飾名（`A.B.C`）→ クラス。
///
/// 複数のファイルのアウトラインを `within` に従ってまとめたもの。名前解決は
/// Modelica の字句的なスコープ（内側のクラスから外側へ）だけを見る。import と
/// 継承したクラスの中の入れ子のクラスは、まだ見ない（P2 の後半で足す）。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ClassIndex {
    classes: BTreeMap<String, ClassOutline>,
}

impl ClassIndex {
    pub fn from_outlines<'a>(outlines: impl IntoIterator<Item = &'a Outline>) -> Self {
        let mut classes = BTreeMap::new();
        for o in outlines {
            let prefix = o.within.as_deref().unwrap_or("");
            for (name, c) in o.qualified_classes() {
                let qualified = if prefix.is_empty() {
                    name
                } else {
                    format!("{prefix}.{name}")
                };
                // 入れ子のクラスは別の項目として持つので、ここでは中身を空にして重複を避ける
                classes.insert(
                    qualified,
                    ClassOutline {
                        classes: Vec::new(),
                        ..c.clone()
                    },
                );
            }
        }
        Self { classes }
    }

    pub fn get(&self, qualified: &str) -> Option<&ClassOutline> {
        self.classes.get(qualified)
    }

    pub fn len(&self) -> usize {
        self.classes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.classes.is_empty()
    }

    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.classes.keys().map(String::as_str)
    }

    /// `scope`（完全修飾名のクラス）の中から見た型名 `name` を、完全修飾名に解決する。
    ///
    /// `A.B.C` の中の `D.E` は、`A.B.C.D.E` → `A.B.D.E` → `A.D.E` → `D.E` の順に探す。
    /// 組み込みの型と、見つからない型は None。
    pub fn resolve(&self, scope: &str, name: &str) -> Option<String> {
        let name = name.trim_start_matches('.');
        if BUILTIN_TYPES.contains(&name) {
            return None;
        }
        let mut parts: Vec<&str> = scope.split('.').filter(|p| !p.is_empty()).collect();
        loop {
            let candidate = if parts.is_empty() {
                name.to_string()
            } else {
                format!("{}.{name}", parts.join("."))
            };
            if self.classes.contains_key(&candidate) {
                return Some(candidate);
            }
            parts.pop()?;
        }
    }

    /// クラスのコンポーネント（extends で継承したものを含む）。
    /// 返す型名は、それぞれの宣言の場所から解決した完全修飾名（解決できなければ元の名前）。
    pub fn interface(&self, qualified: &str) -> Vec<ResolvedComponent> {
        let mut out = Vec::new();
        self.collect_interface(qualified, &mut out, 0);
        out
    }

    fn collect_interface(&self, qualified: &str, out: &mut Vec<ResolvedComponent>, depth: usize) {
        let Some(c) = self.classes.get(qualified) else {
            return;
        };
        if depth > 32 {
            return; // 循環する extends に備える
        }
        for base in &c.extends {
            if let Some(b) = self.resolve(qualified, base) {
                self.collect_interface(&b, out, depth + 1);
            }
        }
        for comp in &c.components {
            out.push(ResolvedComponent {
                name: comp.name.clone(),
                type_name: self
                    .resolve(qualified, &comp.type_name)
                    .unwrap_or_else(|| comp.type_name.clone()),
                resolved: self.resolve(qualified, &comp.type_name).is_some(),
                prefix: comp.prefix,
                causality: comp.causality,
            });
        }
    }

    /// クラスの connect 文（extends で継承したものを含む）。
    pub fn connects(&self, qualified: &str) -> Vec<Connect> {
        let mut out = Vec::new();
        self.collect_connects(qualified, &mut out, 0);
        out
    }

    fn collect_connects(&self, qualified: &str, out: &mut Vec<Connect>, depth: usize) {
        let Some(c) = self.classes.get(qualified) else {
            return;
        };
        if depth > 32 {
            return;
        }
        for base in &c.extends {
            if let Some(b) = self.resolve(qualified, base) {
                self.collect_connects(&b, out, depth + 1);
            }
        }
        out.extend(c.connects.iter().cloned());
    }

    /// 短いクラス定義などで、組み込みの型を（extends で）受け継いでいれば、その型名。
    pub fn builtin_base(&self, qualified: &str) -> Option<&'static str> {
        let c = self.classes.get(qualified)?;
        for base in &c.extends {
            if let Some(b) = BUILTIN_TYPES.iter().find(|t| **t == base.as_str()) {
                return Some(b);
            }
            if let Some(b) = self
                .resolve(qualified, base)
                .and_then(|q| self.builtin_base(&q))
            {
                return Some(b);
            }
        }
        None
    }
}

/// [`ClassIndex::interface`] が返すコンポーネント。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedComponent {
    pub name: String,
    /// 完全修飾名（解決できなければ書かれたままの名前）
    pub type_name: String,
    /// 型名を索引の中で解決できたか（組み込みの型や、ライブラリの型は false）
    pub resolved: bool,
    pub prefix: ConnectPrefix,
    pub causality: Causality,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn class(name: &str, classes: Vec<ClassOutline>) -> ClassOutline {
        ClassOutline {
            name: name.into(),
            kind: ClassKind::Package,
            partial: false,
            causality: Causality::None,
            range: TextRange::default(),
            extends: vec![],
            components: vec![],
            connects: vec![],
            classes,
        }
    }

    #[test]
    fn qualified_names_follow_nesting() {
        let o = Outline {
            within: None,
            classes: vec![class("P", vec![class("A", vec![]), class("B", vec![])])],
        };
        let names: Vec<String> = o.qualified_classes().into_iter().map(|(n, _)| n).collect();
        assert_eq!(names, ["P", "P.A", "P.B"]);
    }

    #[test]
    fn resolve_follows_lexical_scope_from_inside_out() {
        let mut lib = class(
            "Lib",
            vec![
                class("Connectors", vec![class("Pin", vec![])]),
                class("Pin", vec![]),
            ],
        );
        lib.classes[0].classes[0].kind = ClassKind::Connector;
        let index = ClassIndex::from_outlines([&Outline {
            within: None,
            classes: vec![lib],
        }]);
        // Lib.Connectors の中から見た Pin は、外側の Lib.Pin より内側の Lib.Connectors.Pin が先
        assert_eq!(
            index.resolve("Lib.Connectors", "Pin").as_deref(),
            Some("Lib.Connectors.Pin")
        );
        assert_eq!(index.resolve("Lib", "Pin").as_deref(), Some("Lib.Pin"));
        assert_eq!(
            index.resolve("Lib.Connectors", "Connectors.Pin").as_deref(),
            Some("Lib.Connectors.Pin")
        );
        assert_eq!(index.resolve("Lib", "Real"), None);
        assert_eq!(index.resolve("Lib", "Nope"), None);
    }

    #[test]
    fn without_ranges_drops_only_positions() {
        let mut c = class("A", vec![]);
        c.range = TextRange::new(3, 9);
        let o = Outline {
            within: Some("P".into()),
            classes: vec![c],
        };
        let s = o.without_ranges();
        assert_eq!(s.classes[0].range, TextRange::default());
        assert_eq!(s.within.as_deref(), Some("P"));
        assert_ne!(o, s);
        assert_eq!(s, s.without_ranges());
    }

    #[test]
    fn balance() {
        assert!(
            BalanceCount {
                equations: 3,
                unknowns: 3,
                ..Default::default()
            }
            .is_balanced()
        );
        assert!(
            !BalanceCount {
                equations: 2,
                unknowns: 3,
                ..Default::default()
            }
            .is_balanced()
        );
    }
}

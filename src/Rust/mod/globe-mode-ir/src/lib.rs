//! フロントエンドに依存しない中立 IR。
//!
//! Rumoca（Phase A）や自前コア（Track B）の型は、ここに変換してから外に出す。
//! 型はすべて `Eq` を実装する。salsa のクエリの「防火壁」として、
//! 空白やコメントだけの編集では値が変わらないようにするため（ADR-0005）。
//! そのため、ソース上の位置（TextRange）は持たせるが、比較から外したい場合は
//! 呼び出し側で位置を落とした射影を使う（P2 で `outline` クエリを作るときに決める）。

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
    pub range: TextRange,
}

/// `connect(lhs, rhs)`。connect グラフの辺になる。
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
pub struct Connect {
    pub lhs: String,
    pub rhs: String,
    pub range: TextRange,
}

/// 方程式数と変数数（S5 の検査に使う）。
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

#[cfg(test)]
mod tests {
    use super::*;

    fn class(name: &str, classes: Vec<ClassOutline>) -> ClassOutline {
        ClassOutline {
            name: name.into(),
            kind: ClassKind::Package,
            partial: false,
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

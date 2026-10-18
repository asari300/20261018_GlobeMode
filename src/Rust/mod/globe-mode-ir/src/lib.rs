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

    /// このファイルが、完全修飾名 `qualified`（`A.B.C`。先頭の `.` は無視）のクラスを定義しているか。
    /// `within` を前に付けて比べる。入れ子のクラスも含む（`deps_closure` の起点を探すのに使う）。
    pub fn defines(&self, qualified: &str) -> bool {
        let qualified = qualified.trim_start_matches('.');
        let rest = match self.within.as_deref().filter(|w| !w.is_empty()) {
            Some(within) => match qualified
                .strip_prefix(within)
                .and_then(|r| r.strip_prefix('.'))
            {
                Some(rest) => rest,
                None => return false,
            },
            None => qualified,
        };
        self.qualified_classes().iter().any(|(n, _)| n == rest)
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
    /// `import` 文（S006）。`import A.B.C;` は `A.B.C`、`import D = A.B.C;` は `A.B.C`、
    /// `import A.B.*;` は `A.B.*`、`import A.B.{C,D};` は `A.B.C` と `A.B.D`。
    /// `deps_closure`（Rumoca に渡すファイルの絞り込み）が、参照先のクラスを探すのに使う。
    pub imports: Vec<String>,
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

/// DAE の構造（S6 の入力）。方程式と未知数の incidence を、配列をスカラーに展開した粒度で持つ。
///
/// 未知数は Rumoca の構造解析と同じ約束: 状態ごとの `der(x)[k]` と、Algebraic / Output のスカラー。
/// 状態・パラメータ・入力・離散変数・`pre()`・`time` は既知として数えない。
/// 位置（`file` / `range`）は、コンパイルに渡した本文（`source_shape` で正規化した本文）の中の位置。
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize)]
pub struct DaeStructure {
    pub unknowns: Vec<Unknown>,
    pub equations: Vec<EquationRow>,
    /// 方程式ごとに、参照する未知数の添字（昇順、重複なし）。長さは `equations.len()`。
    pub incidence: Vec<Vec<u32>>,
    /// 方程式ごとに、**状態そのもの**（`x`。`der(x)` ではない）を参照しているときの、
    /// その状態の `der(x)` の未知数の添字（昇順、重複なし）。長さは `equations.len()`。
    ///
    /// S6（構造特異）は `x` と `der(x)` を 1 つの列にまとめた二部グラフ（`incidence ∪ state_refs`）で
    /// 完全マッチングを調べる。`incidence` だけ（der の形）で完全マッチングが無いのは、
    /// 微分インデックスが 1 より大きい（S7 の情報）ということで、モデルの誤りではない。
    pub state_refs: Vec<Vec<u32>>,
}

impl DaeStructure {
    /// 位置をすべて落とした写し（防火壁 2、ADR-0005）。
    /// パラメータの値だけを変えた編集などでは前と等しくなり、S6 以降は再計算されない。
    pub fn without_ranges(&self) -> DaeStructure {
        DaeStructure {
            unknowns: self
                .unknowns
                .iter()
                .map(|u| Unknown {
                    range: None,
                    ..u.clone()
                })
                .collect(),
            equations: self
                .equations
                .iter()
                .map(|e| EquationRow {
                    range: None,
                    ..e.clone()
                })
                .collect(),
            incidence: self.incidence.clone(),
            state_refs: self.state_refs.clone(),
        }
    }

    /// `x` と `der(x)` を 1 つの列にまとめた incidence（S6 の判定に使う）。各行は昇順・重複なし。
    pub fn merged_incidence(&self) -> Vec<Vec<u32>> {
        self.incidence
            .iter()
            .enumerate()
            .map(|(i, row)| {
                let mut merged = row.clone();
                if let Some(extra) = self.state_refs.get(i) {
                    merged.extend(extra.iter().copied());
                }
                merged.sort_unstable();
                merged.dedup();
                merged
            })
            .collect()
    }

    /// 方程式数と未知数が同じ。
    pub fn is_square(&self) -> bool {
        self.equations.len() == self.unknowns.len()
    }
}

/// DAE の未知数 1 つ（スカラー）。
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
pub struct Unknown {
    /// 表示名（例: `der(x[2])`、`capacitor.v`）
    pub name: String,
    pub kind: UnknownKind,
    /// 宣言のあるファイル（分かるときだけ）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
    /// 宣言の位置（分かるときだけ）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub range: Option<TextRange>,
}

/// 未知数の種類。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum UnknownKind {
    /// 状態の微分 `der(x)`
    Derivative,
    Algebraic,
    Output,
}

/// DAE の方程式 1 本（スカラー）。
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
pub struct EquationRow {
    /// 表示用の短いラベル（例: ソースの一部、`f[3]`）
    pub label: String,
    pub origin: EquationOrigin,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub range: Option<TextRange>,
}

/// 方程式の由来。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EquationOrigin {
    /// ソースに書かれた方程式
    Source,
    /// connect 文から作った（potential の等式）
    Connection,
    /// connect 文から作った（flow の総和 = 0）
    FlowBalance,
    /// 宣言の束縛（`Real y = 2 * x`）
    Binding,
    /// そのほかコンパイラが生成したもの
    Generated,
}

/// flat モデルの要約（`flat_summary` クエリ）。S5 で数が合わないときに、どの部品が原因かを示すのに使う。
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize)]
pub struct FlatSummary {
    /// 変数の宣言の数（配列は 1 個）
    pub variables: usize,
    /// 方程式の数（スカラー）
    pub equations: usize,
    /// 初期方程式の数（スカラー）
    pub initial_equations: usize,
    pub partial: bool,
    /// トップレベルのコネクタ（FMU の入出力になる）
    pub top_level_connectors: Vec<String>,
    /// 部品ごとの方程式の数（スカラー）。部品のパス順
    pub equations_by_component: Vec<(String, usize)>,
}

/// 初期化系の構造（S8 の入力、S006）。`check` が DAE から取り出す。
///
/// 位置（`file` / `range`）は、コンパイルに渡した本文（`source_shape` で正規化した本文）の中の位置。
/// S8 の判定は数と属性だけで行う（初期方程式の incidence は持たない。S007 以降の候補）。
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize)]
pub struct InitStructure {
    /// 状態（スカラー）。順は [`DaeStructure::unknowns`] の `Derivative` の順と同じ（`der(x)` の `x`）。
    pub states: Vec<InitVariable>,
    /// パラメータ（スカラー）。`fixed = false` のものは初期化の未知数になり、束縛の無いものは警告の対象。
    pub parameters: Vec<InitVariable>,
    /// 離散変数（スカラー。DiscreteReal / DiscreteValue）。`pre(d)` が初期化の未知数になり（MLS §8.6）、
    /// `fixed = true` なら `pre(d) = start` が初期方程式に加わる（初期条件 1 つ）。
    pub discretes: Vec<InitVariable>,
    /// 代数変数（スカラー。Algebraic / Output）。`fixed = true` なら `v = start` が初期方程式に加わる
    /// （初期条件 1 つ）。
    pub algebraics: Vec<InitVariable>,
    /// 初期方程式（スカラー。`initial equation` 節。構造化された族は点ごとに 1 行）。フロントエンドが
    /// 初期方程式や初期アルゴリズムから別に取り分けた定義（離散変数や Real でないパラメータの値）も含む。
    pub initial_equations: Vec<EquationRow>,
}

impl InitStructure {
    /// 位置をすべて落とした写し（`init_shape` クエリの防火壁）。
    pub fn without_ranges(&self) -> InitStructure {
        let strip = |v: &InitVariable| InitVariable {
            range: None,
            ..v.clone()
        };
        InitStructure {
            states: self.states.iter().map(strip).collect(),
            parameters: self.parameters.iter().map(strip).collect(),
            discretes: self.discretes.iter().map(strip).collect(),
            algebraics: self.algebraics.iter().map(strip).collect(),
            initial_equations: self
                .initial_equations
                .iter()
                .map(|e| EquationRow {
                    range: None,
                    ..e.clone()
                })
                .collect(),
        }
    }
}

/// 初期化に関わる変数 1 つ（スカラー）。
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
pub struct InitVariable {
    /// 表示名（例: `capacitor.v`、`x[2]`）
    pub name: String,
    /// `fixed` 属性。宣言が無ければ MLS 4.8 の既定（状態・離散・代数は false、パラメータは true）
    pub fixed: bool,
    /// `start` 属性が明示されている（修飾子か、型の宣言から受け継いだもの）
    pub has_start: bool,
    /// 束縛（`= 式`）がある。パラメータだけ意味を持つ（状態・離散・代数は常に false）
    pub has_binding: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub range: Option<TextRange>,
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
            imports: vec![],
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
    fn defines_matches_qualified_names_after_within() {
        let o = Outline {
            within: Some("Lib.Sub".into()),
            classes: vec![class("P", vec![class("A", vec![])])],
        };
        assert!(o.defines("Lib.Sub.P"));
        assert!(o.defines("Lib.Sub.P.A"));
        assert!(o.defines(".Lib.Sub.P.A"));
        assert!(!o.defines("P.A"), "within を付けない名前は別のクラス");
        assert!(
            !o.defines("Lib.Sub"),
            "within のパッケージそのものは、このファイルの定義ではない"
        );
        assert!(!o.defines("Lib.SubP"));
        assert!(!o.defines("Lib.Sub.P.B"));
        // `within;`（空）と within 無しは、トップレベルから
        for within in [None, Some(String::new())] {
            let top = Outline {
                within,
                classes: vec![class("Q", vec![])],
            };
            assert!(top.defines("Q"));
            assert!(!top.defines("Lib.Q"));
        }
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
    fn init_without_ranges_drops_positions_of_every_list() {
        let v = |name: &str| InitVariable {
            name: name.into(),
            fixed: true,
            has_start: true,
            has_binding: false,
            file: Some("M.mo".into()),
            range: Some(TextRange::new(1, 4)),
        };
        let init = InitStructure {
            states: vec![v("x")],
            parameters: vec![v("p")],
            discretes: vec![v("d")],
            algebraics: vec![v("y")],
            initial_equations: vec![EquationRow {
                label: "x = 1".into(),
                origin: EquationOrigin::Source,
                file: Some("M.mo".into()),
                range: Some(TextRange::new(5, 10)),
            }],
        };
        let s = init.without_ranges();
        let all = s
            .states
            .iter()
            .chain(&s.parameters)
            .chain(&s.discretes)
            .chain(&s.algebraics);
        assert_eq!(all.clone().count(), 4);
        for v in all {
            assert_eq!((v.range, v.file.as_deref()), (None, Some("M.mo")));
        }
        assert_eq!(s.initial_equations[0].range, None);
        assert_eq!(s.algebraics[0].name, "y");
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

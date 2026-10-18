//! connect グラフ。flatten せずに、中立 IR のクラスの索引（[`ClassIndex`]）から作る。
//!
//! - ノード: モデルの部品（コンポーネント）。connect に出てくる自分のコネクタもノードにする。
//! - 辺: connect 文。コネクタのクラスから、辺の種類（[`Domain`]）を決める。
//! - `depth`: 1 なら直下の部品だけ。2 以上なら、中に connect を持つ部品（部分回路）を展開し、
//!   その部品のコネクタを「つなぎ目」のノード（`stage1.p` など）にして、内と外の辺をつなぐ。
//!
//! 出力は graphology の JSON（sigma.js がそのまま読める形）。

use std::collections::{BTreeMap, BTreeSet};

use globe_mode_ir::{ClassIndex, ClassKind, ConnectPrefix};
use serde::Serialize;

/// 辺の種類。コネクタのクラスから決める。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Domain {
    /// potential と flow を持つ物理のコネクタ（例: 電気のピン。Real の v と flow Real の i）
    Flow,
    /// stream を持つコネクタ（例: Medium を持つ流体ポート）
    Stream,
    /// Real の信号（`connector RealInput = input Real` など）
    Real,
    /// Integer の信号
    Integer,
    /// Boolean の信号
    Boolean,
    /// String の信号
    String,
    /// potential だけのコネクタ、または種類を決められないもの
    Other,
    /// コネクタのクラスが索引に無い（ライブラリの型など）
    Unresolved,
}

/// ノードの種類。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeKind {
    /// 部品（モデルやブロックのインスタンス）
    Component,
    /// コネクタ（モデル自身のコネクタ、または展開した部品のつなぎ目）
    Connector,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Node {
    /// `stage1.resistor` のようなパス。グラフの中で一意
    pub key: String,
    pub label: String,
    /// 型（完全修飾名。解決できなければ書かれたまま）
    pub class: String,
    pub kind: NodeKind,
    /// 階層の深さ（直下の部品は 1）
    pub level: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Edge {
    pub key: String,
    pub source: String,
    pub target: String,
    /// connect の左辺と右辺（モデルから見たパス）
    pub lhs: String,
    pub rhs: String,
    pub domain: Domain,
    /// 左辺のコネクタのクラス（完全修飾名）
    pub connector: String,
}

/// connect グラフ。
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize)]
pub struct ConnectGraph {
    pub model: String,
    pub depth: u32,
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
    /// 索引で解決できなかった型（ライブラリの型など）
    pub unresolved: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GraphError {
    ModelNotFound(String),
}

impl std::fmt::Display for GraphError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GraphError::ModelNotFound(m) => write!(f, "モデル `{m}` が索引に無い"),
        }
    }
}

impl ConnectGraph {
    /// 種類ごとの辺の数。
    pub fn domain_counts(&self) -> BTreeMap<Domain, usize> {
        let mut m = BTreeMap::new();
        for e in &self.edges {
            *m.entry(e.domain).or_insert(0) += 1;
        }
        m
    }

    /// graphology の JSON（`graph.import(json)` で読める形）。
    pub fn to_graphology(&self) -> serde_json::Value {
        serde_json::json!({
            "attributes": { "model": self.model, "depth": self.depth, "unresolved": self.unresolved },
            "options": { "type": "undirected", "multi": true, "allowSelfLoops": false },
            "nodes": self.nodes.iter().map(|n| serde_json::json!({
                "key": n.key,
                "attributes": { "label": n.label, "class": n.class, "kind": n.kind, "level": n.level },
            })).collect::<Vec<_>>(),
            "edges": self.edges.iter().map(|e| serde_json::json!({
                "key": e.key,
                "source": e.source,
                "target": e.target,
                "attributes": { "lhs": e.lhs, "rhs": e.rhs, "domain": e.domain, "connector": e.connector },
            })).collect::<Vec<_>>(),
        })
    }
}

/// コネクタのクラスから辺の種類を決める。
pub fn connector_domain(index: &ClassIndex, connector: &str) -> Domain {
    if index.get(connector).is_none() {
        return Domain::Unresolved;
    }
    if let Some(base) = index.builtin_base(connector) {
        return match base {
            "Real" => Domain::Real,
            "Integer" => Domain::Integer,
            "Boolean" => Domain::Boolean,
            "String" => Domain::String,
            _ => Domain::Other,
        };
    }
    let comps = index.interface(connector);
    if comps.iter().any(|c| c.prefix == ConnectPrefix::Stream) {
        Domain::Stream
    } else if comps.iter().any(|c| c.prefix == ConnectPrefix::Flow) {
        Domain::Flow
    } else {
        Domain::Other
    }
}

/// `model` の connect グラフを作る。
pub fn build(index: &ClassIndex, model: &str, depth: u32) -> Result<ConnectGraph, GraphError> {
    if index.get(model).is_none() {
        return Err(GraphError::ModelNotFound(model.to_string()));
    }
    let mut b = Builder {
        index,
        nodes: BTreeMap::new(),
        edges: Vec::new(),
        unresolved: BTreeSet::new(),
    };
    b.walk(model, "", 1, depth.max(1));
    let mut nodes: Vec<Node> = b.nodes.into_values().collect();
    nodes.sort_by(|a, b| (a.level, &a.key).cmp(&(b.level, &b.key)));
    Ok(ConnectGraph {
        model: model.to_string(),
        depth: depth.max(1),
        nodes,
        edges: b.edges,
        unresolved: b.unresolved.into_iter().collect(),
    })
}

struct Builder<'a> {
    index: &'a ClassIndex,
    nodes: BTreeMap<String, Node>,
    edges: Vec<Edge>,
    unresolved: BTreeSet<String>,
}

impl Builder<'_> {
    /// クラスの種類が「部品」（ノードにするもの）か。
    fn is_part(&self, class: &str) -> bool {
        self.index.get(class).is_some_and(|c| {
            matches!(
                c.kind,
                ClassKind::Model | ClassKind::Block | ClassKind::Class
            )
        })
    }

    /// 中に connect を持つ部品（展開できる部分回路）か。
    fn is_composite(&self, class: &str) -> bool {
        !self.index.connects(class).is_empty()
    }

    fn join(prefix: &str, name: &str) -> String {
        if prefix.is_empty() {
            name.to_string()
        } else {
            format!("{prefix}.{name}")
        }
    }

    /// `class`（`prefix` の位置にあるインスタンス）の中身を、`level` の深さでグラフに足す。
    fn walk(&mut self, class: &str, prefix: &str, level: u32, max_depth: u32) {
        let interface = self.index.interface(class);
        let mut expanded = BTreeSet::new();

        for comp in &interface {
            if !comp.resolved {
                if !globe_mode_ir::BUILTIN_TYPES.contains(&comp.type_name.as_str()) {
                    self.unresolved.insert(comp.type_name.clone());
                }
                continue;
            }
            if !self.is_part(&comp.type_name) {
                continue;
            }
            let key = Self::join(prefix, &comp.name);
            if level < max_depth && self.is_composite(&comp.type_name) {
                expanded.insert(comp.name.clone());
                self.walk(&comp.type_name, &key, level + 1, max_depth);
            } else {
                self.nodes.insert(
                    key.clone(),
                    Node {
                        key,
                        label: comp.name.clone(),
                        class: comp.type_name.clone(),
                        kind: NodeKind::Component,
                        level,
                    },
                );
            }
        }

        for (i, k) in self.index.connects(class).iter().enumerate() {
            let (src, src_class) = self.endpoint(class, prefix, &k.lhs, &expanded, level);
            let (dst, _) = self.endpoint(class, prefix, &k.rhs, &expanded, level);
            let domain = src_class
                .as_deref()
                .map_or(Domain::Unresolved, |c| connector_domain(self.index, c));
            self.edges.push(Edge {
                key: format!("{}#{i}", if prefix.is_empty() { "_" } else { prefix }),
                source: src,
                target: dst,
                lhs: Self::join(prefix, &k.lhs),
                rhs: Self::join(prefix, &k.rhs),
                domain,
                connector: src_class.unwrap_or_default(),
            });
        }
    }

    /// connect の片側（`c.p` や `p`）を、ノードのキーとコネクタのクラスにする。
    fn endpoint(
        &mut self,
        class: &str,
        prefix: &str,
        reference: &str,
        expanded: &BTreeSet<String>,
        level: u32,
    ) -> (String, Option<String>) {
        let interface = self.index.interface(class);
        let mut parts = reference.split('.');
        let head = parts
            .next()
            .unwrap_or(reference)
            .split('[')
            .next()
            .unwrap_or(reference);
        let rest: Vec<&str> = parts.collect();
        let head_type = interface
            .iter()
            .find(|c| c.name == head)
            .map(|c| c.type_name.clone());

        if rest.is_empty() {
            // 自分のコネクタ（`p`）。展開した部品の中なら、外側とのつなぎ目のノードになる
            let key = Self::join(prefix, head);
            let connector = head_type.clone();
            self.nodes.entry(key.clone()).or_insert_with(|| Node {
                key: key.clone(),
                label: head.to_string(),
                class: connector.clone().unwrap_or_default(),
                kind: NodeKind::Connector,
                level: level.saturating_sub(1).max(1),
            });
            return (key, connector);
        }

        // 部品のコネクタ（`c.p`）。コネクタのクラスは、部品のクラスの中で解決する
        let connector = head_type
            .as_deref()
            .and_then(|t| {
                self.index
                    .interface(t)
                    .into_iter()
                    .find(|c| c.name == rest[0])
            })
            .map(|c| c.type_name);
        if expanded.contains(head) {
            // 展開した部品のコネクタは、つなぎ目のノード（`stage1.p`）
            let key = Self::join(prefix, &format!("{head}.{}", rest[0]));
            self.nodes.entry(key.clone()).or_insert_with(|| Node {
                key: key.clone(),
                label: format!("{head}.{}", rest[0]),
                class: connector.clone().unwrap_or_default(),
                kind: NodeKind::Connector,
                level: level + 1,
            });
            return (key, connector);
        }
        (Self::join(prefix, head), connector)
    }
}

//! 過剰決定コネクタの接続（`Connections.root`・`branch`・`potentialRoot`・`isRoot`。MLS 9.4。S312）。
//!
//! 平らにし終わったあとで、仮想接続グラフ（VCG）の全域森を選び、Rumoca の flat と同じ数にそろえる。
//! - 節点は、`equalityConstraint` を持つレコード（`Reference`）の部品と、`Connections.*` の引数。
//!   必須の辺は `branch`、任意の辺は `connect` が節点どうしをつなぐもの。
//! - 必須の辺を先に、次に任意の辺を、閉路を作らず、確定した根（`root`）を持つ木どうしをつながないように選ぶ。
//!   選ばれなかった任意の辺をまたぐ等式は `equalityConstraint` に置き換わる。これは Rumoca の flat と同じく、
//!   戻り値の大きさを 0 と見て等式を作らない（shortcut: 大きさが 0 でない `equalityConstraint`（MultiBody の `Orientation`）は
//!   平らにできないので考えない）。
//! - `if Connections.isRoot(N) then … end if`（`else` なし）は、`N` が根のときだけ残す。
//!   根は、木に確定した根があればそれ、無ければ潜在的な根（優先度が小さく、同じなら名前が先のもの）、それも無ければ名前が先のもの。
//!
//! 全域森の選び方（辺の順）は Rumoca と違ってよい。接続集合ごとの等式の数の合計は選び方によらない。

use super::F;
use crate::Equation;
use std::collections::{BTreeSet, HashMap, HashSet};

/// 過剰決定コネクタの接続集合 1 つ。`start` は `F::eqs` の中の、この集合の等式の先頭（`members.len() - 1` 本）
pub(super) struct OcSet {
    pub start: usize,
    pub members: Vec<String>,
}

/// `Connections.branch(a, b)` の引数
fn args(text: &str, f: &str) -> Option<Vec<String>> {
    let inner = text.strip_prefix(f)?.strip_prefix('(')?.strip_suffix(')')?;
    let (mut depth, mut cur, mut out) = (0, String::new(), Vec::new());
    for ch in inner.chars() {
        match ch {
            '[' | '(' => depth += 1,
            ']' | ')' => depth -= 1,
            _ => {}
        }
        if ch == ',' && depth == 0 {
            out.push(cur.trim().to_string());
            cur.clear();
        } else {
            cur.push(ch);
        }
    }
    out.push(cur.trim().to_string());
    Some(out)
}

struct Uf {
    parent: Vec<usize>,
    root: Vec<bool>,
}

impl Uf {
    fn find(&mut self, mut x: usize) -> usize {
        while self.parent[x] != x {
            self.parent[x] = self.parent[self.parent[x]];
            x = self.parent[x];
        }
        x
    }
    /// 閉路になる、または確定した根を持つ木どうしなら、つながずに false
    fn join(&mut self, a: usize, b: usize) -> bool {
        let (a, b) = (self.find(a), self.find(b));
        if a == b || self.root[a] && self.root[b] {
            return false;
        }
        self.parent[b] = a;
        self.root[a] |= self.root[b];
        true
    }
}

impl F<'_> {
    pub(super) fn apply_connection_graph(&mut self) {
        let mut definite: HashSet<String> = HashSet::new();
        let mut potential: Vec<(String, i64)> = Vec::new();
        let mut branches: Vec<(String, String)> = Vec::new();
        let mut root_ifs: Vec<(usize, String)> = Vec::new();
        for (i, e) in self.eqs.iter().enumerate() {
            let t = e.text.trim_end_matches(';').trim();
            if let Some(a) = args(t, "Connections.root") {
                definite.insert(a[0].clone());
            } else if let Some(a) = args(t, "Connections.branch").filter(|a| a.len() == 2) {
                branches.push((a[0].clone(), a[1].clone()));
            } else if let Some(a) = args(t, "Connections.potentialRoot") {
                let prio = a.get(1).and_then(|p| p.parse().ok()).unwrap_or(0);
                potential.push((a[0].clone(), prio));
            } else if let Some(r) = t.strip_prefix("if Connections.isRoot(")
                && let Some((node, body)) = r.split_once(") then ")
                && body.ends_with(" end if")
                && !body.contains(" else")
            {
                root_ifs.push((i, node.trim().to_string()));
            }
        }
        let mut all: BTreeSet<String> = self.oc_nodes.clone();
        all.extend(definite.iter().cloned());
        all.extend(potential.iter().map(|(n, _)| n.clone()));
        all.extend(branches.iter().flat_map(|(a, b)| [a.clone(), b.clone()]));
        if all.is_empty() {
            return;
        }
        let nodes: Vec<&String> = all.iter().collect();
        let idx: HashMap<&str, usize> = nodes
            .iter()
            .enumerate()
            .map(|(i, n)| (n.as_str(), i))
            .collect();
        let mut tree = Uf {
            parent: (0..nodes.len()).collect(),
            root: nodes.iter().map(|n| definite.contains(*n)).collect(),
        };
        for (a, b) in &branches {
            tree.join(idx[a.as_str()], idx[b.as_str()]);
        }
        // 任意の辺: `connect(a, b)` の両側の、同じ接尾辞の節点
        let mut seen = HashSet::new();
        let mut sel = Uf {
            parent: (0..nodes.len()).collect(),
            root: vec![false; nodes.len()],
        };
        for (a, b) in &self.conn_pairs {
            let pre = format!("{a}.");
            let mut pairs: Vec<(usize, usize)> = Vec::new();
            if let (Some(&x), Some(&y)) = (idx.get(a.as_str()), idx.get(b.as_str())) {
                pairs.push((x, y));
            }
            for na in all.range(pre.clone()..).take_while(|n| n.starts_with(&pre)) {
                if let Some(&y) = idx.get(format!("{b}{}", &na[a.len()..]).as_str()) {
                    pairs.push((idx[na.as_str()], y));
                }
            }
            for (x, y) in pairs {
                if seen.insert((x.min(y), x.max(y))) && tree.join(x, y) {
                    sel.join(x, y);
                }
            }
        }
        // 接続集合の等式を、選ばれた辺でつながる群ごとの連鎖にする
        let node_of = |leaf: &str| {
            let mut s = leaf;
            while let Some(i) = s.rfind('.') {
                s = &s[..i];
                if let Some(&k) = idx.get(s) {
                    return Some(k);
                }
            }
            None
        };
        let mut replace: HashMap<usize, (usize, Vec<Equation>)> = HashMap::new();
        for set in &self.oc_sets {
            let keys: Vec<Option<usize>> = set.members.iter().map(|m| node_of(m)).collect();
            if keys.iter().all(Option::is_none) {
                continue;
            }
            let (count, origin) = (
                self.eqs[set.start].count,
                self.eqs[set.start].origin.clone(),
            );
            let mut first: HashMap<usize, usize> = HashMap::new();
            let mut eqs = Vec::new();
            for (i, k) in keys.iter().enumerate() {
                // 節点でない端は、自分だけの群
                let g = k.map_or(usize::MAX - i, |k| sel.find(k));
                match first.get(&g) {
                    Some(&j) => eqs.push(Equation {
                        text: format!("{} = {}", set.members[j], set.members[i]),
                        count,
                        origin: origin.clone(),
                    }),
                    None => {
                        first.insert(g, i);
                    }
                }
            }
            replace.insert(set.start, (set.members.len() - 1, eqs));
        }
        // 根の判定: 木の根（確定した根、無ければ潜在的な根のうち優先度が小さく名前が先のもの、無ければ名前が先のもの）
        let mut drop: HashSet<usize> = HashSet::new();
        for (i, node) in &root_ifs {
            let Some(&k) = idx.get(node.as_str()) else {
                continue;
            };
            let c = tree.find(k);
            let members: Vec<usize> = (0..nodes.len()).filter(|&j| tree.find(j) == c).collect();
            let is_root = definite.contains(node.as_str())
                || (!tree.root[c] && {
                    let best = potential
                        .iter()
                        .filter(|(n, _)| members.iter().any(|&j| nodes[j] == n))
                        .min_by(|a, b| (a.1, &a.0).cmp(&(b.1, &b.0)))
                        .map(|(n, _)| n.as_str());
                    best.or_else(|| members.iter().map(|&j| nodes[j].as_str()).min())
                        == Some(node.as_str())
                });
            if !is_root {
                drop.insert(*i);
            }
        }
        if replace.is_empty() && drop.is_empty() {
            return;
        }
        let old = std::mem::take(&mut self.eqs);
        let mut it = old.into_iter().enumerate();
        while let Some((i, e)) = it.next() {
            if let Some((n, eqs)) = replace.remove(&i) {
                self.eqs.extend(eqs);
                for _ in 1..n {
                    it.next();
                }
            } else if !drop.contains(&i) {
                self.eqs.push(e);
            }
        }
    }
}

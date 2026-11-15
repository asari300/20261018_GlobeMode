//! 過剰決定コネクタの接続（`Connections.root`・`branch`・`potentialRoot`・`isRoot`。MLS 9.4。S312）。
//!
//! 平らにし終わったあとで、仮想接続グラフ（VCG）の全域森を選び、Rumoca の flat と同じ数にそろえる。
//! - 節点は、`equalityConstraint` を持つレコード（`Reference`）の部品と、`Connections.*` の引数。
//!   必須の辺は `branch`、任意の辺は `connect` が節点どうしをつなぐもの。
//! - 必須の辺を先に、次に任意の辺を、閉路を作らず、確定した根（`root`）を持つ木どうしをつながないように選ぶ。
//!   選ばれなかった任意の辺をまたぐ等式は `equalityConstraint` に置き換わる。これは Rumoca の flat と同じく、
//!   戻り値の大きさを 0 と見て等式を作らない（shortcut: 大きさが 0 でない `equalityConstraint`（MultiBody の `Orientation`）は
//!   平らにできないので考えない）。
//! - `if` の方程式の条件に `Connections.isRoot(N)`・`Connections.rooted(N)` があれば、`flatten.rs` が枝ごとに平らにして `OcIf` に置く（S333）。
//!   ここで根の決定のあとに、最初に条件が真の枝（`else` なら常に真）だけを残す。
//!   根は、木に確定した根があればそれ、無ければ潜在的な根（優先度が小さく、同じなら名前が先のもの）、それも無ければ名前が先のもの。
//!   `rooted(a)` は `branch(a, b)` の `a` が根に近い（木の上の深さが小さい）こと。選ばれる木は、根からの距離が近い辺を先に採る（Rumoca と同じ。MLS 9.4 は木の選び方を決めない）。
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

/// 条件に `Connections.isRoot`・`rooted` を含む `if` の方程式。枝（条件、平らにした方程式）の並びで、条件が `None` なら `else`。
/// 条件は `(rooted か、not か、節点)`。置き場所は `F::eqs`（`initial` なら `F::ieqs`）の `at` の 1 行
pub(super) type RootCond = (bool, bool, String);

pub(super) struct OcIf {
    pub initial: bool,
    pub at: usize,
    pub branches: Vec<(Option<RootCond>, Vec<Equation>)>,
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

/// `f(x)` の引数 1 つ（`Connections.isRoot(a.R)` の `a.R`）
pub(super) fn arg(text: &str, f: &str) -> Option<String> {
    args(text, f)
        .filter(|a| a.len() == 1)
        .map(|mut a| a.remove(0))
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
        for e in &self.eqs {
            let t = e.text.trim_end_matches(';').trim();
            if let Some(a) = args(t, "Connections.root") {
                definite.insert(a[0].clone());
            } else if let Some(a) = args(t, "Connections.branch").filter(|a| a.len() == 2) {
                branches.push((a[0].clone(), a[1].clone()));
            } else if let Some(a) = args(t, "Connections.potentialRoot") {
                let prio = a.get(1).and_then(|p| p.parse().ok()).unwrap_or(0);
                potential.push((a[0].clone(), prio));
            }
        }
        let mut all: BTreeSet<String> = self.oc_nodes.clone();
        all.extend(definite.iter().cloned());
        all.extend(potential.iter().map(|(n, _)| n.clone()));
        all.extend(branches.iter().flat_map(|(a, b)| [a.clone(), b.clone()]));
        if all.is_empty() && self.oc_ifs.is_empty() {
            return;
        }
        // 節点の番号は名前順（辺の正規化と同じ）
        let nodes: Vec<&String> = all.iter().collect();
        let idx: HashMap<&str, usize> = nodes
            .iter()
            .enumerate()
            .map(|(i, n)| (n.as_str(), i))
            .collect();
        // 成分の根: 確定した根、無ければ潜在的な根（優先度が小さく、同じなら名前が先）、無ければ名前が先
        let root_of = |members: &[usize]| -> usize {
            let is = |j: &usize| definite.contains(nodes[*j].as_str());
            members.iter().copied().find(is).unwrap_or_else(|| {
                potential
                    .iter()
                    .filter_map(|(n, p)| {
                        Some((
                            *p,
                            idx.get(n.as_str())
                                .copied()
                                .filter(|j| members.contains(j))?,
                        ))
                    })
                    .min()
                    .map_or_else(|| members.iter().copied().min().unwrap_or(0), |(_, j)| j)
            })
        };
        let mut tree = Uf {
            parent: (0..nodes.len()).collect(),
            root: nodes.iter().map(|n| definite.contains(*n)).collect(),
        };
        let mut edges: Vec<(usize, usize)> = Vec::new();
        for (a, b) in &branches {
            let (x, y) = (idx[a.as_str()], idx[b.as_str()]);
            if tree.join(x, y) {
                edges.push((x, y));
            }
        }
        // 任意の辺: `connect(a, b)` の両側の、同じ接尾辞の節点
        let mut seen = HashSet::new();
        let mut optional: Vec<(usize, usize)> = Vec::new();
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
                if seen.insert((x.min(y), x.max(y))) {
                    optional.push((x, y));
                }
            }
        }
        // 任意の辺を採る順: 全辺の上で、成分の根（確定した根、無ければ選んだ根）からの距離が近いもの、次に名前（Rumoca と同じ）
        let mut adj_all: Vec<Vec<usize>> = vec![Vec::new(); nodes.len()];
        for &(x, y) in branches
            .iter()
            .map(|(a, b)| (idx[a.as_str()], idx[b.as_str()]))
            .collect::<Vec<_>>()
            .iter()
            .chain(&optional)
        {
            adj_all[x].push(y);
            adj_all[y].push(x);
        }
        let dist = {
            let mut full = Uf {
                parent: (0..nodes.len()).collect(),
                root: vec![false; nodes.len()],
            };
            for (x, l) in adj_all.iter().enumerate() {
                for &y in l {
                    full.join(x, y);
                }
            }
            let mut comps: HashMap<usize, Vec<usize>> = HashMap::new();
            for k in 0..nodes.len() {
                comps.entry(full.find(k)).or_default().push(k);
            }
            let mut seeds: Vec<usize> = Vec::new();
            for m in comps.values() {
                let d: Vec<usize> = m
                    .iter()
                    .copied()
                    .filter(|&j| definite.contains(nodes[j].as_str()))
                    .collect();
                seeds.extend(if d.is_empty() { vec![root_of(m)] } else { d });
            }
            bfs(&adj_all, &seeds)
        };
        optional.sort_by_key(|&(x, y)| {
            let (dx, dy) = (dist[x], dist[y]);
            (dx.min(dy), dx.max(dy), x.min(y), x.max(y))
        });
        let mut sel = Uf {
            parent: (0..nodes.len()).collect(),
            root: vec![false; nodes.len()],
        };
        for (x, y) in optional {
            if tree.join(x, y) {
                sel.join(x, y);
                edges.push((x, y));
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
        // 根の判定。選ばれた木（必須の辺と選んだ任意の辺）の成分ごとに根を決め、根からの深さを数える
        let mut adj: Vec<Vec<usize>> = vec![Vec::new(); nodes.len()];
        for &(x, y) in &edges {
            adj[x].push(y);
            adj[y].push(x);
        }
        let comp_of: Vec<usize> = (0..nodes.len()).map(|k| tree.find(k)).collect();
        let mut comps: HashMap<usize, Vec<usize>> = HashMap::new();
        for (k, &c) in comp_of.iter().enumerate() {
            comps.entry(c).or_default().push(k);
        }
        let roots: Vec<usize> = comps.values().map(|m| root_of(m)).collect();
        let depth = bfs(&adj, &roots);
        // 節点が無い（過剰決定の接続の外の）ときの Rumoca の既定は、`isRoot` が真、`rooted` が偽
        let is_root = |n: &str| {
            idx.get(n).is_none_or(|&k| {
                let c = comp_of[k];
                definite.contains(n) || (!tree.root[c] && root_of(&comps[&c]) == k)
            })
        };
        // `rooted(a)` は、`branch(a, b)` の `a` が `b` より根に近いこと（同じ `a` の枝が複数なら最後のもの）
        let rooted = |n: &str| {
            branches
                .iter()
                .rfind(|(a, _)| a == n)
                .is_some_and(|(a, b)| depth[idx[a.as_str()]] < depth[idx[b.as_str()]])
        };
        for f in std::mem::take(&mut self.oc_ifs) {
            let eqs = f
                .branches
                .into_iter()
                .find(|(c, _)| {
                    c.as_ref()
                        .is_none_or(|(r, not, n)| (if *r { rooted(n) } else { is_root(n) }) != *not)
                })
                .map(|(_, e)| e)
                .unwrap_or_default();
            if f.initial {
                self.ieqs.splice(f.at..=f.at, eqs);
            } else {
                replace.insert(f.at, (1, eqs));
            }
        }
        if replace.is_empty() {
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
            } else {
                self.eqs.push(e);
            }
        }
    }
}

/// 複数の始点からの幅優先の距離（届かない節点は `usize::MAX`）
fn bfs(adj: &[Vec<usize>], seeds: &[usize]) -> Vec<usize> {
    let mut d = vec![usize::MAX; adj.len()];
    let mut q: std::collections::VecDeque<usize> = seeds.iter().copied().collect();
    for &s in seeds {
        d[s] = 0;
    }
    while let Some(x) = q.pop_front() {
        for &y in &adj[x] {
            if d[y] == usize::MAX {
                d[y] = d[x] + 1;
                q.push_back(y);
            }
        }
    }
    d
}

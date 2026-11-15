//! 計測（`#[ignore]`。`cargo test -p globe-mode-dae --test compare -- --ignored --nocapture`）。
//! 自前の DAE（globe-mode-dae）の釣り合い・構造の正則を、Rumoca の判定（S5 の `balance`、S6 の `structure`）と突き合わせる。
//!
//! - `GLOBE_MSL`: MSL のディレクトリ（既定は `data/msl`）。`GLOBE_DAE_LIMIT`: MSL の本数を絞る。

use globe_mode_dae::build;
use globe_mode_flat::Project;
use globe_mode_frontend::Checker;
use globe_mode_syntax as _;
use globe_mode_syntax::SyntaxKind;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../..")
}

fn msl_dir() -> PathBuf {
    std::env::var_os("GLOBE_MSL").map_or_else(|| root().join("data/msl"), PathBuf::from)
}

fn is_model_class(p: &Project, cid: usize) -> bool {
    matches!(
        p.kind_of(cid),
        Some(SyntaxKind::KwModel | SyntaxKind::KwBlock | SyntaxKind::KwClass)
    )
}

/// ハーネスの規則（xtask の `examples_in`）と同じ: `Modelica.….Examples.…` の model / block / class
fn msl_examples(p: &Project) -> Vec<String> {
    const SUPPORT: [&str; 4] = ["Utilities", "BaseClasses", "Internal", "Interfaces"];
    let mut out = Vec::new();
    for cid in 0..p.lib.classes.len() {
        let name = p.lib.qualified_name(cid);
        let seg: Vec<&str> = name.split('.').collect();
        if seg[0] != "Modelica" || !is_model_class(p, cid) || name.contains("Connections.") {
            continue;
        }
        let Some(e) = seg.iter().position(|s| *s == "Examples") else {
            continue;
        };
        let last = seg.len() - 1;
        if e + 1 > last || seg[e + 1..last].iter().any(|s| SUPPORT.contains(s)) {
            continue;
        }
        let pkgs = (e + 1..last).all(|i| {
            p.lib
                .class_by_name(&seg[..=i].join("."))
                .is_some_and(|c| p.kind_of(c) == Some(SyntaxKind::KwPackage))
        });
        if pkgs {
            out.push(name);
        }
    }
    out.sort();
    out
}

struct Theirs {
    eq: usize,
    unk: usize,
    states: usize,
    discrete: usize,
    regular: Option<bool>,
}

fn rumoca(checker: &Checker, model: &str) -> Option<Theirs> {
    let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| checker.check(&[], model)))
        .ok()?;
    let b = r.balance?;
    Some(Theirs {
        eq: b.equations,
        unk: b.unknowns,
        states: b.states,
        discrete: b.discrete,
        regular: r
            .structure
            .as_ref()
            .map(|s| globe_mode_analysis::analyze(s).is_regular()),
    })
}

#[derive(Default)]
struct Tally {
    both: usize,
    mine_balanced: usize,
    theirs_balanced: usize,
    balanced_same: usize,
    counts_same: usize,
    states_same: usize,
    mine_regular: usize,
    theirs_regular: usize,
    regular_same: usize,
    regular_compared: usize,
    diffs: Vec<String>,
    other: Vec<String>,
}

impl Tally {
    fn add(&mut self, model: &str, m: &globe_mode_dae::Dae, t: &Theirs) {
        self.both += 1;
        let (mb, tb) = (m.is_balanced(), t.eq == t.unk);
        self.mine_balanced += usize::from(mb);
        self.theirs_balanced += usize::from(tb);
        self.balanced_same += usize::from(mb == tb);
        let same = m.counts.equations == t.eq && m.counts.unknowns == t.unk;
        self.counts_same += usize::from(same);
        self.states_same += usize::from(m.counts.states == t.states);
        if m.counts.states != t.states && self.other.len() < 40 {
            self.other.push(format!(
                "状態の数 {model}: {} vs {} (rumoca)",
                m.counts.states, t.states
            ));
        }
        let mr = m.is_regular();
        self.mine_regular += usize::from(mr);
        if let Some(tr) = t.regular {
            self.theirs_regular += usize::from(tr);
            self.regular_compared += 1;
            self.regular_same += usize::from(mr == tr);
            if mr != tr && self.other.len() < 80 {
                self.other
                    .push(format!("正則 {model}: 自前 {mr} / rumoca {tr}"));
            }
        }
        if !same && self.diffs.len() < 400 {
            self.diffs.push(format!(
                "{model}: eq {} vs {}, unk {} vs {} (rumoca); states {} vs {}, discrete {} vs {}, discrete_eq {}",
                m.counts.equations,
                t.eq,
                m.counts.unknowns,
                t.unk,
                m.counts.states,
                t.states,
                m.counts.discrete,
                t.discrete,
                m.discrete_equations
            ));
        }
    }

    fn report(&self, what: &str) {
        println!(
            "{what}: 両方で DAE まで {}。釣り合い 自前 {} / Rumoca {}、判定が一致 {}。方程式・未知数の数が両方一致 {}、状態の数が一致 {}。\
             構造が正則 自前 {} / Rumoca {}（比べられた {}、一致 {}）",
            self.both,
            self.mine_balanced,
            self.theirs_balanced,
            self.balanced_same,
            self.counts_same,
            self.states_same,
            self.mine_regular,
            self.theirs_regular,
            self.regular_compared,
            self.regular_same
        );
        for o in &self.other {
            println!("  他: {o}");
        }
        for d in &self.diffs {
            println!("  差: {d}");
        }
    }
}

#[test]
#[ignore = "MSL と Rumoca の読み込みが要る"]
fn validation_vs_rumoca() {
    let msl = msl_dir();
    let dirs = [root().join("src/Modelica"), msl.clone()];
    let refs: Vec<&Path> = dirs.iter().map(PathBuf::as_path).collect();
    let p = Project::from_dirs(&refs).unwrap();
    let checker = Checker::new(&refs).unwrap_or_else(|_| panic!("Rumoca がライブラリを読めない"));
    let mut tally = Tally::default();
    let (mut total, mut flat_ok, mut dae_ok) = (0, 0, 0);
    let mut why: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for cid in 0..p.lib.classes.len() {
        let name = p.lib.qualified_name(cid);
        if !name.starts_with("GlobeModeValidation.")
            || !is_model_class(&p, cid)
            || p.is_partial(cid)
        {
            continue;
        }
        let parent = p.lib.classes[cid].parent;
        if parent.is_some_and(|q| p.kind_of(q) != Some(SyntaxKind::KwPackage)) {
            continue;
        }
        total += 1;
        let Ok(f) = p.flatten(&name) else { continue };
        flat_ok += 1;
        match build(&f) {
            Ok(d) => {
                dae_ok += 1;
                if let Some(t) = rumoca(&checker, &name) {
                    tally.add(&name, &d, &t);
                }
            }
            Err(e) => why
                .entry(e.kind.to_string())
                .or_default()
                .push(format!("{name}: {}", e.detail)),
        }
    }
    println!("GlobeModeValidation: {total} 本、平らにできた {flat_ok}、DAE にできた {dae_ok}");
    tally.report("GlobeModeValidation");
    for (k, v) in &why {
        println!("  DAE にできなかった {k}: {} 例 {:?}", v.len(), v.first());
    }
}

#[test]
#[ignore = "MSL が要る（約 85 MB を読む）"]
fn msl_examples_dae() {
    let msl = msl_dir();
    let p = Project::from_dirs(&[msl.as_path()]).unwrap();
    let examples = msl_examples(&p);
    let limit: usize = std::env::var("GLOBE_DAE_LIMIT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(usize::MAX);
    let checker =
        Checker::new(&[msl.as_path()]).unwrap_or_else(|_| panic!("Rumoca がライブラリを読めない"));
    let (mut flat_ok, mut dae_ok, mut rumoca_ok) = (0, 0, 0);
    let mut tally = Tally::default();
    let mut why: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for n in examples.iter().take(limit) {
        let Ok(f) = p.flatten(n) else { continue };
        flat_ok += 1;
        match build(&f) {
            Ok(d) => {
                dae_ok += 1;
                if let Some(t) = rumoca(&checker, n) {
                    rumoca_ok += 1;
                    tally.add(n, &d, &t);
                }
            }
            Err(e) => why
                .entry(e.kind.to_string())
                .or_default()
                .push(format!("{n}: {}", e.detail)),
        }
    }
    println!(
        "MSL の例題 {} 本のうち、平らにできた {flat_ok}、DAE にできた {dae_ok}、Rumoca の釣り合いまで通った {rumoca_ok}",
        examples.len().min(limit)
    );
    tally.report("MSL");
    for (k, v) in &why {
        println!("  DAE にできなかった {k}: {} 例 {:?}", v.len(), v.first());
    }
}

/// 1 本の中身を見る（`GLOBE_DAE_SHOW=モデル名`）。差の原因を調べるとき用
#[test]
#[ignore = "MSL が要る"]
fn show_model() {
    let Ok(name) = std::env::var("GLOBE_DAE_SHOW") else {
        return;
    };
    let p = Project::from_dirs(&[msl_dir().as_path()]).unwrap();
    let f = p.flatten(&name).unwrap();
    let d = build(&f).unwrap();
    for (v, r) in f.vars.iter().zip(&d.roles) {
        println!(
            "var {} : {} {:?} {:?} dims{:?} bind={:?}",
            v.name, v.ty, v.kind, r, v.dims, v.binding
        );
    }
    for e in &f.equations {
        println!("eq[{}] {}", e.count, e.text);
    }
    for e in &f.when_equations {
        println!("when[{}] {}", e.count, e.text);
    }
    for a in &f.algorithms {
        println!("alg {a}");
    }
    println!("counts {:?} discrete_eq {}", d.counts, d.discrete_equations);
}

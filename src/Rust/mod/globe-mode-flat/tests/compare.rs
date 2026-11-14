//! 計測と Rumoca の flat との突き合わせ（`#[ignore]`。`cargo test -p globe-mode-flat --test compare -- --ignored --nocapture`）。
//!
//! - `GLOBE_MSL`: MSL のディレクトリ（既定は `data/msl`）。
//! - `GLOBE_FLAT_RUMOCA=1`: `msl_examples` でも、平らにできたものを Rumoca の flat と突き合わせる（`GLOBE_FLAT_LIMIT` で本数を絞る）。

use globe_mode_flat::{FlatModel, Project};
use globe_mode_frontend::Checker;
use globe_mode_syntax::SyntaxKind;
use std::collections::{BTreeMap, BTreeSet};
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
    names: BTreeSet<String>,
    equations: usize,
}

fn rumoca(checker: &Checker, model: &str) -> Option<Theirs> {
    let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| checker.check(&[], model)))
        .ok()?;
    let f = r.flat?;
    Some(Theirs {
        names: f.variable_names.into_iter().collect(),
        equations: f.equations,
    })
}

#[derive(Default)]
struct Tally {
    both: usize,
    names_equal: usize,
    eqs_equal: usize,
    both_equal: usize,
    first_diffs: Vec<String>,
}

impl Tally {
    fn add(&mut self, model: &str, mine: &FlatModel, theirs: &Theirs) {
        self.both += 1;
        let m: BTreeSet<String> = mine.vars.iter().map(|v| v.name.clone()).collect();
        let n_eq = m == theirs.names;
        let e_eq = mine.equation_count() == theirs.equations;
        self.names_equal += usize::from(n_eq);
        self.eqs_equal += usize::from(e_eq);
        self.both_equal += usize::from(n_eq && e_eq);
        if !(n_eq && e_eq) && self.first_diffs.len() < 25 {
            let only_m: Vec<_> = m.difference(&theirs.names).take(4).collect();
            let only_t: Vec<_> = theirs.names.difference(&m).take(4).collect();
            self.first_diffs.push(format!(
                "{model}: eq {} vs {} (rumoca); vars {} vs {}; only mine {only_m:?}; only rumoca {only_t:?}",
                mine.equation_count(),
                theirs.equations,
                m.len(),
                theirs.names.len()
            ));
        }
    }

    fn report(&self, what: &str) {
        println!(
            "{what}: 両方で平らにできた {}、名前の集合が一致 {}、方程式の数が一致 {}、両方一致 {}",
            self.both, self.names_equal, self.eqs_equal, self.both_equal
        );
        for d in &self.first_diffs {
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
    let (mut total, mut mine_ok, mut theirs_ok) = (0, 0, 0);
    let mut why: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for cid in 0..p.lib.classes.len() {
        let name = p.lib.qualified_name(cid);
        if !name.starts_with("GlobeModeValidation.")
            || !is_model_class(&p, cid)
            || p.is_partial(cid)
        {
            continue;
        }
        // モデルの中に入れ子にしたモデルは対象外（親がパッケージのものだけ）
        let parent = p.lib.classes[cid].parent;
        if parent.is_some_and(|q| p.kind_of(q) != Some(SyntaxKind::KwPackage)) {
            continue;
        }
        total += 1;
        let theirs = rumoca(&checker, &name);
        theirs_ok += usize::from(theirs.is_some());
        match p.flatten(&name) {
            Ok(m) => {
                mine_ok += 1;
                if let Some(t) = &theirs {
                    tally.add(&name, &m, t);
                }
            }
            Err(e) => why
                .entry(e.kind.to_string())
                .or_default()
                .push(format!("{name}: {}", e.detail)),
        }
    }
    println!(
        "GlobeModeValidation: {total} 本、自前で平らにできた {mine_ok}、Rumoca で flat まで通った {theirs_ok}"
    );
    tally.report("GlobeModeValidation");
    for (k, v) in &why {
        println!("  断った {k}: {}", v.len());
        for x in v.iter().take(3) {
            println!("    {x}");
        }
    }
}

#[test]
#[ignore = "MSL が要る（約 85 MB を読む）"]
fn msl_examples_flatten() {
    let msl = msl_dir();
    let t = std::time::Instant::now();
    let p = Project::from_dirs(&[msl.as_path()]).unwrap();
    println!("読み込み {:?}、クラス {}", t.elapsed(), p.lib.classes.len());
    let examples = msl_examples(&p);
    let t = std::time::Instant::now();
    let mut ok: Vec<(String, FlatModel)> = Vec::new();
    let mut why: BTreeMap<&'static str, Vec<String>> = BTreeMap::new();
    for n in &examples {
        match p.flatten(n) {
            Ok(m) => ok.push((n.clone(), m)),
            Err(e) => why
                .entry(e.kind)
                .or_default()
                .push(format!("{n}: {}", e.detail)),
        }
    }
    println!(
        "MSL の例題 {} 本のうち、自前で平らにできた {}（{:?}）",
        examples.len(),
        ok.len(),
        t.elapsed()
    );
    let mut kinds: Vec<_> = why.iter().map(|(k, v)| (v.len(), *k)).collect();
    kinds.sort_by(|a, b| b.cmp(a));
    for (n, k) in &kinds {
        println!("  断った {k}: {n}");
        let show = std::env::var("GLOBE_FLAT_SHOW").is_ok_and(|s| s.split(',').any(|x| x == *k));
        for x in why[k].iter().take(if show { usize::MAX } else { 2 }) {
            println!("    {x}");
        }
    }
    if std::env::var_os("GLOBE_FLAT_RUMOCA").is_some() {
        let limit: usize = std::env::var("GLOBE_FLAT_LIMIT")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(usize::MAX);
        let checker = Checker::new(&[msl.as_path()])
            .unwrap_or_else(|_| panic!("Rumoca がライブラリを読めない"));
        let mut tally = Tally::default();
        let mut rumoca_ok = 0;
        for (n, m) in ok.iter().take(limit) {
            if let Some(t) = rumoca(&checker, n) {
                rumoca_ok += 1;
                tally.add(n, m, &t);
            }
        }
        println!("Rumoca の flat まで通った（自前で平らにできたもののうち）: {rumoca_ok}");
        tally.report("MSL");
    }
}

/// 1 本を平らにして印字する（`GLOBE_FLAT_MODEL=Modelica.Blocks.Examples.PID_Controller`）
#[test]
#[ignore = "MSL が要る"]
fn one() {
    let Ok(models) = std::env::var("GLOBE_FLAT_MODEL") else {
        return;
    };
    let p = Project::from_dirs(&[msl_dir().as_path(), &root().join("src/Modelica")]).unwrap();
    for model in models.split(',') {
        println!("== {model}");
        match p.flatten(model) {
            Ok(m) => {
                for v in &m.vars {
                    println!(
                        "{:?} {:?} {} {:?} = {:?} start {:?}",
                        v.kind, v.causality, v.name, v.dims, v.binding, v.start
                    );
                }
                for e in &m.equations {
                    println!("[{}] {}", e.count, e.text);
                }
            }
            Err(e) => println!("断った: {e}"),
        }
    }
}

#[test]
#[ignore = "debug"]
fn refs_of() {
    let Ok(model) = std::env::var("GLOBE_FLAT_MODEL") else {
        return;
    };
    let p = Project::from_dirs(&[msl_dir().as_path()]).unwrap();
    let c = p.lib.class_by_name(&model).unwrap();
    for k in [1714usize, 1718, 1864, c] {
        println!(
            "class {k} {} {:?}",
            p.lib.qualified_name(k),
            p.lib.classes[k]
                .components
                .iter()
                .map(|c| c.name.as_str())
                .collect::<Vec<_>>()
        );
    }
    for r in p.lib.refs.iter().filter(|r| r.scope == c) {
        println!("{:?} {:?} -> {:?}", r.kind, r.parts, p.lib.resolve(r));
    }
}

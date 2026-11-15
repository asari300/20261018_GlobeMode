//! `cargo xtask diff-frontend`: Track B の自前の段（parse → resolve → flat → dae）と Rumoca を、
//! MSL の例題と `GlobeModeValidation` で一度に突き合わせ、段ごとの本数と一致の表を書く（S300）。
//!
//! - 例題の選び方は `msl-funnel` と同じ（[`crate::funnel::msl_examples`]）。`GlobeModeValidation` は
//!   `--validation`（既定 `src/Modelica`）の下の、親がパッケージの model / block / class（partial を除く）。
//! - 段ごとの検査は「はい / いいえ」の旗（[`Row::flags`]）にして、前回の JSON との差（回帰・改善）はその旗の変化で出す。
//! - Rumoca は自前で平らにできたものだけに呼ぶ（呼べば flat・釣り合い・構造まで取れる）。
//!   S278・S284・S290 の各 crate の `tests/compare.rs`（`#[ignore]`）の数え方をここに一本化した。

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::fs;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::Path;
use std::process::ExitCode;
use std::time::Instant;

use globe_mode_dae::build;
use globe_mode_flat::{FlatModel, Project};
use globe_mode_frontend::Checker;
use globe_mode_resolve::{ClassId, RefKind, Resolution};
use globe_mode_syntax::SyntaxKind;
use serde_json::{Value, json};

use crate::funnel::{gigabytes, make_checker, msl_examples, peak_rss_kb};
use crate::{MSL_SHA256, MSL_VERSION, RUMOCA_REV, RUMOCA_VERSION, Result};

pub(crate) struct Options<'a> {
    pub msl: &'a Path,
    pub validation: &'a Path,
    pub filter: Option<&'a str>,
    /// 例題でなくても、この完全修飾名のクラスをそのまま検査する（`,` 区切り。数え方の食い違いを小さなモデルで調べる。S312）
    pub classes: Option<&'a str>,
    /// 自前の flat（変数と方程式）を印字する
    pub dump: bool,
    pub limit: Option<usize>,
    pub report: Option<&'a Path>,
    pub json: Option<&'a Path>,
    /// 回帰を調べる前回の JSON（無ければ差は書かない）
    pub baseline: Option<&'a Path>,
    /// N 本ごとに Rumoca の Session を戻す（0 なら戻さない）
    pub reset_every: usize,
}

/// 旗の一覧（段、キー、表の見出し）。順序は表の順。
const FLAGS: [(&str, &str, &str); 12] = [
    ("parse", "parse", "構文の誤りが 0"),
    (
        "resolve",
        "resolve",
        "未解決の参照が 0（`extends`・部品の型をたどって）",
    ),
    ("flat", "flat", "自前で平らにできた"),
    (
        "flat",
        "rumoca_flat",
        "Rumoca も flat まで通った（比べられた本数）",
    ),
    (
        "flat",
        "flat_names",
        "変数の名前の集合が Rumoca の flat と一致",
    ),
    ("flat", "flat_eqs", "方程式の数が Rumoca の flat と一致"),
    ("dae", "dae", "自前で DAE にできた"),
    (
        "dae",
        "rumoca_balance",
        "Rumoca も釣り合いまで通った（比べられた本数）",
    ),
    (
        "dae",
        "balance",
        "釣り合いの判定（釣り合う / 釣り合わない）が一致",
    ),
    ("dae", "counts", "方程式と未知数の数が一致"),
    ("dae", "states", "状態の数が一致"),
    (
        "dae",
        "regular",
        "構造が正則かの判定が一致（Rumoca の構造が取れたもの）",
    ),
];

/// 断った理由の欄を持つ段。
const REASON_STAGES: [&str; 3] = ["resolve", "flat", "dae"];

/// 例題 1 本の結果。
struct Row {
    name: String,
    /// `msl`（例題）か `validation`（`GlobeModeValidation`）
    group: &'static str,
    /// 検査した項目だけが入る（Rumoca と比べられなかったものは無い）
    flags: BTreeMap<&'static str, bool>,
    /// 段 → 断った理由・未解決の理由
    reasons: BTreeMap<&'static str, String>,
    unresolved: usize,
    diffs: Vec<String>,
}

/// 検査器 1 つを使い回して、例題の列を回す。
fn run_row(
    dump: bool,
    p: &Project,
    checker: &Checker,
    by_scope: &[Vec<usize>],
    name: &str,
    group: &'static str,
) -> Row {
    let mut row = Row {
        name: name.to_string(),
        group,
        flags: BTreeMap::new(),
        reasons: BTreeMap::new(),
        unresolved: 0,
        diffs: Vec::new(),
    };
    let Some(cid) = p.lib.class_by_name(name) else {
        row.reasons.insert("flat", format!("not-found: {name}"));
        return row;
    };
    let file = p.lib.classes[cid].file;
    row.flags.insert(
        "parse",
        !p.lib
            .diagnostics
            .iter()
            .any(|(f, m)| *f == file && m.starts_with("syntax error")),
    );
    // resolve: 例題のクラスと、その中のクラス・`extends`・部品の型のクラス（解決できたもの）を、たどれるだけたどった参照
    let mut first = None;
    let mut seen = std::collections::HashSet::new();
    let mut stack = vec![cid];
    while let Some(c) = stack.pop() {
        if !seen.insert(c) {
            continue;
        }
        stack.extend(p.lib.classes[c].classes.values().copied());
        for &i in &by_scope[c] {
            let r = &p.lib.refs[i];
            match p.lib.resolve(r) {
                Resolution::Unresolved(why) => {
                    row.unresolved += 1;
                    first.get_or_insert(format!("{why:?}"));
                }
                Resolution::Class(b) if matches!(r.kind, RefKind::Type | RefKind::Extends) => {
                    stack.push(b)
                }
                _ => {}
            }
        }
    }
    row.flags.insert("resolve", row.unresolved == 0);
    if let Some(why) = first {
        row.reasons.insert("resolve", why);
    }
    let flat = match p.flatten(name) {
        Ok(f) => f,
        Err(e) => {
            row.flags.insert("flat", false);
            row.reasons
                .insert("flat", format!("{}: {}", e.kind, e.detail));
            return row;
        }
    };
    row.flags.insert("flat", true);
    if dump {
        for v in &flat.vars {
            println!(
                "  var {} {:?} {:?} dims={:?} bind={:?}",
                v.name, v.kind, v.ty, v.dims, v.binding
            );
        }
        for e in &flat.equations {
            println!("  eq[{}] {}", e.count, e.text);
        }
    }
    let dae = match build(&flat) {
        Ok(d) => {
            row.flags.insert("dae", true);
            Some(d)
        }
        Err(e) => {
            row.flags.insert("dae", false);
            row.reasons
                .insert("dae", format!("{}: {}", e.kind, e.detail));
            None
        }
    };
    // Rumoca は 1 回だけ呼ぶ（flat・釣り合い・構造が 1 つの結果から取れる）
    let theirs = catch_unwind(AssertUnwindSafe(|| checker.check(&[], name))).ok();
    let theirs_flat = theirs.as_ref().and_then(|r| r.flat.as_ref());
    if dump && let Some(t) = theirs_flat {
        for (o, n) in &t.equation_origins {
            println!("  rumoca[{n}] {o}");
        }
    }
    row.flags.insert("rumoca_flat", theirs_flat.is_some());
    if let Some(t) = theirs_flat {
        compare_flat(
            &mut row,
            &flat,
            t.variable_names.iter().cloned().collect(),
            t.equations,
        );
    }
    if let Some(d) = dae {
        let b = theirs.as_ref().and_then(|r| r.balance.as_ref());
        row.flags.insert("rumoca_balance", b.is_some());
        if let Some(b) = b {
            let mb = d.is_balanced();
            let same_counts = d.counts.equations == b.equations && d.counts.unknowns == b.unknowns;
            row.flags
                .insert("balance", mb == (b.equations == b.unknowns));
            row.flags.insert("counts", same_counts);
            row.flags.insert("states", d.counts.states == b.states);
            if !same_counts || d.counts.states != b.states {
                row.diffs.push(format!(
                    "dae: eq {} vs {}, unk {} vs {} (rumoca); states {} vs {}, discrete {} vs {}",
                    d.counts.equations,
                    b.equations,
                    d.counts.unknowns,
                    b.unknowns,
                    d.counts.states,
                    b.states,
                    d.counts.discrete,
                    b.discrete
                ));
            }
            if let Some(s) = theirs.as_ref().and_then(|r| r.structure.as_ref()) {
                let tr = globe_mode_analysis::analyze(s).is_regular();
                row.flags.insert("regular", d.is_regular() == tr);
                if d.is_regular() != tr {
                    row.diffs
                        .push(format!("dae: 正則 自前 {} / rumoca {tr}", d.is_regular()));
                }
            }
        }
    }
    row
}

fn compare_flat(
    row: &mut Row,
    mine: &FlatModel,
    theirs: std::collections::BTreeSet<String>,
    eqs: usize,
) {
    let m: std::collections::BTreeSet<String> = mine.vars.iter().map(|v| v.name.clone()).collect();
    let (names, e) = (m == theirs, mine.equation_count() == eqs);
    row.flags.insert("flat_names", names);
    row.flags.insert("flat_eqs", e);
    if !(names && e) {
        let only_m: Vec<_> = m.difference(&theirs).take(4).collect();
        let only_t: Vec<_> = theirs.difference(&m).take(4).collect();
        row.diffs.push(format!(
            "flat: eq {} vs {eqs} (rumoca); vars {} vs {}; only mine {only_m:?}; only rumoca {only_t:?}",
            mine.equation_count(),
            m.len(),
            theirs.len()
        ));
    }
}

fn is_model_class(p: &Project, cid: ClassId) -> bool {
    matches!(
        p.kind_of(cid),
        Some(SyntaxKind::KwModel | SyntaxKind::KwBlock | SyntaxKind::KwClass)
    )
}

/// `GlobeModeValidation.…` の model / block / class（partial と、model の中の入れ子を除く）
fn validation_models(p: &Project) -> Vec<String> {
    let mut out: Vec<String> = (0..p.lib.classes.len())
        .filter(|&c| {
            let n = p.lib.qualified_name(c);
            n.starts_with("GlobeModeValidation.")
                && is_model_class(p, c)
                && !p.is_partial(c)
                && p.lib.classes[c]
                    .parent
                    .is_none_or(|q| p.kind_of(q) == Some(SyntaxKind::KwPackage))
        })
        .map(|c| p.lib.qualified_name(c))
        .collect();
    out.sort();
    out
}

pub(crate) fn diff_frontend(opts: &Options<'_>) -> Result<ExitCode> {
    let started = Instant::now();
    let examples: Vec<String> = msl_examples(opts.msl)?
        .into_iter()
        .map(|e| e.name)
        .collect();
    let t = Instant::now();
    let dirs = [opts.msl, opts.validation];
    let p = Project::from_dirs(&dirs).map_err(|e| e.to_string())?;
    println!(
        "自前の読み込み {:.1} s、クラス {}",
        t.elapsed().as_secs_f64(),
        p.lib.classes.len()
    );
    let t = Instant::now();
    let checker = make_checker(opts.msl, &[opts.validation.to_path_buf()])?;
    println!("Rumoca の読み込み {:.1} s", t.elapsed().as_secs_f64());
    let mut by_scope = vec![Vec::new(); p.lib.classes.len()];
    for (i, r) in p.lib.refs.iter().enumerate() {
        by_scope[r.scope].push(i);
    }
    let mut targets: Vec<(String, &'static str)> =
        examples.into_iter().map(|n| (n, "msl")).collect();
    targets.extend(validation_models(&p).into_iter().map(|n| (n, "validation")));
    if let Some(cs) = opts.classes {
        targets = cs.split(',').map(|c| (c.to_string(), "msl")).collect();
    }
    if let Some(f) = opts.filter {
        targets.retain(|(n, _)| n.contains(f));
    }
    if let Some(n) = opts.limit {
        targets.truncate(n);
    }
    let baseline = read_baseline(opts.baseline);
    let mut rows = Vec::new();
    let mut run_seconds = 0.0;
    for (i, (name, group)) in targets.iter().enumerate() {
        let t = Instant::now();
        rows.push(run_row(opts.dump, &p, &checker, &by_scope, name, group));
        run_seconds += t.elapsed().as_secs_f64();
        if (i + 1) % 100 == 0 {
            println!("[{}/{}]", i + 1, targets.len());
        }
        // Rumoca のキャッシュで Session が大きくなるので、ときどき読み込んだ直後に戻す
        if opts.reset_every > 0 && (i + 1) % opts.reset_every == 0 {
            checker.reset();
        }
    }
    let elapsed = started.elapsed().as_secs_f64();
    let rss = peak_rss_kb();
    let delta = baseline.as_ref().map(|b| compare_baseline(b, &rows));
    let info = Info {
        run_seconds,
        elapsed,
        rss,
        baseline_path: opts.baseline,
        opts,
    };
    let text = render_report(&rows, &info, delta.as_ref());
    for g in ["msl", "validation"] {
        println!("{}", summary_line(&rows, g));
    }
    if let Some(d) = &delta {
        println!(
            "前回との差: 回帰 {} 本、改善 {} 本",
            d.regressions.len(),
            d.improvements.len()
        );
        for (n, f) in d.regressions.iter().take(20) {
            println!("  回帰 {n}: {}", f.join(", "));
        }
    }
    if let Some(kb) = rss {
        println!(
            "時間 {elapsed:.0} s、最大の常駐メモリ（VmHWM）{}",
            gigabytes(kb)
        );
    }
    if let Some(path) = opts.report {
        fs::write(path, text).map_err(|e| format!("{}: {e}", path.display()))?;
        println!("報告: {}", path.display());
    }
    if let Some(path) = opts.json {
        let v = render_json(&rows, &info);
        let s = serde_json::to_string_pretty(&v).map_err(|e| e.to_string())?;
        fs::write(path, s).map_err(|e| format!("{}: {e}", path.display()))?;
        println!("JSON: {}", path.display());
    }
    Ok(if delta.is_some_and(|d| !d.regressions.is_empty()) {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    })
}

struct Info<'a> {
    run_seconds: f64,
    elapsed: f64,
    rss: Option<u64>,
    baseline_path: Option<&'a Path>,
    opts: &'a Options<'a>,
}

// --------------------------------------------------------------------------- 前回との比較

/// 前回の JSON（`models[].name` と `models[].flags`）。形が変わっても読めるよう、要る欄だけを拾う。
fn read_baseline(path: Option<&Path>) -> Option<BTreeMap<String, BTreeMap<String, bool>>> {
    let path = path?;
    let Ok(text) = fs::read_to_string(path) else {
        println!(
            "注意: 前回の結果 {} が無い。前回との差は書かない",
            path.display()
        );
        return None;
    };
    let v: Value = serde_json::from_str(&text).ok()?;
    let mut out = BTreeMap::new();
    for m in v.get("models")?.as_array()? {
        let flags = m.get("flags")?.as_object()?;
        out.insert(
            m.get("name")?.as_str()?.to_string(),
            flags
                .iter()
                .filter_map(|(k, b)| Some((k.clone(), b.as_bool()?)))
                .collect(),
        );
    }
    Some(out)
}

#[derive(Default)]
struct Delta {
    /// 前回にあって通っていた旗が、通らなくなった（または無くなった）モデルと旗
    regressions: Vec<(String, Vec<String>)>,
    improvements: Vec<(String, Vec<String>)>,
    /// 前回にあって今回に無い、今回にあって前回に無いモデルの数
    gone: usize,
    added: usize,
    compared: usize,
}

fn compare_baseline(prev: &BTreeMap<String, BTreeMap<String, bool>>, rows: &[Row]) -> Delta {
    let mut d = Delta::default();
    let now: BTreeMap<&str, &Row> = rows.iter().map(|r| (r.name.as_str(), r)).collect();
    d.gone = prev
        .keys()
        .filter(|n| !now.contains_key(n.as_str()))
        .count();
    for r in rows {
        let Some(before) = prev.get(&r.name) else {
            d.added += 1;
            continue;
        };
        d.compared += 1;
        let (mut worse, mut better) = (Vec::new(), Vec::new());
        for (_, key, _) in FLAGS {
            let (b, a) = (
                before.get(key).copied().unwrap_or(false),
                r.flags.get(key).copied().unwrap_or(false),
            );
            if b && !a {
                worse.push(key.to_string());
            } else if !b && a {
                better.push(key.to_string());
            }
        }
        if !worse.is_empty() {
            d.regressions.push((r.name.clone(), worse));
        }
        if !better.is_empty() {
            d.improvements.push((r.name.clone(), better));
        }
    }
    d
}

// --------------------------------------------------------------------------- 報告

fn group_rows<'a>(rows: &'a [Row], g: &str) -> Vec<&'a Row> {
    rows.iter().filter(|r| r.group == g).collect()
}

/// `(通った本数, 母数)`。母数は、その旗を検査した本数。
fn tally(rows: &[&Row], key: &str) -> (usize, usize) {
    let seen = rows.iter().filter_map(|r| r.flags.get(key));
    (seen.clone().filter(|b| **b).count(), seen.count())
}

fn summary_line(rows: &[Row], g: &str) -> String {
    let rs = group_rows(rows, g);
    let mut s = format!("{g} {} 本:", rs.len());
    for (_, key, _) in FLAGS {
        let (n, of) = tally(&rs, key);
        let _ = write!(s, " {key} {n}/{of}");
    }
    s
}

fn group_title(g: &str) -> &'static str {
    if g == "msl" {
        "MSL の例題"
    } else {
        "GlobeModeValidation"
    }
}

fn render_report(rows: &[Row], info: &Info<'_>, delta: Option<&Delta>) -> String {
    let mut md = String::new();
    let _ = writeln!(md, "# 自前の段と Rumoca の突き合わせ（Track B）\n");
    let _ = writeln!(
        md,
        "`cargo xtask diff-frontend` が生成した。手で編集しない。\n"
    );
    render_delta(&mut md, info, delta);
    let _ = writeln!(md, "| | |\n| --- | --- |");
    let _ = writeln!(
        md,
        "| MSL | {MSL_VERSION}（sha256 `{}…`） |",
        &MSL_SHA256[..12]
    );
    let _ = writeln!(
        md,
        "| Rumoca | {RUMOCA_VERSION}（rev `{}`、ライブラリとして呼ぶ） |",
        &RUMOCA_REV[..8]
    );
    let _ = writeln!(
        md,
        "| 検査した本数 | {} 本{} |",
        rows.len(),
        selection(info.opts)
    );
    let _ = writeln!(
        md,
        "| 時間 | 全体 {:.0} s（1 本ずつの検査 {:.0} s） |",
        info.elapsed, info.run_seconds
    );
    if let Some(kb) = info.rss {
        let _ = writeln!(md, "| 最大の常駐メモリ（VmHWM） | {} |", gigabytes(kb));
    }
    let _ = writeln!(
        md,
        "\n自前の parse・resolve・flat・dae を、Rumoca は自前で平らにできたものにだけ呼んで比べる（母数はその本数）。"
    );
    let _ = writeln!(
        md,
        "resolve は、例題のクラスから、中のクラス・`extends`・部品の型（解決できたもの）をたどって、届くクラスすべての参照を数える。\n"
    );
    for g in ["msl", "validation"] {
        let rs = group_rows(rows, g);
        if rs.is_empty() {
            continue;
        }
        let _ = writeln!(md, "## {}（{} 本）\n", group_title(g), rs.len());
        let _ = writeln!(
            md,
            "| 段 | 項目 | 本数 | 母数 |\n| --- | --- | ---: | ---: |"
        );
        for (stage, key, label) in FLAGS {
            let (n, of) = tally(&rs, key);
            let _ = writeln!(md, "| {stage} | {label} | {n} | {of} |");
        }
        let _ = writeln!(
            md,
            "\n未解決の参照の総数: {}\n",
            rs.iter().map(|r| r.unresolved).sum::<usize>()
        );
        for stage in REASON_STAGES {
            render_reasons(&mut md, stage, &rs);
        }
    }
    let diffs: Vec<_> = rows
        .iter()
        .flat_map(|r| r.diffs.iter().map(move |d| (r, d)))
        .collect();
    if !diffs.is_empty() {
        let _ = writeln!(md, "## 一致しなかったもの（先頭 100 件。全部は JSON）\n");
        for (r, d) in diffs.iter().take(100) {
            let _ = writeln!(md, "- `{}`: {d}", r.name);
        }
        md.push('\n');
    }
    md
}

fn selection(o: &Options<'_>) -> String {
    let mut s = Vec::new();
    if let Some(f) = o.filter {
        s.push(format!("`--filter {f}`"));
    }
    if let Some(n) = o.limit {
        s.push(format!("`--limit {n}`"));
    }
    if s.is_empty() {
        String::new()
    } else {
        format!("（{}）", s.join("・"))
    }
}

fn render_delta(md: &mut String, info: &Info<'_>, delta: Option<&Delta>) {
    let _ = writeln!(md, "## 前回の結果との差\n");
    let (Some(d), Some(path)) = (delta, info.baseline_path) else {
        let _ = writeln!(md, "前回の結果が無かったので、差は書かない。\n");
        return;
    };
    let _ = writeln!(
        md,
        "前回の結果 `{}` と、両方にある {} 本で比べる（この実行にだけある {} 本、前回にだけある {} 本）。\n\
         **回帰 {} 本**（前回は通った項目が通らなくなった）、改善 {} 本。\n",
        path.display(),
        d.compared,
        d.added,
        d.gone,
        d.regressions.len(),
        d.improvements.len()
    );
    for (title, list) in [("回帰", &d.regressions), ("改善", &d.improvements)] {
        for (n, f) in list.iter().take(50) {
            let _ = writeln!(md, "- {title} `{n}`: {}", f.join("・"));
        }
        if list.len() > 50 {
            let _ = writeln!(md, "- {title}: ほか {} 本", list.len() - 50);
        }
        if !list.is_empty() {
            md.push('\n');
        }
    }
}

/// 段ごとの、通らなかった理由（先頭の語で数え、例を 1 つ付ける）。
fn render_reasons(md: &mut String, stage: &str, rows: &[&Row]) {
    let mut by: BTreeMap<String, (usize, &str)> = BTreeMap::new();
    for r in rows {
        if let Some(why) = r.reasons.get(stage) {
            let key = why.split(':').next().unwrap_or(why).to_string();
            let e = by.entry(key).or_insert((0, r.name.as_str()));
            e.0 += 1;
        }
    }
    if by.is_empty() {
        return;
    }
    let mut v: Vec<_> = by.into_iter().collect();
    v.sort_by(|a, b| b.1.0.cmp(&a.1.0).then(a.0.cmp(&b.0)));
    let _ = writeln!(
        md,
        "{stage} の理由（本数の多い順）:\n\n| 理由 | 本数 | 例 |\n| --- | ---: | --- |"
    );
    for (k, (n, ex)) in v {
        let _ = writeln!(md, "| `{k}` | {n} | `{ex}` |");
    }
    md.push('\n');
}

fn render_json(rows: &[Row], info: &Info<'_>) -> Value {
    json!({
        "msl_version": MSL_VERSION,
        "rumoca_rev": RUMOCA_REV,
        "seconds": info.elapsed,
        "peak_rss_kb": info.rss,
        "models": rows.iter().map(|r| json!({
            "name": r.name,
            "group": r.group,
            "flags": r.flags,
            "unresolved": r.unresolved,
            "reasons": r.reasons,
            "diffs": r.diffs,
        })).collect::<Vec<_>>(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(name: &str, flags: &[(&'static str, bool)]) -> Row {
        Row {
            name: name.into(),
            group: "msl",
            flags: flags.iter().copied().collect(),
            reasons: BTreeMap::new(),
            unresolved: 0,
            diffs: Vec::new(),
        }
    }

    #[test]
    fn regressions_are_flags_that_stopped_passing() {
        let prev: BTreeMap<String, BTreeMap<String, bool>> = [
            (
                "A".to_string(),
                [("flat", true), ("flat_names", true), ("dae", false)]
                    .map(|(k, v)| (k.to_string(), v))
                    .into(),
            ),
            ("Gone".to_string(), BTreeMap::new()),
        ]
        .into();
        let rows = [
            row("A", &[("flat", true), ("flat_names", false), ("dae", true)]),
            row("New", &[]),
        ];
        let d = compare_baseline(&prev, &rows);
        assert_eq!(
            d.regressions,
            vec![("A".to_string(), vec!["flat_names".to_string()])]
        );
        assert_eq!(
            d.improvements,
            vec![("A".to_string(), vec!["dae".to_string()])]
        );
        assert_eq!((d.compared, d.added, d.gone), (1, 1, 1));
        assert_eq!(tally(&[&rows[0]], "flat"), (1, 1));
        assert_eq!(tally(&[&rows[0]], "states"), (0, 0));
    }
}

//! 閉包の違うモデルへの切り替え（S010）。`check` は前に渡した文書を Session に残す（`Checker::check_retaining`）。
//!
//! 確かめること: 残しても結果は「閉包の文書だけで検査した結果」と同じ。使わないファイルの編集で `check` は
//! 再実行されない。本文が変わったファイル・消したファイル・名前のぶつかるファイルは Session に残らない。
//! 入力は src/Modelica/GlobeModeValidation（MSL を使う計測は `#[ignore]`）。

use std::collections::BTreeMap;
use std::path::PathBuf;

use globe_mode_core::{Stage, codes, normalize};
use globe_mode_db::GlobeDb;
use globe_mode_frontend::{CheckReport, Checker, Source};
use globe_mode_ir::ClassKind;

fn package_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../Modelica/GlobeModeValidation")
}

fn msl_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../../data/msl")
}

/// パッケージの中のファイルのパス（DB と同じく区切り `/`）。
fn file(name: &str) -> String {
    globe_mode_db::normalize_path(
        &name
            .split('/')
            .fold(package_dir(), |p, part| p.join(part))
            .display()
            .to_string(),
    )
}

fn count(executed: &[String], query: &str) -> usize {
    executed.iter().filter(|q| q.as_str() == query).count()
}

const CIRCUIT: &str = "GlobeModeValidation.Connect.RealCircuit";
const SINGULAR: &str = "GlobeModeValidation.Invalid.StructurallySingular";
const SIGNALS: &str = "GlobeModeValidation.Connect.RealSignals";

/// DB と、DB に入れたファイルの本文の写し（閉包だけの検査を別の検査器で作るため）。
struct Ws {
    db: GlobeDb,
    texts: BTreeMap<String, String>,
    /// 比べる相手: 毎回、渡した文書だけに組をそろえる検査器（`Checker::check`。Session は DB と別）
    reference: Checker,
}

impl Ws {
    fn new() -> Ws {
        Self::with_db(GlobeDb::new(), Checker::new(&[]).unwrap())
    }

    fn with_db(mut db: GlobeDb, reference: Checker) -> Ws {
        db.load_dir(&package_dir())
            .expect("GlobeModeValidation を読める");
        let texts = db
            .paths()
            .map(|p| {
                let text = std::fs::read_to_string(p).unwrap();
                (p.to_string(), text)
            })
            .collect();
        Ws {
            db,
            texts,
            reference,
        }
    }

    fn set(&mut self, path: &str, text: &str) {
        self.db.set_file(path, text);
        self.texts.insert(path.to_string(), text.to_string());
    }

    fn edit(&mut self, path: &str, from: &str, to: &str) {
        let text = self.texts[path].replace("\r\n", "\n");
        let edited = text.replace(from, to);
        assert_ne!(edited, text, "{path}: {from:?} が無い");
        self.set(path, &edited);
    }

    fn remove(&mut self, path: &str) {
        assert!(self.db.remove_file(path));
        self.texts.remove(path);
    }

    /// 閉包のファイルだけを渡して、組をそろえる検査器で検査した結果（S007 までの方式）。
    fn closure_only(&mut self, model: &str) -> CheckReport {
        let closure = self.db.deps_closure(model);
        let shapes: Vec<(String, String)> = closure
            .iter()
            .map(|p| (p.clone(), normalize::source_shape(&self.texts[p]).text))
            .collect();
        let sources: Vec<Source<'_>> = shapes
            .iter()
            .map(|(name, text)| Source { name, text })
            .collect();
        let mut report = self.reference.check(&sources, model);
        if closure.is_empty() {
            // DB の `check` は、文書を渡さなかったときに候補をワークスペースから足す。比べるために同じ候補を使う
            let ours = self.db.check(model);
            if let (Some(d), Some(o)) = (
                report
                    .diagnostics
                    .iter_mut()
                    .find(|d| d.code == codes::MODEL_NOT_FOUND),
                ours.diagnostics
                    .iter()
                    .find(|d| d.code == codes::MODEL_NOT_FOUND),
            ) && d.notes.is_empty()
            {
                d.notes = o.notes.clone();
            }
        }
        report
    }

    /// DB の `check` の結果が、閉包だけの検査と同じことを確かめて返す。
    fn check_matches(&mut self, model: &str) -> CheckReport {
        let ours = self.db.check(model);
        let reference = self.closure_only(model);
        assert_eq!(ours, reference, "{model}");
        ours
    }

    fn session_documents(&self) -> Vec<String> {
        self.db.checker().document_names()
    }
}

fn models(db: &mut GlobeDb) -> Vec<String> {
    let index = db.class_index();
    index
        .names()
        .filter(|n| {
            index
                .get(n)
                .is_some_and(|c| matches!(c.kind, ClassKind::Model | ClassKind::Block))
        })
        .map(str::to_string)
        .collect()
}

/// すべての閉包に入る package.mo の注釈（`version`）を変える。salsa の `check` の結果を捨てさせるのに使う
/// （正規化した本文が変わるので、Session の package.mo も差し替わる）。
fn touch_package(ws: &mut Ws) {
    ws.edit(&file("package.mo"), "version=\"0.0.", "version=\"0.0.1");
}

// --------------------------------------------------------------------------- 和集合

#[test]
fn switching_models_keeps_the_documents_of_both_closures() {
    let mut ws = Ws::new();
    let a = ws.check_matches(CIRCUIT);
    assert!(a.passed(), "{:?}", a.diagnostics);
    let circuit = ws.db.deps_closure(CIRCUIT);
    assert_eq!(ws.session_documents(), circuit);

    // 閉包の違うモデルへ。RealCircuit の文書は取り除かない
    ws.check_matches(SINGULAR);
    let singular = ws.db.deps_closure(SINGULAR);
    let docs = ws.session_documents();
    for p in circuit.iter().chain(&singular) {
        assert!(docs.contains(p), "{p}");
    }
    assert!(singular.iter().any(|p| !circuit.contains(p)));

    // 戻っても、文書の組は package.mo の差し替えのほかは変わらない（salsa の結果を捨てるため、両方に効く
    // ファイルを変える）
    touch_package(&mut ws);
    let back = ws.check_matches(CIRCUIT);
    assert_eq!(back, a);
    assert_eq!(ws.session_documents(), docs);
}

#[test]
fn every_model_checked_in_turn_matches_the_closure_only_check() {
    // 1 つの Session で索引のモデルをすべて順に検査し、文書が積み上がっても結果が変わらないことを確かめる。
    // 2 周目は逆順（残っている文書の組が 1 周目と違う）
    let mut ws = Ws::new();
    let all = models(&mut ws.db);
    assert!(all.len() >= 20, "{}", all.len());
    let mut first = Vec::new();
    for m in &all {
        first.push(ws.check_matches(m));
    }
    assert!(ws.session_documents().len() > ws.db.deps_closure(CIRCUIT).len());
    // salsa の結果を捨てる
    touch_package(&mut ws);
    for (m, before) in all.iter().zip(&first).rev() {
        let again = ws.check_matches(m);
        assert_eq!(&again, before, "{m}");
    }
}

#[test]
fn an_unknown_model_lists_the_same_candidates_after_other_checks() {
    let mut ws = Ws::new();
    ws.check_matches(CIRCUIT);
    ws.check_matches(SINGULAR);
    let r = ws.check_matches("GlobeModeValidation.Connect.Nope");
    assert_eq!(r.diagnostics[0].code, codes::MODEL_NOT_FOUND);
    // パッケージを指したとき（閉包はあるが、Rumoca のモデルの候補には入らない）も、
    // 残した文書のモデルを候補に混ぜない
    let r = ws.check_matches("GlobeModeValidation.Invalid");
    assert_eq!(r.failed_stage(), Some(Stage::S1), "{:?}", r.diagnostics);
}

// --------------------------------------------------------------------------- 防火壁

#[test]
fn editing_a_file_outside_the_closure_does_not_rerun_check_and_drops_the_old_text() {
    let mut ws = Ws::new();
    ws.check_matches(SINGULAR);
    let before = ws.check_matches(CIRCUIT);
    let path = file("Invalid/StructurallySingular.mo");
    assert!(ws.db.checker().retains(&path));
    ws.db.take_executed();

    // RealCircuit の閉包に入らないファイルの式を変える
    ws.edit(&path, "  x = 2;\n", "  x = 3 * y;\n");
    let after = ws.db.check(CIRCUIT);
    let executed = ws.db.take_executed();
    assert_eq!(count(&executed, "check"), 0, "{executed:?}");
    assert_eq!(after, before);
    // 古い本文は Session に残さない（次にこのファイルを使うモデルの検査で足し直す）
    assert!(!ws.session_documents().contains(&path));
    ws.check_matches(SINGULAR);
    assert!(ws.db.checker().retains(&path));
}

#[test]
fn a_comment_only_edit_keeps_the_retained_document() {
    let mut ws = Ws::new();
    ws.check_matches(SINGULAR);
    ws.check_matches(CIRCUIT);
    let path = file("Invalid/StructurallySingular.mo");
    // 渡すのは正規化した本文なので、コメントと空白だけの編集では取り除かない（解決し直さない）
    ws.edit(&path, "  x = 2;\n", "  x   = 2; // コメント\n");
    assert!(ws.db.checker().retains(&path));
    ws.check_matches(SINGULAR);
}

// --------------------------------------------------------------------------- 消す・移す

#[test]
fn removing_a_file_removes_it_from_the_session() {
    let mut ws = Ws::new();
    let extra = file("Extra.mo");
    ws.set(
        &extra,
        "within GlobeModeValidation;\nmodel Extra\n  Real x(start = 1);\nequation\n  der(x) = -x;\nend Extra;\n",
    );
    assert!(ws.check_matches("GlobeModeValidation.Extra").passed());
    ws.check_matches(CIRCUIT);
    assert!(ws.session_documents().contains(&extra));

    ws.remove(&extra);
    assert!(!ws.session_documents().contains(&extra));
    assert!(!ws.db.paths().any(|p| p == extra));
    assert!(!ws.db.remove_file(&extra));
    let r = ws.check_matches("GlobeModeValidation.Extra");
    assert!(!r.passed());
    ws.check_matches(CIRCUIT);
}

#[test]
fn renaming_a_class_does_not_leave_the_old_definition() {
    // Helper.mo の Gain を使うモデル。Gain を Gain2 に改名すると、モデルの参照は解決できなくなる。
    // 古い本文が Session に残っていると、残った Gain で解決できてしまう（閉包だけのときと違う結果）
    let mut ws = Ws::new();
    let helper = file("Helper.mo");
    let user = file("UsesHelper.mo");
    ws.set(
        &helper,
        "within GlobeModeValidation;\nmodel Gain\n  parameter Real k = 2;\n  Real y;\nequation\n  y = k;\nend Gain;\n",
    );
    ws.set(
        &user,
        "within GlobeModeValidation;\nmodel UsesHelper\n  Gain g;\nend UsesHelper;\n",
    );
    let model = "GlobeModeValidation.UsesHelper";
    assert!(ws.check_matches(model).passed());
    ws.check_matches(CIRCUIT);

    ws.edit(&helper, "model Gain", "model Gain2");
    ws.edit(&helper, "end Gain;", "end Gain2;");
    assert!(!ws.db.deps_closure(model).contains(&helper));
    let r = ws.check_matches(model);
    assert!(!r.passed(), "{:?}", r.diagnostics);
}

#[test]
fn moving_a_class_to_another_file_keeps_one_definition() {
    // Gain を Helper.mo から Moved.mo へ移す（Helper.mo には別のクラスが残る）
    let mut ws = Ws::new();
    let helper = file("Helper.mo");
    let moved = file("Moved.mo");
    let gain = "within GlobeModeValidation;\nmodel Gain\n  parameter Real k = 2;\n  Real y;\nequation\n  y = k;\nend Gain;\n";
    ws.set(&helper, gain);
    let model = "GlobeModeValidation.Gain";
    let before = ws.check_matches(model);
    assert!(before.passed(), "{:?}", before.diagnostics);
    ws.check_matches(CIRCUIT);

    ws.set(&moved, gain);
    ws.set(
        &helper,
        "within GlobeModeValidation;\nmodel Other\n  Real z;\nequation\n  z = 1;\nend Other;\n",
    );
    assert!(!ws.session_documents().contains(&helper));
    let after = ws.check_matches(model);
    assert!(after.passed(), "{:?}", after.diagnostics);
    ws.check_matches(CIRCUIT);
}

#[test]
fn a_retained_document_that_defines_the_same_class_is_removed() {
    // DB を通さずに、同じクラスを定義する別の文書を渡す（DB の閉包では、同じ名前を定義するファイルは
    // いつも一緒に入るので起きないが、検査器としては古い定義を残さない）
    let checker = Checker::new(&[]).unwrap();
    let old = "within P;\nmodel M\n  Real x;\nequation\n  x = 1;\nend M;\n";
    let new = "within P;\nmodel M\n  Real x;\nequation\n  x = 2;\nend M;\n";
    let package = "package P\nend P;\n";
    let first = [
        Source {
            name: "P/package.mo",
            text: package,
        },
        Source {
            name: "P/Old.mo",
            text: old,
        },
    ];
    assert!(checker.check_retaining(&first, "P.M").passed());
    let second = [
        Source {
            name: "P/package.mo",
            text: package,
        },
        Source {
            name: "P/New.mo",
            text: new,
        },
    ];
    let r = checker.check_retaining(&second, "P.M");
    assert_eq!(checker.document_names(), ["P/New.mo", "P/package.mo"]);
    assert_eq!(r, Checker::new(&[]).unwrap().check(&second, "P.M"));
}

#[test]
fn a_retained_document_that_does_not_resolve_does_not_affect_other_models() {
    // 名前を解決できない（S1 で止まる）ファイルを残したまま、ほかのモデルを検査する
    let mut ws = Ws::new();
    let bad = file("Bad.mo");
    ws.set(
        &bad,
        "within GlobeModeValidation;\nmodel Bad\n  NoSuchPackage.NoSuchModel m;\nend Bad;\n",
    );
    let r = ws.check_matches("GlobeModeValidation.Bad");
    assert!(!r.passed());
    assert!(ws.session_documents().contains(&bad));
    assert!(ws.check_matches(CIRCUIT).passed());
    ws.check_matches(SIGNALS);
}

#[test]
fn databases_sharing_a_checker_do_not_retain_each_others_documents() {
    let shared = Checker::new(&[]).unwrap();
    let mut one = Ws::with_db(
        GlobeDb::with_checker(shared.clone()),
        Checker::new(&[]).unwrap(),
    );
    let mut two = Ws::with_db(
        GlobeDb::with_checker(shared.clone()),
        Checker::new(&[]).unwrap(),
    );
    one.check_matches(SINGULAR);
    // 2 つ目の DB の検査は、1 つ目の DB の文書を残さない（Session は共有）
    two.check_matches(CIRCUIT);
    assert_eq!(shared.document_names(), two.db.deps_closure(CIRCUIT));
    assert!(
        !one.db
            .checker()
            .retains(&file("Invalid/StructurallySingular.mo"))
    );
    // 1 つ目に戻ると、1 つ目の閉包をまた足す
    touch_package(&mut one);
    one.check_matches(SINGULAR);
    assert_eq!(shared.document_names(), one.db.deps_closure(SINGULAR));
}

// --------------------------------------------------------------------------- 計測

/// 閉包の違うモデルへの切り替えの時間（S010 の前後の比べ）。`data/msl` が無ければ何もしない。
/// `cargo test --release -p globe-mode-db --test switching -- --ignored --nocapture` で見る。
#[test]
#[ignore]
fn measure_switching_with_msl() {
    let msl = msl_dir();
    if !msl.is_dir() {
        eprintln!("{} が無いので計測しない", msl.display());
        return;
    }
    let t = std::time::Instant::now();
    let checker = Checker::new(&[msl.as_path()]).expect("MSL を読める");
    let msl_load = ms(t);
    let mut ws = Ws::with_db(GlobeDb::with_checker(checker.clone()), checker.new_handle());
    let medium = "GlobeModeValidation.Connect.MediumFlow";
    let others = [
        "Connect.RealSignals",
        "Connect.BooleanSignals",
        "Connect.Hierarchical",
        "Init.Balanced",
        "Index.Pendulum",
    ]
    .map(|m| format!("GlobeModeValidation.{m}"));
    let rss0 = rss_mib();

    // (a) 最初の検査（MediumFlow）
    let t = std::time::Instant::now();
    ws.db.check_model(medium);
    let first = ms(t);

    // (e) 編集なしで閉包の違うモデルを初めて検査する（新しい文書を足すので、Rumoca は解決し直す）
    let mut first_visit = Vec::new();
    for m in &others {
        let t = std::time::Instant::now();
        ws.db.check_model(m);
        first_visit.push(ms(t));
    }

    // (f) すべて一度検査した後、閉包の違うモデルへ切り替える（salsa の結果を捨てるため、毎回すべての
    // 閉包に入る package.mo の注釈を変える。正規化した本文が変わるので、package.mo は差し替える）
    let mut switch_edit = Vec::new();
    for m in &others {
        touch_package(&mut ws);
        let t = std::time::Instant::now();
        ws.db.check_model(m);
        switch_edit.push(ms(t));
    }

    // (g) 編集なしで、閉包の違うモデルへ切り替える（salsa の外で、検査器を直に呼ぶ: DB と同じ取っ手の
    // `check_retaining` に閉包だけを渡す）。S007 の方式（`check`。組をそろえる）と比べる
    let mut sources_of = Vec::new();
    for m in [medium.to_string()].iter().chain(&others) {
        let shapes: Vec<(String, String)> = ws
            .db
            .deps_closure(m)
            .iter()
            .map(|p| (p.clone(), normalize::source_shape(&ws.texts[p]).text))
            .collect();
        sources_of.push((m.clone(), shapes));
    }
    let handle = ws.db.checker().clone();
    let mut retaining = Vec::new();
    let mut exact = Vec::new();
    for round in 0..2 {
        for (m, shapes) in &sources_of {
            let sources: Vec<Source<'_>> = shapes
                .iter()
                .map(|(name, text)| Source { name, text })
                .collect();
            let t = std::time::Instant::now();
            handle.check_retaining(&sources, m);
            if round == 1 {
                retaining.push(ms(t));
            }
        }
    }
    for (m, shapes) in sources_of.iter().take(3) {
        let sources: Vec<Source<'_>> = shapes
            .iter()
            .map(|(name, text)| Source { name, text })
            .collect();
        let t = std::time::Instant::now();
        handle.check(&sources, m);
        exact.push(ms(t));
    }

    // (b) 使わないファイル（Session に残っている）の式を変えた後の再検査（check は 0 回）
    ws.db.check_model(medium);
    let unused = file("Invalid/StructurallySingular.mo");
    ws.db.check_model(SINGULAR);
    ws.db.check_model(medium);
    ws.db.take_executed();
    let mut unused_times = Vec::new();
    for i in 0..5 {
        ws.edit(&unused, "  x = ", &format!("  x = {} + ", i + 3));
        let t = std::time::Instant::now();
        ws.db.check_model(medium);
        unused_times.push(ms(t));
    }
    let executed = ws.db.take_executed();
    assert_eq!(count(&executed, "check"), 0, "{executed:?}");
    let rss1 = rss_mib();

    eprintln!("MSL の読み込み: {msl_load:.0} ms");
    eprintln!("(a) MediumFlow の最初の検査: {first:.0} ms");
    for (label, times) in [
        (
            "(e) 編集なしで閉包の違うモデルを初めて検査",
            &mut first_visit,
        ),
        (
            "(f) 一度検査したモデルへ切り替え（package.mo を変えて）",
            &mut switch_edit,
        ),
        (
            "(g) 一度検査したモデルへ切り替え（check_retaining、6 本）",
            &mut retaining,
        ),
        ("参考: 組をそろえる check で切り替え（3 本）", &mut exact),
        ("(b) 使わないファイルの式を変えた後", &mut unused_times),
    ] {
        times.sort_by(f64::total_cmp);
        eprintln!(
            "{label}: 中央値 {:.2} ms / 最大 {:.2} ms（{} 回）",
            times[times.len() / 2],
            times[times.len() - 1],
            times.len()
        );
    }
    eprintln!(
        "Session の文書: {} 個（全 {} ファイル）。RSS: 検査の前 {rss0:.0} MiB → 後 {rss1:.0} MiB",
        ws.session_documents().len(),
        ws.db.paths().count()
    );
}

fn ms(t: std::time::Instant) -> f64 {
    t.elapsed().as_secs_f64() * 1000.0
}

/// いまの RSS（MiB）。/proc が無ければ 0。
fn rss_mib() -> f64 {
    std::fs::read_to_string("/proc/self/status")
        .ok()
        .and_then(|s| {
            s.lines()
                .find(|l| l.starts_with("VmRSS:"))
                .and_then(|l| l.split_whitespace().nth(1))
                .and_then(|kb| kb.parse::<f64>().ok())
        })
        .map_or(0.0, |kb| kb / 1024.0)
}

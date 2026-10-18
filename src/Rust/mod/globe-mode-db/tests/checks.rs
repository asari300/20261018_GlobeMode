//! `check` / `dae_shape` / `structural` のクエリと、防火壁 0（`source_shape`）・防火壁 2（`dae_shape`）のテスト。
//! 入力は src/Modelica/GlobeModeValidation（MSL を使わないモデルだけを検査する）。

use std::path::PathBuf;

use globe_mode_core::{Stage, codes, line_col};
use globe_mode_db::GlobeDb;
use globe_mode_frontend::{Checker, Source};
use globe_mode_ir::UnknownKind;

fn package_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../Modelica/GlobeModeValidation")
}

fn loaded() -> GlobeDb {
    let mut db = GlobeDb::new();
    db.load_dir(&package_dir())
        .expect("GlobeModeValidation を読める");
    db
}

/// パッケージの中のファイルのパス（`load_dir` が読んだパスと同じ形）。
fn file(name: &str) -> String {
    name.split('/')
        .fold(package_dir(), |p, part| p.join(part))
        .display()
        .to_string()
}

/// ファイルの本文（Windows の CRLF でも置換が当たるように LF にそろえる）。
fn read(path: &str) -> String {
    std::fs::read_to_string(path).unwrap().replace("\r\n", "\n")
}

fn count(executed: &[String], query: &str) -> usize {
    executed.iter().filter(|q| q.as_str() == query).count()
}

const CIRCUIT: &str = "GlobeModeValidation.Connect.RealCircuit";
const SIGNALS: &str = "GlobeModeValidation.Connect.RealSignals";

// --------------------------------------------------------------------------- 結果

#[test]
fn real_circuit_passes_s0_to_s6() {
    let mut db = loaded();
    let r = db.check_model(CIRCUIT);
    assert!(r.passed(), "{:?}", r.diagnostics);
    assert_eq!(r.failed_stage(), None);
    let s = r.check.structure.as_ref().expect("structure");
    assert_eq!((s.unknowns.len(), s.equations.len()), (20, 20));
    let s6 = r.structural.as_ref().expect("S6");
    assert!(s6.is_regular());
    assert_eq!(s6.matching, 20);
}

#[test]
fn structural_matching_agrees_with_rumoca() {
    // 自前の S6（globe-mode-analysis の Hopcroft–Karp）と、Rumoca の phase_structural::analyze を照合する
    let files: Vec<(String, String)> = {
        let mut paths = Vec::new();
        collect(&package_dir(), &mut paths);
        paths.sort();
        paths
            .into_iter()
            .map(|p| {
                (
                    p.display().to_string(),
                    std::fs::read_to_string(&p).unwrap(),
                )
            })
            .collect()
    };
    let sources: Vec<Source<'_>> = files
        .iter()
        .map(|(n, t)| Source { name: n, text: t })
        .collect();
    let checker = Checker::new(&[]).unwrap();
    for model in [
        "Connect.RealCircuit",
        "Connect.RealSignals",
        "Connect.BooleanSignals",
        "Connect.Hierarchical",
        "Invalid.StructurallySingular",
    ] {
        let model = format!("GlobeModeValidation.{model}");
        let report = checker.check(&sources, &model);
        let structure = report.structure.as_ref().expect(&model);
        let ours = globe_mode_analysis::analyze(structure);
        let reference = checker.reference_structure(&sources, &model).expect(&model);
        assert_eq!(
            (ours.n_equations, ours.n_unknowns, ours.matching),
            (
                reference.n_equations,
                reference.n_unknowns,
                reference.matching
            ),
            "{model}"
        );
    }

    fn collect(dir: &std::path::Path, out: &mut Vec<PathBuf>) {
        for e in std::fs::read_dir(dir).unwrap() {
            let p = e.unwrap().path();
            if p.is_dir() {
                collect(&p, out);
            } else if p.extension().is_some_and(|x| x == "mo") {
                out.push(p);
            }
        }
    }
}

#[test]
fn state_refs_point_at_derivatives_and_merged_graph_matches() {
    let mut db = loaded();
    // GlobeModeExamples は別のパッケージなので、同じ DB に足す
    let examples = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../Modelica/GlobeModeExamples/package.mo");
    db.set_file(
        &examples.display().to_string(),
        std::fs::read_to_string(&examples).unwrap(),
    );
    for model in [
        "GlobeModeExamples.RC",
        "GlobeModeValidation.Invalid.StructurallySingular",
        "GlobeModeValidation.Connect.Hierarchical",
    ] {
        let s = db.dae_shape(model).expect(model);
        assert_eq!(s.state_refs.len(), s.equations.len(), "{model}");
        for row in &s.state_refs {
            assert!(
                row.windows(2).all(|w| w[0] < w[1]),
                "{model}: 昇順・重複なし"
            );
            for &u in row {
                assert_eq!(
                    s.unknowns[u as usize].kind,
                    UnknownKind::Derivative,
                    "{model}: state_refs は der(x) の添字だけ"
                );
            }
        }
        if model == "GlobeModeValidation.Invalid.StructurallySingular" {
            continue;
        }
        assert!(
            s.state_refs.iter().any(|r| !r.is_empty()),
            "{model}: 状態を直接参照する方程式がある"
        );
        let n = s.equations.len();
        let merged = globe_mode_analysis::hopcroft_karp(n, s.unknowns.len(), &s.merged_incidence());
        assert_eq!(merged.size, n, "{model}: まとめたグラフで完全マッチング");
    }
}

#[test]
fn structurally_singular_stops_at_s6_with_positions() {
    // a を 2 回決め、b を決める方程式が無い（x と der(x) をまとめたグラフでも、der の形でも特異）
    let text = "// 先頭のコメント\n/* ブロック\n   コメント */\nmodel Singular\n  Real x(start = 1);\n  Real a;\n  Real b \"どの方程式にも現れない\";\nequation\n  der(x) = -x;\n  a = 2 * x;\n  a = 3 * x;\nend Singular;\n";
    let mut db = GlobeDb::new();
    db.set_file("Singular.mo", text);
    let r = db.check_model("Singular");
    assert!(r.check.passed(), "S5 までは通る: {:?}", r.check.diagnostics);
    assert_eq!(r.failed_stage(), Some(Stage::S6), "{:?}", r.diagnostics);
    let located: Vec<_> = r
        .diagnostics
        .iter()
        .filter(|d| d.code == codes::SINGULAR && d.range.is_some())
        .collect();
    assert!(!located.is_empty(), "{:?}", r.diagnostics);
    for d in &located {
        assert_eq!(d.file.as_deref(), Some("Singular.mo"));
        assert!(line_col(text, d.range.unwrap().start).line > 3, "{d:?}");
    }
    // 不足している未知数 b は、元の本文の宣言の位置（7 行目）を指す
    let b = located
        .iter()
        .find(|d| d.message.contains("`b`"))
        .expect("b の診断");
    let range = b.range.unwrap();
    assert_eq!(line_col(text, range.start).line, 7);
    assert!(text[range.start as usize..range.end as usize].contains('b'));
}

#[test]
fn syntax_error_positions_are_mapped_back_to_the_original_text() {
    let mut db = loaded();
    let path = file("Connect/RealSignals.mo");
    // コメントと空白の後ろに構文の誤り（`;` が無い）
    let text = "within GlobeModeValidation.Connect;\n// コメント\n\n/* もう 1 つ */\nmodel RealSignals\n  Real x\nend RealSignals;\n";
    db.set_file(&path, text);
    let r = db.check_model(SIGNALS);
    assert_eq!(r.failed_stage(), Some(Stage::S0));
    let d = &r.diagnostics[0];
    assert_eq!(d.code, codes::SYNTAX);
    let range = d.range.expect("range");
    // check のままの位置は正規化した本文の中、FullReport の位置は元の本文の中
    let raw = r.check.diagnostics[0].range.unwrap();
    assert!(range.start > raw.start);
    assert!(line_col(text, range.start).line >= 6, "{range:?}");
}

#[test]
fn unbalanced_flat_summary_is_available() {
    let mut db = GlobeDb::new();
    db.set_file(
        "Unbalanced.mo",
        "model Unbalanced\n  Real x(start = 1);\n  Real y;\nequation\n  der(x) = -x;\nend Unbalanced;\n",
    );
    let r = db.check_model("Unbalanced");
    assert_eq!(r.failed_stage(), Some(Stage::S5));
    assert!(r.structural.is_none());
    let flat = db.flat_summary("Unbalanced").expect("flat");
    assert_eq!((flat.variables, flat.equations), (2, 1));
}

// --------------------------------------------------------------------------- 防火壁

#[test]
fn nothing_reruns_without_changes() {
    let mut db = loaded();
    db.check_model(CIRCUIT);
    let first = db.take_executed();
    assert_eq!(count(&first, "check"), 1);
    assert_eq!(count(&first, "structural"), 1);
    db.check_model(CIRCUIT);
    assert!(db.take_executed().is_empty(), "変更が無いのに再実行された");
}

#[test]
fn whitespace_and_comment_edits_stop_at_firewall_0() {
    let mut db = loaded();
    let path = file("Connect/RealCircuit.mo");
    let before = db.check_model(CIRCUIT);
    db.take_executed();

    let text = read(&path);
    let edited = text.replace(
        "equation\n",
        "\n  // コメントを足し、空白を増やした\n\nequation\n\n   /* ブロック */",
    );
    assert_ne!(edited, text);
    db.set_file(&path, edited);

    let after = db.check_model(CIRCUIT);
    let executed = db.take_executed();
    assert_eq!(count(&executed, "shape"), 1, "{executed:?}");
    assert_eq!(count(&executed, "source_shape"), 1, "{executed:?}");
    assert_eq!(count(&executed, "check"), 0, "{executed:?}");
    assert_eq!(count(&executed, "dae_shape"), 0, "{executed:?}");
    assert_eq!(count(&executed, "structural"), 0, "{executed:?}");
    assert_eq!(after.structural, before.structural);
}

#[test]
fn comment_edits_in_another_file_also_stop_at_firewall_0() {
    let mut db = loaded();
    let path = file("Connectors.mo");
    db.check_model(CIRCUIT);
    db.take_executed();

    let text = read(&path);
    db.set_file(&path, format!("// 先頭にコメント\n\n{text}\n/* 末尾 */\n"));
    db.check_model(CIRCUIT);
    let executed = db.take_executed();
    assert_eq!(count(&executed, "shape"), 1, "{executed:?}");
    assert_eq!(count(&executed, "check"), 0, "{executed:?}");
    assert_eq!(count(&executed, "dae_shape"), 0, "{executed:?}");
    assert_eq!(count(&executed, "structural"), 0, "{executed:?}");
}

#[test]
fn a_new_connect_reaches_structural() {
    let mut db = loaded();
    let path = file("Connect/RealSignals.mo");
    let before = db.check_model(SIGNALS);
    db.take_executed();

    let text = read(&path);
    let edited = text
        .replace(
            "  Components.RealGain gain2(k = 0.5);\n",
            "  Components.RealGain gain2(k = 0.5);\n  Components.RealGain gain3;\n",
        )
        .replace(
            "  connect(gain1.y, gain2.u);\n",
            "  connect(gain1.y, gain2.u);\n  connect(gain2.y, gain3.u);\n",
        );
    assert_ne!(edited, text);
    db.set_file(&path, edited);

    let after = db.check_model(SIGNALS);
    let executed = db.take_executed();
    assert_eq!(count(&executed, "check"), 1, "{executed:?}");
    assert_eq!(count(&executed, "dae_shape"), 1, "{executed:?}");
    assert_eq!(count(&executed, "structural"), 1, "{executed:?}");
    let (b, a) = (before.structural.unwrap(), after.structural.unwrap());
    assert_eq!((b.n_equations, a.n_equations), (5, 7));
    assert!(a.is_regular());
}

#[test]
fn a_parameter_value_edit_stops_at_firewall_2() {
    let mut db = loaded();
    let path = file("Connect/RealSignals.mo");
    let text = read(&path);
    let before = db.check_model(SIGNALS);
    db.take_executed();

    // 同じ長さの値: 位置も変わらないので、check の結果ごと前と等しい（check より先は動かない）
    db.set_file(&path, text.replace("gain2(k = 0.5)", "gain2(k = 0.7)"));
    let r = db.check_model(SIGNALS);
    assert!(r.passed(), "{:?}", r.diagnostics);
    let executed = db.take_executed();
    assert_eq!(count(&executed, "check"), 1, "{executed:?}");
    assert_eq!(count(&executed, "dae_shape"), 0, "{executed:?}");
    assert_eq!(count(&executed, "structural"), 0, "{executed:?}");

    // 長さの違う値: 後ろの方程式（connect）の位置がずれて check の結果は変わるが、
    // 位置を落とした DAE の構造は同じなので、S6 はやり直さない
    db.set_file(&path, text.replace("gain2(k = 0.5)", "gain2(k = 0.125)"));
    let after = db.check_model(SIGNALS);
    assert!(after.passed(), "{:?}", after.diagnostics);
    let executed = db.take_executed();
    assert_eq!(count(&executed, "check"), 1, "{executed:?}");
    assert_eq!(count(&executed, "dae_shape"), 1, "{executed:?}");
    assert_eq!(count(&executed, "structural"), 0, "{executed:?}");
    assert_ne!(after.check.structure, before.check.structure);
    assert_eq!(after.structural, before.structural);
}

/// 空白だけの編集のあとの再検査の時間（P2 の完了条件: 50 ms 未満、`check` が再実行されないこと）。
/// 時間は環境で揺れるので CI では検査せず、`cargo test --release -p globe-mode-db -- --ignored --nocapture` で見る。
#[test]
#[ignore]
fn measure_recheck_after_whitespace_edit() {
    let mut db = loaded();
    let path = file("Connect/RealCircuit.mo");
    let text = read(&path);
    let t = std::time::Instant::now();
    db.check_model(CIRCUIT);
    let first = t.elapsed().as_secs_f64() * 1000.0;
    db.take_executed();
    let mut times = Vec::new();
    for i in 0..20 {
        db.set_file(&path, format!("{text}{}", "\n ".repeat(i + 1)));
        let t = std::time::Instant::now();
        let r = db.check_model(CIRCUIT);
        times.push(t.elapsed().as_secs_f64() * 1000.0);
        assert!(r.passed());
    }
    let executed = db.take_executed();
    assert_eq!(count(&executed, "check"), 0, "{executed:?}");
    times.sort_by(f64::total_cmp);
    eprintln!(
        "空白だけの編集のあとの再検査（check_model）: 中央値 {:.2} ms / 最大 {:.2} ms（20 回）。最初の検査 {first:.1} ms",
        times[10], times[19]
    );
    assert!(times[10] < 50.0, "中央値 {:.2} ms", times[10]);
}

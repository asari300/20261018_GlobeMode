//! src/Modelica/GlobeModeValidation（Modelica のパッケージ）を使った、connect グラフと遅延評価のテスト。

use std::collections::BTreeMap;
use std::path::PathBuf;

use globe_mode_db::GlobeDb;
use globe_mode_graph::{Domain, NodeKind};

fn package_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../Modelica/GlobeModeValidation")
}

fn loaded() -> GlobeDb {
    let mut db = GlobeDb::new();
    let n = db
        .load_dir(&package_dir())
        .expect("GlobeModeValidation を読める");
    assert!(n >= 10, "ファイルが少なすぎる: {n}");
    db
}

fn file(name: &str) -> String {
    package_dir().join(name).display().to_string()
}

fn domains(db: &mut GlobeDb, model: &str, depth: u32) -> BTreeMap<Domain, usize> {
    db.connect_graph(&format!("GlobeModeValidation.Connect.{model}"), depth)
        .expect("グラフ")
        .domain_counts()
}

// --------------------------------------------------------------------------- connect の型

#[test]
fn package_is_indexed_with_qualified_names() {
    let mut db = loaded();
    let index = db.class_index();
    for name in [
        "GlobeModeValidation",
        "GlobeModeValidation.Connectors.Pin",
        "GlobeModeValidation.Connectors.FluidPort",
        "GlobeModeValidation.Components.Resistor",
        "GlobeModeValidation.Connect.RealCircuit",
        "GlobeModeValidation.Invalid.SignalTypeMismatch",
    ] {
        assert!(index.get(name).is_some(), "{name} が索引に無い");
    }
}

#[test]
fn real_potential_and_flow() {
    let mut db = loaded();
    let g = db
        .connect_graph("GlobeModeValidation.Connect.RealCircuit", 1)
        .unwrap();
    assert_eq!(g.nodes.len(), 4);
    assert_eq!(g.domain_counts(), BTreeMap::from([(Domain::Flow, 4)]));
    assert!(
        g.edges
            .iter()
            .all(|e| e.connector == "GlobeModeValidation.Connectors.Pin")
    );
    // 継承した connect の端点（TwoPin の p / n）も、部品のノードに付く
    assert!(
        g.edges
            .iter()
            .any(|e| e.source == "capacitor" && e.target == "source")
    );
}

#[test]
fn real_signals() {
    let mut db = loaded();
    assert_eq!(
        domains(&mut db, "RealSignals", 1),
        BTreeMap::from([(Domain::Real, 2)])
    );
}

#[test]
fn boolean_and_real_signals_mixed() {
    let mut db = loaded();
    assert_eq!(
        domains(&mut db, "BooleanSignals", 1),
        BTreeMap::from([(Domain::Boolean, 2), (Domain::Real, 1)])
    );
}

#[test]
fn medium_ports_are_stream() {
    let mut db = loaded();
    let g = db
        .connect_graph("GlobeModeValidation.Connect.MediumFlow", 1)
        .unwrap();
    assert_eq!(g.domain_counts(), BTreeMap::from([(Domain::Stream, 2)]));
    assert!(
        g.edges
            .iter()
            .all(|e| e.connector == "GlobeModeValidation.Connectors.FluidPort")
    );
}

#[test]
fn hierarchy_expands_with_depth() {
    let mut db = loaded();
    let g1 = db
        .connect_graph("GlobeModeValidation.Connect.Hierarchical", 1)
        .unwrap();
    assert_eq!((g1.nodes.len(), g1.edges.len()), (4, 4));

    let g2 = db
        .connect_graph("GlobeModeValidation.Connect.Hierarchical", 2)
        .unwrap();
    assert_eq!((g2.nodes.len(), g2.edges.len()), (10, 10));
    let joints: Vec<&str> = g2
        .nodes
        .iter()
        .filter(|n| n.kind == NodeKind::Connector)
        .map(|n| n.key.as_str())
        .collect();
    assert_eq!(joints, ["stage1.n", "stage1.p", "stage2.n", "stage2.p"]);
    // 外側の connect（source.p — stage1.p）と内側の connect（p — resistor.p）が、つなぎ目で出会う
    assert!(
        g2.edges
            .iter()
            .any(|e| e.source == "source" && e.target == "stage1.p")
    );
    assert!(
        g2.edges
            .iter()
            .any(|e| e.source == "stage1.p" && e.target == "stage1.resistor")
    );
}

#[test]
fn graphology_json_has_nodes_and_edges() {
    let mut db = loaded();
    let g = db
        .connect_graph("GlobeModeValidation.Connect.BooleanSignals", 1)
        .unwrap();
    let json = g.to_graphology();
    assert_eq!(json["nodes"].as_array().unwrap().len(), 4);
    assert_eq!(json["edges"][0]["attributes"]["domain"], "boolean");
}

#[test]
fn unknown_model_is_an_error() {
    let mut db = loaded();
    assert!(
        db.connect_graph("GlobeModeValidation.Connect.Nope", 1)
            .is_err()
    );
}

// --------------------------------------------------------------------------- 遅延評価（防火壁）

fn count(executed: &[String], query: &str) -> usize {
    executed.iter().filter(|q| q.as_str() == query).count()
}

#[test]
fn nothing_reruns_without_changes() {
    let mut db = loaded();
    db.connect_graph("GlobeModeValidation.Connect.RealCircuit", 1)
        .unwrap();
    let first = db.take_executed();
    assert_eq!(count(&first, "connect_graph"), 1);
    assert_eq!(count(&first, "class_index"), 1);

    db.connect_graph("GlobeModeValidation.Connect.RealCircuit", 1)
        .unwrap();
    assert!(db.take_executed().is_empty(), "変更が無いのに再実行された");
}

#[test]
fn whitespace_and_comment_edits_stop_at_the_firewall() {
    let mut db = loaded();
    let path = file("Connect/RealCircuit.mo");
    let before = db
        .connect_graph("GlobeModeValidation.Connect.RealCircuit", 1)
        .unwrap();
    db.take_executed();

    let text = std::fs::read_to_string(&path).unwrap();
    let edited = text.replace(
        "equation\n",
        "\n  // コメントを足し、空白を増やした\n\nequation\n\n",
    );
    assert_ne!(edited, text);
    db.set_file(&path, edited);

    let after = db
        .connect_graph("GlobeModeValidation.Connect.RealCircuit", 1)
        .unwrap();
    let executed = db.take_executed();
    assert_eq!(after, before);
    // 編集したファイルの outline は作り直す（エディタの位置が変わるため）
    assert_eq!(count(&executed, "outline"), 1, "{executed:?}");
    assert_eq!(count(&executed, "outline_shape"), 1, "{executed:?}");
    // 位置を落とした写しが前と等しいので、索引とグラフは作り直さない
    assert_eq!(count(&executed, "class_index"), 0, "{executed:?}");
    assert_eq!(count(&executed, "connect_graph"), 0, "{executed:?}");
}

#[test]
fn edits_in_another_file_also_stop_at_the_firewall() {
    let mut db = loaded();
    let path = file("Connectors.mo");
    db.connect_graph("GlobeModeValidation.Connect.MediumFlow", 1)
        .unwrap();
    db.take_executed();

    let text = std::fs::read_to_string(&path).unwrap();
    db.set_file(&path, format!("// 先頭にコメント\n\n{text}"));
    db.connect_graph("GlobeModeValidation.Connect.MediumFlow", 1)
        .unwrap();
    let executed = db.take_executed();
    assert_eq!(count(&executed, "class_index"), 0, "{executed:?}");
    assert_eq!(count(&executed, "connect_graph"), 0, "{executed:?}");
}

#[test]
fn a_new_connect_reaches_the_graph() {
    let mut db = loaded();
    let path = file("Connect/RealSignals.mo");
    let before = db
        .connect_graph("GlobeModeValidation.Connect.RealSignals", 1)
        .unwrap();
    db.take_executed();

    let text = std::fs::read_to_string(&path).unwrap();
    let edited = text.replace(
        "  Components.RealGain gain2(k = 0.5);\n",
        "  Components.RealGain gain2(k = 0.5);\n  Components.RealGain gain3;\n",
    );
    let edited = edited.replace(
        "  connect(gain1.y, gain2.u);\n",
        "  connect(gain1.y, gain2.u);\n  connect(gain2.y, gain3.u);\n",
    );
    db.set_file(&path, edited);

    let after = db
        .connect_graph("GlobeModeValidation.Connect.RealSignals", 1)
        .unwrap();
    let executed = db.take_executed();
    assert_eq!(count(&executed, "class_index"), 1, "{executed:?}");
    assert_eq!(count(&executed, "connect_graph"), 1, "{executed:?}");
    assert_eq!((before.edges.len(), after.edges.len()), (2, 3));
    assert_eq!(after.nodes.len(), 4);
}

#[test]
fn syntax_error_keeps_other_files_usable() {
    let mut db = loaded();
    let path = file("Connect/RealSignals.mo");
    db.set_file(
        &path,
        "within GlobeModeValidation.Connect;\nmodel RealSignals\n  Real x\nend RealSignals;\n",
    );
    // 壊れたファイルのクラスは索引から消えるが、ほかのモデルのグラフは作れる
    assert!(db.outline(&path).is_some_and(|r| r.is_err()));
    assert!(
        db.connect_graph("GlobeModeValidation.Connect.RealSignals", 1)
            .is_err()
    );
    assert!(
        db.connect_graph("GlobeModeValidation.Connect.RealCircuit", 1)
            .is_ok()
    );
}

/// 空白だけの編集のあとの再計算の時間（P2 の完了条件の目安: 50 ms 未満）。
/// 時間は環境で揺れるので CI では検査せず、`cargo test --release -p globe-mode-db -- --ignored --nocapture` で見る。
#[test]
#[ignore]
fn measure_requery_after_whitespace_edit() {
    let mut db = loaded();
    let path = file("Connect/RealCircuit.mo");
    let text = std::fs::read_to_string(&path).unwrap();
    db.connect_graph("GlobeModeValidation.Connect.RealCircuit", 2)
        .unwrap();
    let mut times = Vec::new();
    for i in 0..20 {
        db.set_file(&path, format!("{text}{}", " ".repeat(i + 1)));
        let t = std::time::Instant::now();
        db.connect_graph("GlobeModeValidation.Connect.RealCircuit", 2)
            .unwrap();
        times.push(t.elapsed().as_secs_f64() * 1000.0);
    }
    times.sort_by(f64::total_cmp);
    eprintln!(
        "空白だけの編集のあとの再計算: 中央値 {:.2} ms / 最大 {:.2} ms（20 回）",
        times[10], times[19]
    );
}

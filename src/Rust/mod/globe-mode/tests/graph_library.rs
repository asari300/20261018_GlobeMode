//! ライブラリとして読んだモデルの connect グラフ（S079）。`check` と同じく、ワークスペースに無いモデルも引ける。

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use globe_mode::Workspace;

const MODEL: &str = "GlobeModeValidation.Connect.RealCircuit";

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../..")
}

fn sources(dir: &Path, out: &mut BTreeMap<String, String>) {
    for e in std::fs::read_dir(dir).unwrap() {
        let p = e.unwrap().path();
        if p.is_dir() {
            sources(&p, out);
        } else if p.extension().is_some_and(|e| e == "mo") {
            out.insert(
                p.display().to_string(),
                std::fs::read_to_string(&p).unwrap(),
            );
        }
    }
}

fn nodes(ws: &mut Workspace, model: &str) -> usize {
    let graph = ws.graph(model, 1).expect("ライブラリのモデルのグラフ");
    assert!(!graph.edges.is_empty(), "{model}");
    graph.nodes.len()
}

#[test]
fn library_model_graph() {
    let base = root().join("src/Modelica");
    let mut files = BTreeMap::new();
    sources(&base.join("GlobeModeValidation"), &mut files);
    let counts = [
        nodes(
            &mut Workspace::with_libraries(std::slice::from_ref(&base)).unwrap(),
            MODEL,
        ),
        nodes(&mut Workspace::with_libraries_full(&[base]).unwrap(), MODEL),
        nodes(
            &mut Workspace::with_library_sources_split("GlobeModeValidation", &files).unwrap(),
            MODEL,
        ),
        nodes(
            &mut Workspace::with_library_sources("GlobeModeValidation", &files).unwrap(),
            MODEL,
        ),
    ];
    assert_eq!(counts, [4; 4]);
    // 索引に無いモデルは今までどおり Err
    assert!(
        Workspace::with_libraries(&[root().join("src/Modelica")])
            .unwrap()
            .graph("GlobeModeValidation.NoSuchModel", 1)
            .is_err()
    );
}

#[test]
fn msl_chua_circuit_graph() {
    let msl = root().join("data/msl");
    if !msl.is_dir() {
        eprintln!("{} が無いので飛ばす", msl.display());
        return;
    }
    let mut ws = Workspace::with_libraries(&[msl]).unwrap();
    let n = nodes(&mut ws, "Modelica.Electrical.Analog.Examples.ChuaCircuit");
    eprintln!("ChuaCircuit のノード {n}");
    assert!(n > 0);
    // PartialStraightPipe の入れ子の `replaceable model FlowModel` を受け継いだ型で引く（S101）
    let g = ws.graph("Modelica.Fluid.Pipes.StaticPipe", 1).unwrap();
    assert!(
        g.nodes.iter().any(|n| n.key == "flowModel"),
        "{:?}",
        g.unresolved
    );
}

/// ワークスペースのモデルの中のライブラリの部品も、ライブラリの索引から引く（S081）。2 回目はアウトラインを持ち越す
#[test]
fn workspace_model_with_msl_parts() {
    let msl = root().join("data/msl");
    if !msl.is_dir() {
        eprintln!("{} が無いので飛ばす", msl.display());
        return;
    }
    let text = include_str!("data/MslParts.mo");
    let mut plain = Workspace::new();
    plain.set_file("MslParts.mo", text);
    let before = plain.graph("MslParts", 1).unwrap().unresolved;
    for open in [Workspace::with_libraries, Workspace::with_libraries_full] {
        let mut ws = open(std::slice::from_ref(&msl)).unwrap();
        ws.set_file("MslParts.mo", text);
        let after = ws.graph("MslParts", 1).unwrap().unresolved;
        eprintln!("unresolved {} → {}: {after:?}", before.len(), after.len());
        assert!(after.len() < before.len(), "{before:?} → {after:?}");
    }
    // import で書いた名前も解決する（S089）
    let mut ws = Workspace::with_libraries(std::slice::from_ref(&msl)).unwrap();
    ws.set_file("MslPartsImport.mo", include_str!("data/MslPartsImport.mo"));
    let graph = ws.graph("MslPartsImport", 1).unwrap();
    assert!(graph.unresolved.is_empty(), "{:?}", graph.unresolved);
    assert!(!graph.edges.is_empty());
    let mut ws = Workspace::with_libraries_full(&[msl]).unwrap();
    for i in 1..=2 {
        let start = std::time::Instant::now();
        let g = ws
            .graph("Modelica.Electrical.Analog.Examples.ChuaCircuit", 1)
            .unwrap();
        eprintln!(
            "ChuaCircuit の graph {i} 回目 {:?}（{} ノード・{} 辺）",
            start.elapsed(),
            g.nodes.len(),
            g.edges.len()
        );
        assert_eq!((g.nodes.len(), g.edges.len()), (7, 9));
    }
}

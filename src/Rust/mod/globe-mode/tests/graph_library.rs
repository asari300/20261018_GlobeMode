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
}

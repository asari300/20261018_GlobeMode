//! ネイティブで [`globe_mode_wasm`] の Rust の形（[`Workspace::run`]・[`run`]）を回す。
//! wasm の上の試験は node/smoke.mjs と node/compare.mjs（`mise run wasm:node`）。

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use globe_mode_wasm::{Request, Workspace, run, solvers_json};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../..")
}

/// `dir` の下の `.mo` を（`root` からの相対の名前 → 本文で）読む。
fn read_mo(dir: &Path, root: &Path) -> BTreeMap<String, String> {
    fn walk(dir: &Path, root: &Path, out: &mut BTreeMap<String, String>) {
        for entry in std::fs::read_dir(dir).unwrap() {
            let p = entry.unwrap().path();
            if p.is_dir() {
                walk(&p, root, out);
            } else if p.extension().is_some_and(|e| e == "mo") {
                let name = p
                    .strip_prefix(root)
                    .unwrap()
                    .to_string_lossy()
                    .replace('\\', "/");
                out.insert(name, std::fs::read_to_string(&p).unwrap());
            }
        }
    }
    let mut out = BTreeMap::new();
    walk(dir, root, &mut out);
    out
}

fn package() -> BTreeMap<String, String> {
    let root = root();
    read_mo(&root.join("src/Modelica/GlobeModeValidation"), &root)
}

fn request(value: serde_json::Value) -> Request {
    serde_json::from_value(value).unwrap()
}

#[test]
fn decay_runs_to_the_stop_time() {
    let response = run(&request(serde_json::json!({
        "files": package(),
        "model": "GlobeModeValidation.Simulation.Decay",
    })));
    assert!(response.ok, "{:?}", response.diagnostics);
    let csv = response.csv.unwrap();
    let last = csv.lines().last().unwrap();
    let x: f64 = last.split(',').nth(1).unwrap().parse().unwrap();
    // der(x) = -x、x(0) = 1 の t = 1
    assert!((x - (-1.0f64).exp()).abs() < 1e-6, "{last}");
    assert_eq!(response.final_values.unwrap()["x"], x);
    for phase in ["load", "check", "compile", "run"] {
        assert!(response.timings_ms.contains_key(phase), "{phase}");
    }
}

#[test]
fn decay_with_each_solver() {
    let mut ws = Workspace::new();
    for (name, text) in package() {
        ws.put_file(&name, text);
    }
    for solver in ["bdf", "dopri45", "auto"] {
        let response = ws.run(&request(serde_json::json!({
            "model": "GlobeModeValidation.Simulation.Decay",
            "settings": { "solver": solver },
            "no_csv": true,
        })));
        assert!(response.ok, "{solver}: {:?}", response.diagnostics);
        assert!(response.csv.is_none());
        let x = response.final_values.unwrap()["x"];
        assert!((x - (-1.0f64).exp()).abs() < 1e-5, "{solver}: {x}");
    }
}

#[test]
fn library_is_read_once_and_files_come_and_go() {
    let mut ws = Workspace::with_library("GlobeModeValidation", &package()).unwrap();
    // モデルがライブラリの中にあれば、ワークスペースのファイルは要らない
    let response = ws.run(&request(serde_json::json!({
        "model": "GlobeModeValidation.Simulation.Decay",
        "no_csv": true,
    })));
    assert!(response.ok, "{:?}", response.diagnostics);
    // ワークスペースのモデルがライブラリのモデルを extends する
    ws.put_file(
        "Faster.mo",
        "model Faster\n  extends GlobeModeValidation.Simulation.Decay(k = 2);\nend Faster;\n",
    );
    let response = ws.run(&request(
        serde_json::json!({ "model": "Faster", "no_csv": true }),
    ));
    assert!(response.ok, "{:?}", response.diagnostics);
    let x = response.final_values.unwrap()["x"];
    // experiment annotation は継がないので、許容誤差は既定の 1e-6
    assert!((x - (-2.0f64).exp()).abs() < 1e-4, "{x}");
    // 除くと、もう見つからない
    assert!(ws.delete_file("Faster.mo"));
    assert!(!ws.delete_file("Faster.mo"));
    let response = ws.run(&request(serde_json::json!({ "model": "Faster" })));
    assert!(!response.ok);
    assert!(response.refused.is_some());
}

#[test]
fn broken_library_is_gm_e1003() {
    let mut files = BTreeMap::new();
    files.insert(
        "Broken.mo".to_string(),
        "model Broken\n  Real x\nend Broken;\n".to_string(),
    );
    let err = Workspace::with_library("broken", &files).unwrap_err();
    assert_eq!(err[0].code, "GM-E1003", "{err:?}");
    let err = Workspace::with_library("empty", &BTreeMap::new()).unwrap_err();
    assert_eq!(err[0].code, "GM-E1003", "{err:?}");
}

#[test]
fn refuses_when_the_check_fails() {
    let mut files = package();
    files.insert(
        "Unbalanced.mo".to_string(),
        "model Unbalanced\n  Real x;\n  Real y;\nequation\n  der(x) = -x;\nend Unbalanced;\n"
            .to_string(),
    );
    let response = run(&request(
        serde_json::json!({ "files": files, "model": "Unbalanced" }),
    ));
    assert!(!response.ok);
    assert!(response.refused.unwrap().contains("force"));
    assert!(response.diagnostics.iter().any(|d| d.is_error()));
}

#[test]
fn solver_registry_is_json() {
    let v: serde_json::Value = serde_json::from_str(&solvers_json()).unwrap();
    let names: Vec<&str> = v
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["name"].as_str().unwrap())
        .collect();
    for n in ["bdf", "dopri45", "auto", "dassl", "radau5"] {
        assert!(names.contains(&n), "{n}: {names:?}");
    }
}

/// MSL の ChuaCircuit で、ライブラリを in-memory の組で読む道と、文書として渡す道の時間を比べる。
/// MSL（`cargo xtask fetch-msl`）が要るので既定では回さない:
/// `cargo test --release -p globe-mode-wasm --test decay -- --ignored --nocapture`
#[test]
#[ignore = "MSL が要る（data/msl）。時間の比較"]
fn msl_library_in_memory_vs_documents() {
    let root = root();
    let msl = root.join("data/msl");
    assert!(msl.is_dir(), "data/msl が無い（cargo xtask fetch-msl）");
    let model = "Modelica.Electrical.Analog.Examples.ChuaCircuit";
    let t = std::time::Instant::now();
    let files = read_mo(&msl, &root);
    eprintln!(
        "read: {} files, {:.0} ms",
        files.len(),
        t.elapsed().as_secs_f64() * 1e3
    );

    let t = std::time::Instant::now();
    let mut ws = Workspace::with_library("MSL", &files).unwrap();
    let load = t.elapsed().as_secs_f64();
    let first = ws.run(&request(
        serde_json::json!({ "model": model, "no_csv": true }),
    ));
    assert!(first.ok, "{:?}", first.diagnostics);
    let second = ws.run(&request(
        serde_json::json!({ "model": model, "no_csv": true }),
    ));
    eprintln!(
        "in-memory: library {load:.2} s, 1st {:?}, 2nd {:?}",
        first.timings_ms, second.timings_ms
    );

    let t = std::time::Instant::now();
    let mut ws = Workspace::new();
    for (name, text) in &files {
        ws.put_file(name, text.clone());
    }
    let load = t.elapsed().as_secs_f64();
    let docs = ws.run(&request(
        serde_json::json!({ "model": model, "no_csv": true }),
    ));
    assert!(docs.ok, "{:?}", docs.diagnostics);
    let docs2 = ws.run(&request(
        serde_json::json!({ "model": model, "no_csv": true }),
    ));
    eprintln!(
        "documents: set_file {load:.2} s, 1st {:?}, 2nd {:?}",
        docs.timings_ms, docs2.timings_ms
    );
    let a = first.final_values.unwrap();
    let b = docs.final_values.unwrap();
    assert_eq!(a.len(), b.len());
    for (k, v) in &a {
        assert_eq!(v.to_bits(), b[k].to_bits(), "{k}");
    }
}

#[test]
fn check_returns_diagnostics_without_running() {
    let mut ws = Workspace::new();
    ws.put_file(
        "Bad.mo",
        "model Bad Real x; equation der(x) = -x; x = 1; end Bad;",
    );
    let v: serde_json::Value = serde_json::from_str(&ws.check("Bad").unwrap()).unwrap();
    assert_eq!(v["passed"], false);
    assert!(!v["diagnostics"].as_array().unwrap().is_empty(), "{v}");
}

#[test]
fn graph_of_the_rc_circuit_is_graphology_json() {
    // connect グラフは文書として渡したモデルから作る（ライブラリとして読んだモデルは索引に無い）
    let mut ws = Workspace::new();
    for (name, text) in package() {
        ws.put_file(&name, text);
    }
    // JS から呼ぶ形（失敗の JsValue は wasm32 でしか作れないので、成功だけ）
    let json = ws
        .graph("GlobeModeValidation.Connect.RealCircuit", 1)
        .unwrap();
    let g: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert!(g["nodes"].as_array().unwrap().len() >= 4, "{g}");
    assert!(g["edges"].as_array().unwrap().len() >= 4, "{g}");
    assert!(ws.inner().graph("No.Such.Model", 1).is_err());
}

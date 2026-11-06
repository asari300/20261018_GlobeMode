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
    // DAE にできない Error は強行しても進めない（S173）
    assert!(response.refused.unwrap().contains("強行しても実行できない"));
    assert!(!response.can_force && response.unsupported.is_none());
    assert!(response.diagnostics.iter().any(|d| d.is_error()));
}

/// S173: 初期条件が余る（GM-E8001）は強行で実行に進める。`can_force` が立ち、強行した結果には立たない
#[test]
fn over_determined_initialization_can_be_forced() {
    let over = "model Over\n  Real x(start = 1, fixed = true);\ninitial equation\n  x = 1;\nequation\n  der(x) = -x;\nend Over;\n";
    let files = BTreeMap::from([("Over.mo".to_string(), over.to_string())]);
    let refused = run(&request(
        serde_json::json!({ "files": files, "model": "Over" }),
    ));
    assert!(!refused.ok && refused.can_force && refused.unsupported.is_none());
    assert!(refused.refused.unwrap().contains("force で強行できる"));
    let forced = run(&request(
        serde_json::json!({ "files": files, "model": "Over", "force": true }),
    ));
    assert!(
        forced.final_values.is_some() && !forced.can_force,
        "{:?}",
        forced.diagnostics
    );
}

/// S184: 強行でコンパイルに失敗しても、同じ診断は 1 つだけ（facade で除く。CLI・LSP・Python・UI が同じものを見る）
#[test]
fn forced_failure_does_not_repeat_diagnostics() {
    let src = "model Unbalanced\n  Real x(start = 1);\n  Real y;\nequation\n  der(x) = -x;\nend Unbalanced;\n";
    let files = BTreeMap::from([("U.mo".to_string(), src.to_string())]);
    let r = run(&request(
        serde_json::json!({ "files": files, "model": "Unbalanced", "force": true }),
    ));
    assert_eq!(
        r.diagnostics
            .iter()
            .filter(|d| d.code == "GM-E5002")
            .count(),
        1,
        "{:?}",
        r.diagnostics
    );
    assert!(r.summary.unwrap().contains("。最初の error: GM-E5002 "));
}

/// S173: Error がすべて GM-U なら `unsupported` に要約が付く（誤りとは分ける）。強行しても進めない
#[test]
fn unsupported_errors_are_summarised() {
    let src = "model Div\n  parameter Real p = 1 / 0;\n  Real x(start = 1, fixed = true);\nequation\n  der(x) = -x;\nend Div;\n";
    let files = BTreeMap::from([("Div.mo".to_string(), src.to_string())]);
    let r = run(&request(
        serde_json::json!({ "files": files, "model": "Div" }),
    ));
    assert!(
        r.diagnostics.iter().any(|d| d.is_unsupported()),
        "{:?}",
        r.diagnostics
    );
    assert!(r.unsupported.is_some() && !r.can_force);
    assert!(r.refused.unwrap().contains("強行しても実行できない"));
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
    assert!(
        v.get("can_force").is_none() && v.get("unsupported").is_none(),
        "{v}"
    ); // S173: 無いときは出ない
}

/// S173: 検査の応答にも `can_force`（GM-E8001）と `unsupported`（GM-U）が付く
#[test]
fn check_marks_forceable_and_unsupported() {
    let mut ws = Workspace::new();
    ws.put_file("Over.mo", "model Over Real x(start=1, fixed=true); initial equation x = 1; equation der(x) = -x; end Over;");
    ws.put_file("Div.mo", "model Div parameter Real p = 1 / 0; Real x(start=1, fixed=true); equation der(x) = -x; end Div;");
    let over: serde_json::Value = serde_json::from_str(&ws.check("Over").unwrap()).unwrap();
    assert_eq!(over["can_force"], true, "{over}");
    let div: serde_json::Value = serde_json::from_str(&ws.check("Div").unwrap()).unwrap();
    assert!(
        div["unsupported"].is_string() && div.get("can_force").is_none(),
        "{div}"
    );
}

#[test]
fn graph_of_the_rc_circuit_is_graphology_json() {
    // connect グラフは文書として渡したモデルから作る（ライブラリとして読んだモデルも S079 から引ける。facade の tests/graph_library.rs）
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

/// S191: 読み取りの口（Python の `outline` / `deps` / `flat` と同じ形）。JsValue の誤りは wasm32 でしか作れないので成功だけ
#[test]
fn read_outline_deps_flat_and_check_summary() {
    let mut ws = Workspace::new();
    ws.put_file(
        "A.mo",
        "model A Real x(start = 1, fixed = true); equation der(x) = -x; end A;",
    );
    ws.put_file("Uses.mo", "model Uses A a; end Uses;");
    ws.put_file("Other.mo", "model Other Real y; end Other;");
    let json = |s: String| serde_json::from_str::<serde_json::Value>(&s).unwrap();
    let o = json(ws.outline("A.mo").unwrap());
    assert_eq!(o["classes"][0]["name"], "A");
    assert_eq!(
        json(ws.deps("Uses").unwrap()),
        serde_json::json!(["A.mo", "Uses.mo"])
    );
    assert_eq!(json(ws.deps("NoSuch").unwrap()), serde_json::json!([]));
    let f = json(ws.flat("A").unwrap());
    assert_eq!(
        (f["variables"].as_u64(), f["equations"].as_u64()),
        (Some(1), Some(1)),
        "{f}"
    );
    ws.put_file("Bad.mo", "model Bad Real x; equation y = 1; end Bad;");
    assert!(json(ws.flat("Bad").unwrap()).is_null());
    assert_eq!(
        json(ws.check("A").unwrap())["summary"],
        "A: OK（S0〜S10 を通過）"
    );
}

#[test]
fn columns_match_the_csv() {
    let req = || {
        request(serde_json::json!({
            "files": package(),
            "model": "GlobeModeValidation.Simulation.Decay",
        }))
    };
    let csv = Workspace::new().run(&req()).csv.unwrap();
    let columns = Workspace::new().run_columns(req());
    let response: serde_json::Value = serde_json::from_str(&columns.response()).unwrap();
    assert!(response["ok"].as_bool().unwrap() && response["csv"].is_null());
    let layout: globe_mode::ColumnsLayout = serde_json::from_str(&columns.layout()).unwrap();
    assert_eq!(layout.names, ["time", "x"]);
    assert_eq!(layout.points, csv.lines().count() - 1);
    assert_eq!(layout.defaults, ["x"]); // S190: 既定で描く信号（状態）
    for (i, _) in layout.names.iter().enumerate() {
        let col = columns.column(i);
        assert_eq!(col.len(), layout.points);
        for (row, line) in csv.lines().skip(1).enumerate() {
            let v: f64 = line.split(',').nth(i).unwrap().parse().unwrap();
            assert_eq!(col[row], v);
        }
    }
    assert!(columns.column(99).is_empty());
}

#[test]
fn observed_columns_report_progress_and_match() {
    // S133（ADR-0033 の 2 段目）: 進捗を受けても列は同じ。最後の進捗は積分の t = 1
    let req = || {
        request(serde_json::json!({
            "files": package(),
            "model": "GlobeModeValidation.Simulation.Decay",
            "settings": {"solver": "esdirk34"},
        }))
    };
    let plain = Workspace::new().run_columns(req());
    let mut seen = Vec::new();
    let mut notify = |p: &globe_mode::Progress| seen.push((p.stage, p.time));
    let handle = globe_mode::SimulateHandle {
        on_progress: Some(&mut notify),
        cancel: None,
    };
    let observed = Workspace::new().run_columns_observed(req(), handle);
    assert_eq!(
        seen.first().map(|s| s.0),
        Some(globe_mode::ExecStage::Lowering)
    );
    assert_eq!(
        seen.last(),
        Some(&(globe_mode::ExecStage::Integration, Some(1.0)))
    );
    for i in 0..2 {
        assert_eq!(plain.column(i), observed.column(i));
    }
}

#[test]
fn library_tree_opens_a_model_file_as_the_document() {
    // S127: UI のライブラリツリー。木で選んだモデルのファイルを同じ名前の文書として開き、編集すると検査・実行に効く
    let mut ws = Workspace::with_library_split("GlobeModeValidation", &package())
        .unwrap()
        .with_tree(package());
    let top: serde_json::Value = serde_json::from_str(&ws.tree_children("")).unwrap();
    assert_eq!(top[0]["path"], "GlobeModeValidation", "{top}");
    let sim: serde_json::Value =
        serde_json::from_str(&ws.tree_children("GlobeModeValidation")).unwrap();
    let sim = sim.as_array().unwrap();
    assert!(
        sim.iter()
            .any(|e| e["name"] == "Simulation" && e["hasChildren"] == true),
        "{sim:?}"
    );
    // S167: 入れ子のクラスも木の子で、検索で見つかり、開くのは定義するファイル
    let nested: serde_json::Value =
        serde_json::from_str(&ws.tree_children("GlobeModeValidation.Components")).unwrap();
    assert!(
        nested
            .as_array()
            .unwrap()
            .iter()
            .any(|e| e["name"] == "Resistor"),
        "{nested}"
    );
    let found: serde_json::Value = serde_json::from_str(&ws.tree_search("RESIS", 5)).unwrap();
    assert_eq!(
        found[0]["path"], "GlobeModeValidation.Components.Resistor",
        "{found}"
    );
    assert!(
        ws.tree_source("GlobeModeValidation.Components.Resistor")
            .unwrap()
            .1
            .contains("model Resistor")
    );
    let model = "GlobeModeValidation.Simulation.Decay";
    let (file, text) = ws.tree_source(model).unwrap();
    let (file, text) = (file.to_string(), text.replace("k = 1", "k = 2"));
    assert!(file.ends_with("Simulation/Decay.mo"), "{file}");
    ws.put_file(&file, text);
    assert!(ws.inner().check(model).passed);
    let response = ws.run(&request(
        serde_json::json!({ "model": model, "no_csv": true }),
    ));
    assert!(response.ok, "{:?}", response.diagnostics);
    let x = response.final_values.unwrap()["x"];
    assert!((x - (-2.0f64).exp()).abs() < 1e-4, "{x}");
}

#[test]
fn definition_jumps_into_the_library_file() {
    // S138: UI の F12。ライブラリの木から読んだファイルを引く
    let mut ws = Workspace::with_library_split("GlobeModeValidation", &package())
        .unwrap()
        .with_tree(package());
    let text =
        "model T\n  import GlobeModeValidation.Components;\n  Components.Resistor r1;\nend T;\n";
    ws.put_file("T.mo", text);
    let r: serde_json::Value = serde_json::from_str(&ws.definition("T.mo", text, 2, 15)).unwrap();
    assert!(
        r["file"].as_str().unwrap().ends_with("Components.mo"),
        "{r}"
    );
    assert!(r["text"].as_str().unwrap().contains("Resistor"), "{r}");
    assert_eq!(r["range"]["start"]["line"], 19, "{r}");
    assert_eq!(ws.definition("T.mo", text, 0, 0), "null");
    // S149: ホバーは同じ解決で宣言の 1 行（クラスの説明文字列つき）
    let h: serde_json::Value = serde_json::from_str(&ws.hover("T.mo", text, 2, 15)).unwrap();
    assert!(
        h["contents"]
            .as_str()
            .unwrap()
            .starts_with("model Resistor"),
        "{h}"
    );
    assert_eq!(h["range"]["start"]["line"], 2, "{h}");
    assert_eq!(ws.hover("T.mo", text, 0, 0), "null");
}

#[test]
fn references_and_rename_in_the_same_class() {
    // S161: UI の Shift+F12 と F2。rename は断ると理由つきの例外（ネイティブ以外では JsValue なので、成功側だけ確かめる）
    let mut ws = Workspace::new();
    let text = "model T\n  Real x;\nequation\n  der(x) = -x;\nend T;\n";
    ws.put_file("T.mo", text);
    let refs: serde_json::Value = serde_json::from_str(&ws.references("T.mo", text, 3, 6)).unwrap();
    assert_eq!(refs.as_array().unwrap().len(), 3, "{refs}");
    let edits: serde_json::Value =
        serde_json::from_str(&ws.rename("T.mo", text, 1, 7, "pos").unwrap()).unwrap();
    assert_eq!(edits.as_array().unwrap().len(), 3, "{edits}");
    assert_eq!(edits[0]["newText"], "pos");
}

#[test]
fn decay_exports_a_source_fmu_as_bytes() {
    // S144: ブラウザの「FMU を書き出す」。ファイルに触らず zip のバイト列で返る
    let mut ws = Workspace::new();
    for (name, text) in package() {
        ws.put_file(&name, text);
    }
    let fmu = ws
        .export_fmu("GlobeModeValidation.Simulation.Decay", 2)
        .ok()
        .unwrap();
    assert_eq!(&fmu[..2], b"PK");
    let has = |name: &[u8]| fmu.windows(name.len()).any(|w| w == name);
    assert!(has(b"modelDescription.xml") && has(b"sources/"));
    assert!(ws.inner().export_fmu_bytes("Nope", 2).is_err()); // export_fmu の Err は JsValue で、ネイティブでは作れない
}

#[test]
fn decay_lists_tunable_parameters() {
    // S150: UI の実行の設定の一覧。Decay の k が既定値 1 で出る
    let mut ws = Workspace::new();
    for (name, text) in package() {
        ws.put_file(&name, text);
    }
    let r = ws
        .inner()
        .tunable_parameters("GlobeModeValidation.Simulation.Decay");
    let r = r.ok().unwrap();
    let k = r
        .parameters
        .unwrap()
        .into_iter()
        .find(|p| p.name == "k")
        .unwrap();
    assert_eq!(k.default, 1.0);
    assert!(ws.inner().tunable_parameters("Nope").is_err());
}

#[test]
fn models_lists_the_runnable_classes_of_a_file() {
    // S170: UI のモデル名の候補。model・block・class の修飾名（partial は除く。package の中まで入る）、壊れた構文は空
    let mut ws = Workspace::new();
    let text = "model Helper\n  Real x;\nend Helper;\npartial model P\nend P;\nmodel Main\n  Helper h;\nend Main;\npackage Pk\n  block B\n  end B;\n  function f\n  end f;\nend Pk;\n";
    ws.put_file("T.mo", text);
    assert_eq!(ws.models("T.mo"), r#"["Helper","Main","Pk.B"]"#);
    ws.put_file("Bad.mo", "model Bad\n  Real x\nend Bad;\n");
    assert_eq!(ws.models("Bad.mo"), "[]");
    assert_eq!(ws.models("Nope.mo"), "[]");
}

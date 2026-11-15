//! FMI Import のマスター（S249、ADR-0026 の (q)）の試験。Reference-FMUs（無ければ飛ばす）の Dahlquist（2.0）→ Feedthrough（3.0）。
//! Jacobi 型なので、b の出力は a の出力の 1 刻み遅れに**ちょうど**等しい（手で確かめられる）。

use std::path::PathBuf;
use std::process::Command;

fn reference_fmu(version: &str, name: &str) -> Option<PathBuf> {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../../data/reference-fmus")
        .join(version)
        .join(format!("{name}.fmu"));
    if p.is_file() {
        Some(p)
    } else {
        assert!(
            std::env::var_os("GLOBE_REQUIRE_REFERENCE_FMUS").is_none(),
            "{p:?} が無い"
        );
        eprintln!("skip: {p:?} が無い（cargo xtask fetch-reference-fmus）");
        None
    }
}

fn scratch(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("globe-cosim-{}-{tag}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn cosim(dir: &std::path::Path, spec: serde_json::Value) -> std::process::Output {
    let file = dir.join("spec.json");
    std::fs::write(&file, spec.to_string()).unwrap();
    Command::new(env!("CARGO_BIN_EXE_globe"))
        .args(["fmu", "cosim"])
        .arg(&file)
        .output()
        .unwrap()
}

#[test]
fn feedthrough_is_one_step_behind_dahlquist() {
    let (Some(a), Some(b)) = (
        reference_fmu("2.0", "Dahlquist"),
        reference_fmu("3.0", "Feedthrough"),
    ) else {
        return;
    };
    let dir = scratch("1");
    let out = cosim(
        &dir,
        serde_json::json!({
            "fmus": {"a": a, "b": b},
            "connections": [["a.x", "b.Float64_continuous_input"]],
            "stop_time": 1.0, "step": 0.1,
        }),
    );
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let text = String::from_utf8(out.stdout).unwrap();
    let mut lines = text.lines();
    let header: Vec<&str> = lines.next().unwrap().split(',').collect();
    let col = |n: &str| {
        header
            .iter()
            .position(|h| *h == n)
            .unwrap_or_else(|| panic!("{n} in {header:?}"))
    };
    let (x, y) = (col("a.x"), col("b.Float64_continuous_output"));
    let rows: Vec<Vec<f64>> = lines
        .map(|l| l.split(',').map(|c| c.parse().unwrap()).collect())
        .collect();
    assert_eq!(rows.len(), 11);
    assert_eq!(rows[0][x], 1.0); // Dahlquist の x(0) = 1
    assert!(rows[10][x] < 1.0 && rows[10][x] > 0.0, "x が減衰している");
    for k in 1..rows.len() {
        assert_eq!(
            rows[k][y],
            rows[k - 1][x],
            "行 {k}: b の出力 = 1 刻み前の a.x"
        );
    }
}

#[test]
fn bad_wires_are_refused_before_running() {
    let Some(a) = reference_fmu("2.0", "Dahlquist") else {
        return;
    };
    let dir = scratch("2");
    let out = cosim(
        &dir,
        serde_json::json!({
            "fmus": {"a": a}, "connections": [["a.x", "a.k"]], "stop_time": 1.0, "step": 0.1,
        }),
    );
    assert!(!out.status.success());
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(err.contains("入力でない"), "{err}");
}

/// S253: `.ssp`（zip）を試験の中で組み立てる。`ssd` は `SystemStructure.ssd` の本文、`fmus` は `resources/<名前>.fmu` に入れる。
fn ssp(
    dir: &std::path::Path,
    ssd: &str,
    fmus: &[(&str, &PathBuf)],
    extra: &[(&str, &str)],
) -> PathBuf {
    use std::io::Write;
    let path = dir.join("sys.ssp");
    let mut zip = zip::ZipWriter::new(std::fs::File::create(&path).unwrap());
    let opt = zip::write::SimpleFileOptions::default();
    zip.start_file("SystemStructure.ssd", opt).unwrap();
    zip.write_all(ssd.as_bytes()).unwrap();
    for (name, fmu) in fmus {
        zip.start_file(format!("resources/{name}.fmu"), opt)
            .unwrap();
        zip.write_all(&std::fs::read(fmu).unwrap()).unwrap();
    }
    for (name, body) in extra {
        zip.start_file(*name, opt).unwrap();
        zip.write_all(body.as_bytes()).unwrap();
    }
    zip.finish().unwrap();
    path
}

const SSD: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<ssd:SystemStructureDescription xmlns:ssd="http://ssp-standard.org/SSP1/SystemStructureDescription"
  xmlns:ssc="http://ssp-standard.org/SSP1/SystemStructureCommon" name="demo" version="2.0">
  <ssd:System name="top">
    <ssd:Elements>
      EXTRA
      <ssd:Component name="a" source="resources/a.fmu"/>
      <ssd:Component name="b" source="resources/b.fmu"/>
    </ssd:Elements>
    <ssd:Connections>
      <!-- start / end はデータの向きではない: 入力側を start に書いても通る -->
      <ssd:Connection startElement="b" startConnector="Float64_continuous_input" endElement="a" endConnector="x"/>
    </ssd:Connections>
  </ssd:System>
  <ssd:DefaultExperiment startTime="0" stopTime="1"/>
</ssd:SystemStructureDescription>"#;

#[test]
fn ssp_gives_the_same_csv_as_the_json_spec() {
    let (Some(a), Some(b)) = (
        reference_fmu("2.0", "Dahlquist"),
        reference_fmu("3.0", "Feedthrough"),
    ) else {
        return;
    };
    let dir = scratch("3");
    let json = cosim(
        &dir,
        serde_json::json!({
            "fmus": {"a": a, "b": b},
            "connections": [["a.x", "b.Float64_continuous_input"]],
            "start_time": 0.0, "stop_time": 1.0, "step": 0.1,
        }),
    );
    assert!(json.status.success());
    let file = ssp(
        &dir,
        &SSD.replace("EXTRA", ""),
        &[("a", &a), ("b", &b)],
        &[],
    );
    let out = Command::new(env!("CARGO_BIN_EXE_globe"))
        .args(["fmu", "cosim"])
        .arg(&file)
        .args(["--step", "0.1"])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(out.stdout, json.stdout);
}

/// S295: 最上位のシステム自身のコネクタへの接続（`startElement` なし）は今も明確な誤り。
#[test]
fn ssp_refuses_top_level_system_connector() {
    let (Some(a), Some(b)) = (
        reference_fmu("2.0", "Dahlquist"),
        reference_fmu("3.0", "Feedthrough"),
    ) else {
        return;
    };
    let dir = scratch("4");
    let ssd = SSD.replace("EXTRA", "").replace("startElement=\"b\" ", "");
    let file = ssp(&dir, &ssd, &[("a", &a), ("b", &b)], &[]);
    let out = run_ssp(&file);
    assert!(!out.status.success());
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(err.contains("システム自身のコネクタ"), "{err}");
}

const NESTED_HEAD: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<ssd:SystemStructureDescription xmlns:ssd="http://ssp-standard.org/SSP1/SystemStructureDescription"
  xmlns:ssc="http://ssp-standard.org/SSP1/SystemStructureCommon"
  xmlns:ssv="http://ssp-standard.org/SSP1/SystemStructureParameterValues" name="demo" version="2.0">
  <ssd:System name="top">"#;
const NESTED_TAIL: &str = r#"</ssd:System>
  <ssd:DefaultExperiment startTime="0" stopTime="1"/>
</ssd:SystemStructureDescription>"#;

/// 平らな Dahlquist（a）→ Feedthrough（b）の CSV と、入れ子の `ssd` の CSV を比べる。
/// `rename` は（平らな列の名前 → 入れ子での列の名前）。
fn nested_equals_flat(tag: &str, ssd: &str, rename: &dyn Fn(&str) -> String) {
    let (Some(a), Some(b)) = (
        reference_fmu("2.0", "Dahlquist"),
        reference_fmu("3.0", "Feedthrough"),
    ) else {
        return;
    };
    let dir = scratch(tag);
    let flat = table(&cosim(
        &dir,
        serde_json::json!({
            "fmus": {"a": a, "b": b},
            "connections": [["a.x", "b.Float64_continuous_input"]],
            "start_time": 0.0, "stop_time": 1.0, "step": 0.1,
        }),
    ));
    let nested = table(&run_ssp(&ssp(&dir, ssd, &[("a", &a), ("b", &b)], &[])));
    assert_eq!(nested.len(), flat.len());
    for (col, v) in &flat {
        let key = if col == "time" {
            col.clone()
        } else {
            rename(col)
        };
        assert_eq!(nested[&key], *v, "{col}");
    }
    // 結線が効いている（b の出力は a の x の 1 刻み遅れで 0 のままではない）
    assert!(flat["b.Float64_continuous_output"][5] > 0.0);
}

/// S295: 入れ子の System（`s` の中に a・b）は `s.a`・`s.b` に平らになり、平らな版と同じ値になる。
#[test]
fn ssp_nested_system_gives_the_same_csv_as_flat() {
    let ssd = format!(
        r#"{NESTED_HEAD}<ssd:Elements><ssd:System name="s">
    <ssd:Elements>
      <ssd:Component name="a" source="resources/a.fmu"/>
      <ssd:Component name="b" source="resources/b.fmu"/>
    </ssd:Elements>
    <ssd:Connections>
      <ssd:Connection startElement="b" startConnector="Float64_continuous_input" endElement="a" endConnector="x"/>
    </ssd:Connections>
  </ssd:System></ssd:Elements>{NESTED_TAIL}"#
    );
    nested_equals_flat("10", &ssd, &|c| format!("s.{c}"));
}

/// S295: System のコネクタを通る接続（a.x → g の入力 `i` → g の中の b）は 1 本の結線にたどられ、平らな版と同じ値になる。
#[test]
fn ssp_connection_through_system_connectors_gives_the_same_csv_as_flat() {
    let ssd = format!(
        r#"{NESTED_HEAD}<ssd:Elements>
    <ssd:Component name="a" source="resources/a.fmu"/>
    <ssd:System name="g">
      <ssd:Connectors><ssd:Connector name="i" kind="input"><ssc:Real/></ssd:Connector></ssd:Connectors>
      <ssd:Elements><ssd:Component name="b" source="resources/b.fmu"/></ssd:Elements>
      <ssd:Connections>
        <ssd:Connection startConnector="i" endElement="b" endConnector="Float64_continuous_input"/>
      </ssd:Connections>
    </ssd:System>
  </ssd:Elements>
  <ssd:Connections>
    <ssd:Connection startElement="a" startConnector="x" endElement="g" endConnector="i"/>
  </ssd:Connections>{NESTED_TAIL}"#
    );
    nested_equals_flat("11", &ssd, &|c| {
        if c.starts_with("b.") {
            format!("g.{c}")
        } else {
            c.to_owned()
        }
    });
    // 入力 `i` が内側でどこにもつながらないと、黙って捨てず断る
    let (Some(a), Some(b)) = (
        reference_fmu("2.0", "Dahlquist"),
        reference_fmu("3.0", "Feedthrough"),
    ) else {
        return;
    };
    let dangling = ssd.replace(
        r#"<ssd:Connection startConnector="i" endElement="b" endConnector="Float64_continuous_input"/>"#,
        "",
    );
    let out = run_ssp(&ssp(
        &scratch("11"),
        &dangling,
        &[("a", &a), ("b", &b)],
        &[],
    ));
    assert!(!out.status.success());
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(err.contains("1 つで終わっている"), "{err}");
}

/// S295: System の `ParameterBindings` は下の Component に効き、階層が違えば上位が勝つ（5.2.3）。
/// Dahlquist `g.a` の k: Component の binding が 3、`g` の binding（名前 `a.k`）が 4、最上位の binding（名前 `g.a.k`）が 2。
#[test]
fn ssp_system_parameter_bindings_follow_the_hierarchy() {
    let Some(a) = reference_fmu("2.0", "Dahlquist") else {
        return;
    };
    let dir = scratch("12");
    let binding = |name: &str, k: u8| {
        format!(
            r#"<ssd:ParameterBindings><ssd:ParameterBinding><ssd:ParameterValues>
          <ssv:ParameterSet version="2.0" name="p"><ssv:Parameters>
            <ssv:Parameter name="{name}"><ssv:Real value="{k}"/></ssv:Parameter>
          </ssv:Parameters></ssv:ParameterSet></ssd:ParameterValues></ssd:ParameterBinding></ssd:ParameterBindings>"#
        )
    };
    let ssd = |root: &str, group: &str, comp: &str| {
        format!(
            r#"{NESTED_HEAD}{root}<ssd:Elements><ssd:System name="g">{group}<ssd:Elements>
      <ssd:Component name="a" source="resources/a.fmu">{comp}</ssd:Component>
    </ssd:Elements></ssd:System></ssd:Elements>{NESTED_TAIL}"#
        )
    };
    let (root, group, comp) = (binding("g.a.k", 2), binding("a.k", 4), binding("k", 3));
    for (root, group, comp, k) in [
        (root.as_str(), group.as_str(), comp.as_str(), 2),
        ("", group.as_str(), comp.as_str(), 4),
        ("", "", comp.as_str(), 3),
    ] {
        let x = table(&run_ssp(&ssp(
            &dir,
            &ssd(root, group, comp),
            &[("a", &a)],
            &[],
        )))["g.a.x"][10];
        let want = (1.0 - 0.1 * f64::from(k)).powi(10);
        assert!((x - want).abs() < 1e-9, "k = {k}: x(1) = {x}");
    }
}

/// S263: Dahlquist 1 つの SSP。`binding` は `<ssd:Component name="a">` の子（ParameterBindings）。
fn dahlquist_ssd(binding: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<ssd:SystemStructureDescription xmlns:ssd="http://ssp-standard.org/SSP1/SystemStructureDescription"
  xmlns:ssv="http://ssp-standard.org/SSP1/SystemStructureParameterValues" name="demo" version="2.0">
  <ssd:System name="top"><ssd:Elements>
    <ssd:Component name="a" source="resources/a.fmu">{binding}</ssd:Component>
  </ssd:Elements></ssd:System>
  <ssd:DefaultExperiment startTime="0" stopTime="1"/>
</ssd:SystemStructureDescription>"#
    )
}

const SSV: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<ssv:ParameterSet xmlns:ssv="http://ssp-standard.org/SSP1/SystemStructureParameterValues" version="2.0" name="p">
  <ssv:Parameters>
    <ssv:Parameter name="k"><ssv:Real value="2"/></ssv:Parameter>
    <ssv:Parameter name="not_in_the_fmu"><ssv:Integer value="7"/></ssv:Parameter>
  </ssv:Parameters>
</ssv:ParameterSet>"#;

fn run_ssp(file: &std::path::Path) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_globe"))
        .args(["fmu", "cosim"])
        .arg(file)
        .args(["--step", "0.1"])
        .output()
        .unwrap()
}

/// S263: `.ssv` の参照とインラインの `ParameterSet` は `start` になる（k = 2 なら x(1) は 0.8^10。FMU に無い名前は無視）。
#[test]
fn ssp_parameter_bindings_set_start_values() {
    let Some(a) = reference_fmu("2.0", "Dahlquist") else {
        return;
    };
    let dir = scratch("7");
    let by_file = dahlquist_ssd(
        r#"<ssd:ParameterBindings><ssd:ParameterBinding source="resources/p.ssv"/></ssd:ParameterBindings>"#,
    );
    let inline = dahlquist_ssd(
        r#"<ssd:ParameterBindings><ssd:ParameterBinding>
          <ssd:ParameterValues><ssv:ParameterSet version="2.0" name="i"><ssv:Parameters>
            <ssv:Parameter name="k"><ssv:Real value="1"/></ssv:Parameter>
          </ssv:Parameters></ssv:ParameterSet></ssd:ParameterValues>
        </ssd:ParameterBinding>
        <ssd:ParameterBinding><ssd:ParameterValues><ssv:ParameterSet version="2.0" name="j"><ssv:Parameters>
            <ssv:Parameter name="k"><ssv:Real value="2"/></ssv:Parameter>
          </ssv:Parameters></ssv:ParameterSet></ssd:ParameterValues></ssd:ParameterBinding></ssd:ParameterBindings>"#,
    );
    // 後ろの binding が勝つ（5.2.3）ので、インラインの 2 本は k = 2
    for (ssd, extra) in [(by_file, vec![("resources/p.ssv", SSV)]), (inline, vec![])] {
        let out = run_ssp(&ssp(&dir, &ssd, &[("a", &a)], &extra));
        let x = table(&out)["a.x"][10];
        assert!((x - 0.8f64.powi(10)).abs() < 1e-9, "x(1) = {x}");
    }
}

/// S263: `ParameterMapping`（.ssm）・System の階層の ParameterBindings は明確な誤り。
#[test]
fn ssp_refuses_parameter_mapping() {
    let Some(a) = reference_fmu("2.0", "Dahlquist") else {
        return;
    };
    let dir = scratch("8");
    let ssd = dahlquist_ssd(
        r#"<ssd:ParameterBindings><ssd:ParameterBinding source="resources/p.ssv">
          <ssd:ParameterMapping source="resources/m.ssm"/></ssd:ParameterBinding></ssd:ParameterBindings>"#,
    );
    let out = run_ssp(&ssp(&dir, &ssd, &[("a", &a)], &[("resources/p.ssv", SSV)]));
    assert!(!out.status.success());
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(err.contains("ParameterMapping"), "{err}");
}

/// S288: `.ssp` にも `--timeout`・`--memory-limit` が効く（不正な上限は実行の前に断る。正しい上限なら結果は変わらない）。
#[test]
fn ssp_honours_timeout_and_memory_limit_flags() {
    let Some(a) = reference_fmu("2.0", "Dahlquist") else {
        return;
    };
    let dir = scratch("9");
    let file = ssp(&dir, &dahlquist_ssd(""), &[("a", &a)], &[]);
    let run = |extra: &[&str]| {
        Command::new(env!("CARGO_BIN_EXE_globe"))
            .args(["fmu", "cosim"])
            .arg(&file)
            .args(["--step", "0.1"])
            .args(extra)
            .output()
            .unwrap()
    };
    let bad = run(&["--timeout=-1"]);
    let err = String::from_utf8_lossy(&bad.stderr);
    assert!(
        !bad.status.success() && err.contains("timeout") && !err.contains("unexpected"),
        "{err}"
    );
    let plain = run(&[]);
    let limited = run(&["--timeout", "60", "--memory-limit", "4096"]);
    assert_eq!(table(&limited), table(&plain));
}

/// 列の名前 → 全行の値。
fn table(out: &std::process::Output) -> std::collections::BTreeMap<String, Vec<f64>> {
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let text = String::from_utf8(out.stdout.clone()).unwrap();
    let mut lines = text.lines();
    let header: Vec<String> = lines
        .next()
        .unwrap()
        .split(',')
        .map(str::to_string)
        .collect();
    let mut cols = vec![Vec::new(); header.len()];
    for l in lines {
        for (c, v) in cols.iter_mut().zip(l.split(',')) {
            c.push(v.parse().unwrap());
        }
    }
    header.into_iter().zip(cols).collect()
}

/// S259: Gauss–Seidel は FMU を結線の順（元 → 先。名前の順ではない）に進め、同じ刻みの出力をすぐ渡す。
/// Jacobi（上の試験）の 1 刻み遅れが 0 になる。
#[test]
fn gauss_seidel_has_no_delay() {
    let (Some(a), Some(b)) = (
        reference_fmu("2.0", "Dahlquist"),
        reference_fmu("3.0", "Feedthrough"),
    ) else {
        return;
    };
    let dir = scratch("5");
    let t = table(&cosim(
        &dir,
        serde_json::json!({
            "fmus": {"z": a, "b": b}, "algorithm": "gauss-seidel",
            "connections": [["z.x", "b.Float64_continuous_input"]],
            "stop_time": 1.0, "step": 0.1,
        }),
    ));
    let (x, y) = (&t["z.x"], &t["b.Float64_continuous_output"]);
    assert_eq!(x.len(), 11);
    assert!(x[10] < x[0]);
    for k in 1..x.len() {
        assert_eq!(y[k], x[k], "行 {k}: b の出力 = 同じ刻みの z.x");
    }
}

/// S259: `start` は初期化の前に効く（Dahlquist の k = 2 なら x(1) は 0.8^10）。
#[test]
fn start_sets_parameters_before_initialization() {
    let Some(a) = reference_fmu("2.0", "Dahlquist") else {
        return;
    };
    let dir = scratch("6");
    let t = table(&cosim(
        &dir,
        serde_json::json!({
            "fmus": {"a": a}, "start": {"a.k": 2.0}, "connections": [],
            "stop_time": 1.0, "step": 0.1,
        }),
    ));
    let x = t["a.x"][10];
    // Reference-FMUs の Dahlquist は通信刻みを Euler で 1 回: k = 2、h = 0.1 なら x(1) = 0.8^10（k = 1 なら 0.9^10）
    assert!((x - 0.8f64.powi(10)).abs() < 1e-9, "x(1) = {x}");
}

/// S259: Int32・Boolean の接続（2.0 と 3.0 の Feedthrough を入出力でたすき掛け）。Jacobi なので値は 1 刻みごとに入れ替わる。
#[test]
fn integer_and_boolean_wires() {
    let (Some(a), Some(b)) = (
        reference_fmu("2.0", "Feedthrough"),
        reference_fmu("3.0", "Feedthrough"),
    ) else {
        return;
    };
    let dir = scratch("7");
    let spec = |extra: serde_json::Value| {
        let mut s = serde_json::json!({
            "fmus": {"a": a, "b": b},
            "start": {"a.Int32_input": 7.0, "a.Boolean_input": 1.0},
            "connections": [
                ["a.Int32_output", "b.Int32_input"], ["b.Int32_output", "a.Int32_input"],
                ["a.Boolean_output", "b.Boolean_input"], ["b.Boolean_output", "a.Boolean_input"],
            ],
            "stop_time": 0.4, "step": 0.1,
        });
        s.as_object_mut()
            .unwrap()
            .extend(extra.as_object().unwrap().clone());
        s
    };
    let t = table(&cosim(&dir, spec(serde_json::json!({}))));
    assert_eq!(t["a.Int32_output"], [7.0, 0.0, 7.0, 0.0, 7.0]);
    assert_eq!(t["b.Int32_output"], [0.0, 7.0, 0.0, 7.0, 0.0]);
    assert_eq!(t["a.Boolean_output"], [1.0, 0.0, 1.0, 0.0, 1.0]);
    assert_eq!(t["b.Boolean_output"], [0.0, 1.0, 0.0, 1.0, 0.0]);
    // 型の違う接続と、整数でない Int の start は実行の前に断る
    let mut bad = spec(serde_json::json!({}));
    bad["connections"] = serde_json::json!([["a.Int32_output", "b.Float64_continuous_input"]]);
    let err = String::from_utf8_lossy(&cosim(&dir, bad).stderr).to_string();
    assert!(err.contains("型が違う"), "{err}");
    let err = String::from_utf8_lossy(
        &cosim(
            &dir,
            spec(serde_json::json!({"start": {"a.Int32_input": 1.5}})),
        )
        .stderr,
    )
    .to_string();
    assert!(err.contains("不正"), "{err}");
}

/// S274: spec.json → `ssp-export` → `.ssp` を `cosim` で回した CSV は、JSON を直接回した CSV と同じ。
/// 刻みと algorithm（と start）は SSD の Annotation を通って戻る（`--step` は渡さない）。
#[test]
fn ssp_export_round_trips_to_the_same_csv() {
    let (Some(a), Some(b)) = (
        reference_fmu("2.0", "Dahlquist"),
        reference_fmu("3.0", "Feedthrough"),
    ) else {
        return;
    };
    let dir = scratch("9");
    for (tag, extra) in [
        ("jacobi", serde_json::json!({})),
        (
            "gs-start",
            serde_json::json!({"algorithm": "gauss-seidel", "start": {"a.k": 2.0}}),
        ),
    ] {
        let mut spec = serde_json::json!({
            "fmus": {"a": a, "b": b},
            "connections": [["a.x", "b.Float64_continuous_input"]],
            "stop_time": 1.0, "step": 0.1,
        });
        spec.as_object_mut()
            .unwrap()
            .extend(extra.as_object().unwrap().clone());
        let json = cosim(&dir, spec);
        assert!(json.status.success());
        let ssp_file = dir.join(format!("{tag}.ssp"));
        let export = Command::new(env!("CARGO_BIN_EXE_globe"))
            .args(["fmu", "ssp-export"])
            .arg(dir.join("spec.json"))
            .arg("-o")
            .arg(&ssp_file)
            .output()
            .unwrap();
        assert!(
            export.status.success(),
            "{}",
            String::from_utf8_lossy(&export.stderr)
        );
        let out = Command::new(env!("CARGO_BIN_EXE_globe"))
            .args(["fmu", "cosim"])
            .arg(&ssp_file)
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        assert_eq!(out.stdout, json.stdout, "{tag}");
        if tag == "gs-start" {
            // k = 2 が効いている（x(1) = 0.8^10）
            assert!((table(&out)["a.x"][10] - 0.8f64.powi(10)).abs() < 1e-9);
        }
    }
}

/// S295: Int32・Boolean の接続も `ssp-export` → `cosim` で往復する（2.0 と 3.0 の Feedthrough のたすき掛け。CSV がバイトで一致する）。
#[test]
fn ssp_export_round_trips_integer_and_boolean_wires() {
    let (Some(a), Some(b)) = (
        reference_fmu("2.0", "Feedthrough"),
        reference_fmu("3.0", "Feedthrough"),
    ) else {
        return;
    };
    let dir = scratch("13");
    let json = cosim(
        &dir,
        serde_json::json!({
            "fmus": {"a": a, "b": b},
            "start": {"a.Int32_input": 7.0, "a.Boolean_input": 1.0},
            "connections": [
                ["a.Int32_output", "b.Int32_input"], ["b.Int32_output", "a.Int32_input"],
                ["a.Boolean_output", "b.Boolean_input"], ["b.Boolean_output", "a.Boolean_input"],
            ],
            "stop_time": 0.4, "step": 0.1,
        }),
    );
    let ssp_file = dir.join("int.ssp");
    let export = Command::new(env!("CARGO_BIN_EXE_globe"))
        .args(["fmu", "ssp-export"])
        .arg(dir.join("spec.json"))
        .arg("-o")
        .arg(&ssp_file)
        .output()
        .unwrap();
    assert!(
        export.status.success(),
        "{}",
        String::from_utf8_lossy(&export.stderr)
    );
    let out = Command::new(env!("CARGO_BIN_EXE_globe"))
        .args(["fmu", "cosim"])
        .arg(&ssp_file)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(out.stdout, json.stdout);
    assert_eq!(table(&out)["a.Int32_output"], [7.0, 0.0, 7.0, 0.0, 7.0]);
}

fn globe_ok(args: &[&std::path::Path]) -> std::process::Output {
    let out = Command::new(env!("CARGO_BIN_EXE_globe"))
        .args(["fmu"])
        .args(args)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    out
}

/// S315: spec.json → `ssp-export` → `ssp-import` → spec.json は、パス以外が元と同じで、回した CSV もバイトで一致する。
#[test]
fn ssp_import_inverts_ssp_export() {
    let (Some(a), Some(b)) = (
        reference_fmu("2.0", "Dahlquist"),
        reference_fmu("3.0", "Feedthrough"),
    ) else {
        return;
    };
    let dir = scratch("s315a");
    let spec = serde_json::json!({
        "fmus": {"a": a, "b": b},
        "connections": [["a.x", "b.Float64_continuous_input"]],
        "start_time": 0.0, "stop_time": 1.0, "step": 0.1,
        "algorithm": "gauss-seidel", "start": {"a.k": 2.0},
    });
    let json = cosim(&dir, spec.clone());
    assert!(json.status.success());
    let ssp_file = dir.join("s.ssp");
    globe_ok(&[
        "ssp-export".as_ref(),
        &dir.join("spec.json"),
        "-o".as_ref(),
        &ssp_file,
    ]);
    let back = dir.join("back");
    std::fs::create_dir_all(&back).unwrap();
    globe_ok(&[
        "ssp-import".as_ref(),
        &ssp_file,
        "-o".as_ref(),
        &back.join("spec.json"),
    ]);
    let mut got: serde_json::Value =
        serde_json::from_slice(&std::fs::read(back.join("spec.json")).unwrap()).unwrap();
    assert_eq!(got["fmus"], serde_json::json!({"a": "a.fmu", "b": "b.fmu"}));
    // パスと、SSP に置き場の無い（null の）上限を除けば元と同じ
    let mut want = spec;
    for v in [&mut got, &mut want] {
        let o = v.as_object_mut().unwrap();
        o.remove("fmus");
        o.retain(|_, v| !v.is_null());
    }
    assert_eq!(got, want);
    // FMU は外に出ているので、JSON をそのまま回せる
    let again = globe_ok(&["cosim".as_ref(), &back.join("spec.json")]);
    assert_eq!(again.stdout, json.stdout);
    // 同名のファイルは上書きしない
    let out = Command::new(env!("CARGO_BIN_EXE_globe"))
        .args(["fmu", "ssp-import"])
        .arg(&ssp_file)
        .arg("-o")
        .arg(back.join("spec2.json"))
        .output()
        .unwrap();
    assert!(!out.status.success());
}

/// S315: 入れ子の `.ssp` を読む → 書く → 読むで同じ CSV になる（`s.a`・`s.b` は平らな Component として書かれる）。
#[test]
fn nested_ssp_round_trips_through_flat_dotted_names() {
    let (Some(a), Some(b)) = (
        reference_fmu("2.0", "Dahlquist"),
        reference_fmu("3.0", "Feedthrough"),
    ) else {
        return;
    };
    let dir = scratch("s315b");
    let ssd = format!(
        r#"{NESTED_HEAD}<ssd:Elements><ssd:System name="s">
    <ssd:Elements>
      <ssd:Component name="a" source="resources/a.fmu"/>
      <ssd:Component name="b" source="resources/b.fmu"/>
    </ssd:Elements>
    <ssd:Connections>
      <ssd:Connection startElement="b" startConnector="Float64_continuous_input" endElement="a" endConnector="x"/>
    </ssd:Connections>
  </ssd:System></ssd:Elements>{NESTED_TAIL}"#
    );
    let nested = ssp(&dir, &ssd, &[("a", &a), ("b", &b)], &[]);
    let want = run_ssp(&nested);
    assert!(want.status.success());
    let back = dir.join("back");
    std::fs::create_dir_all(&back).unwrap();
    globe_ok(&[
        "ssp-import".as_ref(),
        &nested,
        "-o".as_ref(),
        &back.join("spec.json"),
        "--step".as_ref(),
        "0.1".as_ref(),
    ]);
    let flat = dir.join("flat.ssp");
    globe_ok(&[
        "ssp-export".as_ref(),
        &back.join("spec.json"),
        "-o".as_ref(),
        &flat,
    ]);
    let got = globe_ok(&["cosim".as_ref(), &flat]);
    assert_eq!(got.stdout, want.stdout);
    assert!(table(&got).contains_key("s.a.x"));
}

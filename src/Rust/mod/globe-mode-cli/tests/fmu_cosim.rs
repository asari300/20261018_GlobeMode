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
fn ssp(dir: &std::path::Path, ssd: &str, fmus: &[(&str, &PathBuf)]) -> PathBuf {
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
    let file = ssp(&dir, &SSD.replace("EXTRA", ""), &[("a", &a), ("b", &b)]);
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

#[test]
fn ssp_refuses_nested_system() {
    let (Some(a), Some(b)) = (
        reference_fmu("2.0", "Dahlquist"),
        reference_fmu("3.0", "Feedthrough"),
    ) else {
        return;
    };
    let dir = scratch("4");
    let ssd = SSD.replace("EXTRA", r#"<ssd:System name="inner"/>"#);
    let file = ssp(&dir, &ssd, &[("a", &a), ("b", &b)]);
    let out = Command::new(env!("CARGO_BIN_EXE_globe"))
        .args(["fmu", "cosim"])
        .arg(&file)
        .output()
        .unwrap();
    assert!(!out.status.success());
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(err.contains("入れ子の System"), "{err}");
}

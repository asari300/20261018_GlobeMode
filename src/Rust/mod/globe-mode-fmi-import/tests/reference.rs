//! Reference-FMUs（v0.0.42、`cargo xtask fetch-reference-fmus`。2-Clause BSD）の Co-Simulation を、この crate で読み込んで回す。
//! 突き合わせの相手は FMU が同梱する `extra/org.fmi-standard.fmi-ls-ref/<名前>_out.csv`（同じ FMU を刻み 0.01 などで
//! 回した参照結果）。取得物が無ければ飛ばす（既存の fixture の流儀）。

use std::io::Read;
use std::path::PathBuf;

use globe_mode_fmi_import::{CsOptions, Fmu, run_cs};

fn fmu_path(version: &str, name: &str) -> Option<PathBuf> {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../../data/reference-fmus")
        .join(version)
        .join(format!("{name}.fmu"));
    if p.is_file() {
        Some(p)
    } else {
        // fmu-import.yml は取得に失敗したら飛ばさず落とす
        assert!(
            std::env::var_os("GLOBE_REQUIRE_REFERENCE_FMUS").is_none(),
            "{p:?} が無い"
        );
        eprintln!("skip: {p:?} が無い（cargo xtask fetch-reference-fmus）");
        None
    }
}

/// 参照 CSV（時刻の列が先頭）。
fn reference_csv(path: &std::path::Path, name: &str) -> (Vec<String>, Vec<Vec<f64>>) {
    let mut zip = zip::ZipArchive::new(std::fs::File::open(path).unwrap()).unwrap();
    let mut text = String::new();
    zip.by_name(&format!("extra/org.fmi-standard.fmi-ls-ref/{name}_out.csv"))
        .unwrap()
        .read_to_string(&mut text)
        .unwrap();
    let mut lines = text.lines();
    let header = lines
        .next()
        .unwrap()
        .split(',')
        .map(|s| s.trim().to_owned())
        .collect();
    let rows = lines
        .map(|l| l.split(',').map(|c| c.trim().parse().unwrap()).collect())
        .collect();
    (header, rows)
}

/// `fmu` を参照 CSV と同じ刻みで回して全行を比べる。参照の FMU が前進 Euler なので、ほぼ全桁で一致する。
fn matches_reference(fmu_file: &std::path::Path, csv_file: &std::path::Path, name: &str) {
    let (header, rows) = reference_csv(csv_file, name);
    let fmu = Fmu::open(fmu_file).unwrap();
    let run = run_cs(
        &fmu,
        &CsOptions {
            stop_time: Some(rows.last().unwrap()[0]),
            step: Some(rows[1][0] - rows[0][0]),
            outputs: header[1..].to_vec(),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(run.rows.len(), rows.len(), "{name}: 行数");
    let mut worst = 0.0f64;
    for (got, want) in run.rows.iter().zip(&rows) {
        assert!(
            (got.0 - want[0]).abs() < 1e-9,
            "{name}: 時刻 {} と {}",
            got.0,
            want[0]
        );
        for (g, w) in got.1.iter().zip(&want[1..]) {
            worst = worst.max((g - w).abs() / w.abs().max(1.0));
        }
    }
    eprintln!(
        "{name} {fmu_file:?}: {} 行、最大の相対差 {worst:e}",
        rows.len()
    );
    assert!(worst < 1e-9, "{name}: 参照との最大の相対差 {worst:e}");
}

#[test]
fn reference_csv_matches_for_fmi2_and_fmi3() {
    for name in ["BouncingBall", "Dahlquist", "VanDerPol"] {
        let Some(csv_fmu) = fmu_path("3.0", name) else {
            return;
        };
        for version in ["2.0", "3.0"] {
            // 2.0 の Dahlquist・VanDerPol は CSV を持たない。同じモデル（同じ式）なので 3.0 の CSV を使う
            if let Some(fmu) = fmu_path(version, name) {
                matches_reference(&fmu, &csv_fmu, name);
            }
        }
    }
}

#[test]
fn dahlquist_parameter_and_analytic_solution() {
    for version in ["2.0", "3.0"] {
        let Some(path) = fmu_path(version, "Dahlquist") else {
            return;
        };
        let fmu = Fmu::open(&path).unwrap();
        // k = 2、内部刻み 0.1 の前進 Euler は (1 - k h)^n に一致し、解析解 exp(-k t) に近い
        let run = run_cs(
            &fmu,
            &CsOptions {
                stop_time: Some(1.0),
                step: Some(0.1),
                set: vec![("k".into(), 2.0)],
                ..Default::default()
            },
        )
        .unwrap();
        let x1 = run.rows.last().unwrap().1[0];
        assert!((x1 - 0.8f64.powi(10)).abs() < 1e-12, "{version}: {x1}");
        assert!((x1 - (-2.0f64).exp()).abs() < 0.05, "{version}: {x1}");
        // 参照 FMU の内部の積分は固定刻み 0.1 の前進 Euler なので、通信刻みを細かくしても結果は変わらない
        let fine = run_cs(
            &fmu,
            &CsOptions {
                stop_time: Some(1.0),
                step: Some(0.0125),
                set: vec![("k".into(), 2.0)],
                ..Default::default()
            },
        )
        .unwrap();
        assert!(
            (fine.rows.last().unwrap().1[0] - x1).abs() < 1e-12,
            "{version}"
        );
    }
}

#[test]
fn errors_are_reported_not_panics() {
    let Some(path) = fmu_path("3.0", "Dahlquist") else {
        return;
    };
    let fmu = Fmu::open(&path).unwrap();
    let e = run_cs(
        &fmu,
        &CsOptions {
            stop_time: Some(1.0),
            outputs: vec!["nope".into()],
            ..Default::default()
        },
    )
    .unwrap_err();
    assert!(e.to_string().contains("nope"), "{e}");
    let e = run_cs(
        &fmu,
        &CsOptions {
            stop_time: Some(0.0),
            ..Default::default()
        },
    )
    .unwrap_err();
    assert!(e.to_string().contains("終了時刻"), "{e}");
}

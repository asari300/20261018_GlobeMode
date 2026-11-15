//! 自前の段の閉包の読み込み（S307）: モデルが使うファイルだけを読んでも、全部を読んだときと平坦化の結果が同じこと。

#![cfg(feature = "native")]

use std::path::{Path, PathBuf};
use std::time::Instant;

use globe_mode_flat::Project;
use globe_mode_frontend::closure_sources;

/// `/proc/self/status` の `VmHWM`（最大 RSS、KB）。読めなければ 0
fn peak_rss_kb() -> u64 {
    std::fs::read_to_string("/proc/self/status")
        .ok()
        .and_then(|s| {
            s.lines()
                .find_map(|l| l.strip_prefix("VmHWM:"))
                .and_then(|v| v.split_whitespace().next()?.parse().ok())
        })
        .unwrap_or(0)
}

fn write(dir: &Path, rel: &str, text: &str) {
    let p = dir.join(rel);
    std::fs::create_dir_all(p.parent().unwrap()).unwrap();
    std::fs::write(p, text).unwrap();
}

/// 使わないファイル（`Unused`。構文が誤り・どこからも参照されない）は閉包に入らず、
/// 外側の `package.mo`・`extends`・`import` でたどるファイルは入る。結果は全部読みと同じ。
#[test]
fn closure_keeps_only_used_files_and_flattens_the_same() {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("native-closure");
    let _ = std::fs::remove_dir_all(&dir);
    write(&dir, "L/package.mo", "package L end L;\n");
    write(
        &dir,
        "L/Top.mo",
        "within L; model Top\n  extends Base;\n  import U = L.Units;\n  U.T t(start = 1);\nequation\n  der(x) = -t;\nend Top;\n",
    );
    write(
        &dir,
        "L/Base.mo",
        "within L; partial model Base Real x; end Base;\n",
    );
    write(
        &dir,
        "L/Units.mo",
        "within L; package Units type T = Real; end Units;\n",
    );
    write(
        &dir,
        "L/Unused.mo",
        "within L; model Unused Real y = ; end Unused;\n",
    );

    let lib = [dir.join("L")];
    let libs: Vec<&Path> = lib.iter().map(|p| p.as_path()).collect();
    let files = closure_sources(&libs, "L.Top", &[]).unwrap();
    let names: Vec<_> = files
        .iter()
        .map(|(n, _)| n.rsplit('/').next().unwrap().to_string())
        .collect();
    assert!(names.contains(&"Top.mo".to_string()), "{names:?}");
    assert!(names.contains(&"Base.mo".to_string()), "{names:?}");
    assert!(names.contains(&"Units.mo".to_string()), "{names:?}");
    assert!(!names.contains(&"Unused.mo".to_string()), "{names:?}");

    let part = Project::from_sources(files.iter().map(|(n, t)| (n.as_str(), t.as_str())));
    let full = Project::from_dirs(&libs).unwrap();
    assert_eq!(part.flatten("L.Top"), full.flatten("L.Top"));
    assert!(part.flatten("L.Top").is_ok());
}

/// MSL（`data/msl`）で、閉包だけの `Project` と全部読みの `Project` が同じ平坦化（成功も、断る理由も）を返す。
/// 時間と、読んだファイルの数も出す。`cargo test -p globe-mode-frontend --release --no-default-features --features native --test native_closure -- --ignored --nocapture`
#[test]
#[ignore = "data/msl が要る（cargo xtask fetch-msl）。release で回す"]
fn msl_closure_flattens_like_full_load() {
    let msl = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../../data/msl");
    if !msl.is_dir() {
        eprintln!("data/msl が無いので省く");
        return;
    }
    let libs = [msl.as_path()];
    let models = [
        "Modelica.Blocks.Examples.PID_Controller",
        "Modelica.Mechanics.Rotational.Examples.First",
        "Modelica.Electrical.Analog.Examples.ChuaCircuit",
        "Modelica.Electrical.Analog.Examples.Rectifier",
        "Modelica.Thermal.HeatTransfer.Examples.TwoMasses",
        "Modelica.Blocks.Examples.Filter",
        "Modelica.Mechanics.MultiBody.Examples.Elementary.Pendulum",
        "Modelica.Fluid.Examples.PumpingSystem",
        "Modelica.Mechanics.Translational.Examples.Damper",
        "Modelica.Electrical.Machines.Examples.SynchronousInductionMachines.SMPM_CurrentSource",
    ];
    // 先に閉包の道だけを回して最大 RSS（VmHWM）を見て、そのあと全部読みと比べる
    let mut closure = Vec::new();
    for m in models {
        let t = Instant::now();
        let files = closure_sources(&libs, m, &[]).unwrap();
        let n = files.len();
        let r =
            Project::from_sources(files.iter().map(|(n, t)| (n.as_str(), t.as_str()))).flatten(m);
        eprintln!(
            "{m}: 閉包 {n} ファイル {:.2?}、平坦化 {}",
            t.elapsed(),
            if r.is_ok() { "成功" } else { "断る" }
        );
        closure.push(r);
    }
    eprintln!("閉包の道の最大 RSS: {} KB", peak_rss_kb());
    let t = Instant::now();
    let full = Project::from_dirs(&libs).unwrap();
    eprintln!(
        "全部読み: {:.2?}、ここまでの最大 RSS: {} KB",
        t.elapsed(),
        peak_rss_kb()
    );
    let mut ok = 0;
    for (m, a) in models.iter().zip(&closure) {
        assert!(
            *a == full.flatten(m),
            "{m}: 閉包だけと全部読みで平坦化の結果が違う"
        );
        ok += usize::from(a.is_ok());
    }
    assert!(ok >= 3, "平坦化できたのが {ok} 本しかない");
}

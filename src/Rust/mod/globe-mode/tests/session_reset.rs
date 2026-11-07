//! #37: 長く生きる `Workspace` の RSS と `Checker::reset`（S212）。測った結果は S212 の解説書にある。
//! 測ったところ、reset しても RSS は下がらなかったので、facade の `Workspace` は reset を呼ばない。
//! ここには、reset の前後で検査の結果が変わらないことの試験と、RSS を測る手順（`#[ignore]`）を残す。

use std::path::PathBuf;

use globe_mode::Workspace;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../..")
}

fn json(ws: &mut Workspace, model: &str) -> String {
    serde_json::to_string(&ws.check(model)).unwrap()
}

/// 1 本ごとに reset する Workspace と reset しない Workspace で、同じ順にモデルを検査して JSON が一致する。
/// 同じモデルを reset の前と後に検査しても同じ（salsa のキャッシュと、足し直した文書の両方を通る）。
#[test]
fn checks_are_the_same_before_and_after_a_reset() {
    let base = root().join("src/Modelica");
    let libs = [base];
    let models = [
        "GlobeModeValidation.Simulation.Decay",
        "GlobeModeValidation.Simulation.InlinedParameters",
        "GlobeModeValidation.Simulation.ArrayParameters",
        "GlobeModeValidation.Simulation.Decay",
        "GlobeModeValidation.Simulation.InlinedParameters",
    ];
    let mut a = Workspace::with_libraries(&libs).unwrap();
    let mut b = Workspace::with_libraries(&libs).unwrap();
    for m in models {
        a.db().checker().reset();
        assert_eq!(json(&mut a, m), json(&mut b, m), "{m}");
    }
    // 利用者のファイルを足したあとの reset でも、文書が足し直って同じ結果になる
    for ws in [&mut a, &mut b] {
        ws.set_file(
            "user.mo",
            "model U parameter Real k = 2; Real x(start = 1); equation der(x) = -k*x; end U;",
        );
    }
    for m in ["U", models[0], models[1], "U"] {
        a.db().checker().reset();
        let (x, y) = (json(&mut a, m), json(&mut b, m));
        assert_eq!(x, y, "{m}");
        assert!(x.contains("\"passed\":true"), "{m}: {x}");
    }
}

fn status_kb(key: &str) -> u64 {
    std::fs::read_to_string("/proc/self/status")
        .ok()
        .and_then(|s| {
            s.lines()
                .find_map(|l| l.strip_prefix(key)?.split_whitespace().next()?.parse().ok())
        })
        .unwrap_or(0)
}

/// MSL の例題を 1 つの Workspace で続けて検査したときの RSS（`GM_RESET_EVERY` 本ごとに `Checker::reset` を呼ぶ。0 は呼ばない。既定 50）。
/// `GM_N` 本（既定 100）。`GM_FULL=1` なら全部読む道。数字は標準エラーに出す。
/// `GM_RESET_EVERY=0 cargo test -p globe-mode --test session_reset -- --ignored --nocapture rss`
#[test]
#[ignore = "MSL が要り、長い（数分）。RSS を測る"]
fn rss_stays_flat_with_resets() {
    let msl = root().join("data/msl");
    let pkg = std::fs::read_dir(&msl)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .find(|p| {
            p.file_name()
                .is_some_and(|n| n.to_string_lossy().starts_with("Modelica "))
        });
    let Some(pkg) = pkg else {
        eprintln!("data/msl が無いので飛ばす");
        return;
    };
    let env = |k: &str, d: usize| {
        std::env::var(k)
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(d)
    };
    let (n, every) = (env("GM_N", 100), env("GM_RESET_EVERY", 50));
    let mut models = Vec::new();
    let default_dirs = [
        "Electrical/Analog/Examples",
        "Mechanics/Rotational/Examples",
        "Mechanics/Translational/Examples",
        "Thermal/HeatTransfer/Examples",
        "Blocks/Examples",
        "Electrical/Digital/Examples",
        "Magnetic/FluxTubes/Examples",
        "Electrical/Machines/Examples",
    ];
    // `GM_DIRS=Fluid/Examples,Mechanics/MultiBody/Examples/Elementary` のように、重い例題の置き場を選べる
    let dirs: Vec<String> = match std::env::var("GM_DIRS") {
        Ok(v) => v.split(',').map(String::from).collect(),
        Err(_) => default_dirs.map(String::from).to_vec(),
    };
    for dir in dirs.iter().map(String::as_str) {
        let mut names: Vec<_> = std::fs::read_dir(pkg.join(dir))
            .into_iter()
            .flatten()
            .flatten()
            .map(|e| e.path())
            .filter(|p| {
                p.extension().is_some_and(|e| e == "mo") && p.file_name().unwrap() != "package.mo"
            })
            .collect();
        names.sort();
        models.extend(names.into_iter().map(|p| {
            format!(
                "Modelica.{}.{}",
                dir.replace('/', "."),
                p.file_stem().unwrap().to_string_lossy()
            )
        }));
    }
    models.truncate(n);
    let msl = msl.clone();
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(move || {
            // GM_FULL=1 は全部を読む道（`with_libraries_full`）。既定は分割読み込み（LSP・Python・C・NIF が通る道）
            let mut ws = if std::env::var("GM_FULL").is_ok() {
                Workspace::with_libraries_full(&[msl]).unwrap()
            } else {
                Workspace::with_libraries(&[msl]).unwrap()
            };
            eprintln!("models={} reset_every={every}", models.len());
            for (i, m) in models.iter().enumerate() {
                if every > 0 && i > 0 && i % every == 0 {
                    ws.db().checker().reset();
                }
                let r = ws.check(m);
                if (i + 1) % 5 == 0 {
                    eprintln!(
                        "after {:3}: VmRSS {:6} MiB  VmHWM {:6} MiB  (last {} passed={})",
                        i + 1,
                        status_kb("VmRSS:") / 1024,
                        status_kb("VmHWM:") / 1024,
                        m,
                        r.passed
                    );
                }
            }
        })
        .unwrap()
        .join()
        .unwrap();
}

/// エディタの打鍵に近い使い方: 1 つのモデルの本文を 300 回変えて検査する（モデルの名前は同じなので、異なるモデルの数では数えられない）。
/// `GM_EDIT_RESET=n` なら n 回ごとに `Checker::reset` を呼ぶ（0 は呼ばない）。
#[test]
#[ignore = "MSL が要り、長い。RSS を測る"]
fn rss_with_edits() {
    let msl = root().join("data/msl");
    let n_reset: usize = std::env::var("GM_EDIT_RESET")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(move || {
            let mut ws = Workspace::with_libraries(&[msl]).unwrap();
            for i in 1..=300usize {
                let text = format!(
                    "model U\n  import Modelica.Electrical.Analog.Basic.Resistor;\n  parameter Real k = {i};\n  Real x(start = 1);\n  Resistor r(R = {i});\nequation\n  der(x) = -k*x;\nend U;\n"
                );
                ws.set_file("user.mo", text);
                let _ = ws.check("U");
                if n_reset > 0 && i % n_reset == 0 {
                    ws.db().checker().reset();
                }
                if i % 50 == 0 {
                    eprintln!(
                        "edit {i:3}: VmRSS {:6} MiB  VmHWM {:6} MiB",
                        status_kb("VmRSS:") / 1024,
                        status_kb("VmHWM:") / 1024
                    );
                }
            }
        })
        .unwrap()
        .join()
        .unwrap();
}

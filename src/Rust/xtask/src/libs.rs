//! `cargo xtask fetch-library <名前>`: MSL 以外の公開ライブラリ（S058）を GitHub のアーカイブ（commit 固定）から
//! `data/libs/<名前>/` に取得する。`fetch-msl` と同じく sha256 で確かめる。
//!
//! 置き方は `data/msl` と同じ（`data/libs/<名前>/<パッケージ>/package.mo`）にして、`msl-funnel --library data/libs/<名前>
//! --package <根のパッケージ>` に渡せるようにする。アーカイブの最上位ディレクトリ（`<repo>-<commit>/`）は捨て、
//! 部品ごとの `subdir`（アーカイブの中のパッケージのディレクトリ。空ならアーカイブの根）を `<package>/` に置く。
//! 出典・版・ライセンスは `doc/research/libraries-2026-11-01.md`。

use std::fs;
use std::path::{Component, Path, PathBuf};
use std::process::ExitCode;

use crate::{Result, download};

/// アーカイブ 1 つから取り出す部分。
struct Part {
    /// `https://github.com/<repo>/archive/<commit>.zip`
    url: &'static str,
    sha256: &'static str,
    /// 最上位ディレクトリの下の、パッケージのディレクトリ（空ならアーカイブの根）
    subdir: &'static str,
    /// `data/libs/<名前>/` の下に置くパッケージのディレクトリ名
    package: &'static str,
}

struct Lib {
    name: &'static str,
    /// `msl-funnel --package` に渡す根のパッケージ名
    package: &'static str,
    version: &'static str,
    parts: &'static [Part],
}

const LIBS: &[Lib] = &[
    Lib {
        name: "Buildings",
        package: "Buildings",
        version: "14.0.0 (v14.0.0, ad48324b)",
        parts: &[Part {
            url: "https://github.com/lbl-srg/modelica-buildings/archive/ad48324b74e9751e2e49dac5fe3c4dcb546755a2.zip",
            sha256: "895e649843f47bad151792e133339fc1946382828be4f69c3194e20448879834",
            subdir: "Buildings",
            package: "Buildings",
        }],
    },
    Lib {
        name: "AixLib",
        package: "AixLib",
        version: "3.0.1 (v3.0.1, e5df9a8d)",
        parts: &[Part {
            url: "https://github.com/RWTH-EBC/AixLib/archive/e5df9a8da27833ee391c2f7a0adbcd80278f47d2.zip",
            sha256: "9067935c8073a1dd3881f45bc4079ff5a6cdab9b26c87e4784b9c01923a27995",
            subdir: "AixLib",
            package: "AixLib",
        }],
    },
    Lib {
        name: "IBPSA",
        package: "IBPSA",
        version: "4.0.0 (master, 2adc9194, 2026-10-29)",
        parts: &[Part {
            url: "https://github.com/ibpsa/modelica-ibpsa/archive/2adc9194f762dbee92dc0f3d0d139e5e7dbdfc71.zip",
            sha256: "1ea5ae80987f45ae0eefa1c740b93eaeacd559b68067521d594ac79123b45a00",
            subdir: "IBPSA",
            package: "IBPSA",
        }],
    },
    Lib {
        name: "ClaRa",
        package: "ClaRa",
        version: "1.9.0 (master, 573a02b1) + TILMedia 1.9.0 ClaRa (master, 1f3380f9)",
        parts: &[
            Part {
                url: "https://github.com/ClaRaLibrary/ClaRa/archive/573a02b1194e3eb2fb289e560b717f66de4b0f65.zip",
                sha256: "78f9c5af9203145afd727c9f467b5f5a81320d55f01722837490bd053d0aedf5",
                subdir: "",
                package: "ClaRa",
            },
            Part {
                url: "https://github.com/ClaRaLibrary/TILMedia/archive/1f3380f91f8ab0d746da7c45590fe55f16f046f3.zip",
                sha256: "d31661d9aea58e0e668a2c1ec0c721818db850ca173cbedbf83895091e0bcca0",
                subdir: "",
                package: "TILMedia",
            },
        ],
    },
    Lib {
        name: "ThermofluidStream",
        package: "ThermofluidStream",
        version: "1.4.1 (v1.4.1, a2faa411) + TILMediaWrapper",
        parts: &[
            Part {
                url: "https://github.com/DLR-SR/ThermofluidStream/archive/a2faa411bc7ac7de9b46c5c30b6850505cc05dac.zip",
                sha256: "671c9fa06c2ea10876a45bad4eed95c84cea16c7f1bd921042ddc4df6d25d56c",
                subdir: "ThermofluidStream",
                package: "ThermofluidStream",
            },
            Part {
                url: "https://github.com/DLR-SR/ThermofluidStream/archive/a2faa411bc7ac7de9b46c5c30b6850505cc05dac.zip",
                sha256: "671c9fa06c2ea10876a45bad4eed95c84cea16c7f1bd921042ddc4df6d25d56c",
                subdir: "TILMediaWrapper",
                package: "TILMediaWrapper",
            },
        ],
    },
    Lib {
        name: "DeltaRobot",
        package: "DeltaRobot",
        version: "1.0.2 (master, da73b5d4)",
        parts: &[Part {
            url: "https://github.com/looms-polimi/Parallel_manipulators/archive/da73b5d43284db6941426b16ffadad11a6dfa965.zip",
            sha256: "7de61807ea71ebed19a5d52cb46a0c5ef00fb15f08fac193931b4ea636b01ebd",
            subdir: "Modelica_models/DeltaRobot",
            package: "DeltaRobot",
        }],
    },
    Lib {
        name: "Chemical",
        package: "Chemical",
        version: "2.0.0 (v2.0.0, 2da36279)",
        parts: &[Part {
            url: "https://github.com/MarekMatejak/Chemical/archive/2da36279db8adf3effa142a8dfa270c40b0e517d.zip",
            sha256: "bb4892eb6ed5e66562ec79a890dd88a8ff784afa4fd05dac33bf666f1aeed709",
            subdir: "Chemical",
            package: "Chemical",
        }],
    },
];

/// 名前（大文字小文字を区別しない）からライブラリを探す。
fn find(name: &str) -> Result<&'static Lib> {
    LIBS.iter()
        .find(|l| l.name.eq_ignore_ascii_case(name))
        .ok_or_else(|| {
            let names: Vec<_> = LIBS.iter().map(|l| l.name).collect();
            format!(
                "未知のライブラリ {name}。取得できるのは {}",
                names.join(", ")
            )
        })
}

/// アーカイブの中の相対パス（最上位ディレクトリを除いたもの）を、`subdir` の下なら展開先の相対パスにする。
fn target(rel: &Path, part: &Part) -> Option<PathBuf> {
    let rest = rel.strip_prefix(part.subdir).ok()?;
    if rest.as_os_str().is_empty() {
        return None;
    }
    Some(Path::new(part.package).join(rest))
}

pub(crate) fn fetch_library(name: &str, dest: &Path) -> Result<ExitCode> {
    let lib = find(name)?;
    let dir = dest.join(lib.name);
    let marker = dir.join(".globe-mode-library");
    let expected = format!(
        "{} {}\n",
        lib.version,
        lib.parts
            .iter()
            .map(|p| p.sha256)
            .collect::<Vec<_>>()
            .join(" ")
    );
    if fs::read_to_string(&marker).ok().as_deref() == Some(expected.as_str()) {
        println!("{} は展開済み: {}", lib.name, dir.display());
        return Ok(ExitCode::SUCCESS);
    }
    let _ = fs::remove_dir_all(&dir);
    let mut count = 0usize;
    for (i, part) in lib.parts.iter().enumerate() {
        let zip_path = dir.join(".download").join(format!("{i}.zip"));
        download(part.url, part.sha256, &zip_path)?;
        let file = fs::File::open(&zip_path).map_err(|e| e.to_string())?;
        let mut archive = zip::ZipArchive::new(file).map_err(|e| e.to_string())?;
        for j in 0..archive.len() {
            let mut entry = archive.by_index(j).map_err(|e| e.to_string())?;
            // enclosed_name は「..」や絶対パスで展開先の外に出る名前を拒む
            let Some(full) = entry.enclosed_name() else {
                let n = entry.name().map(|n| n.into_owned()).unwrap_or_default();
                return Err(format!("展開先の外を指す名前: {n}"));
            };
            // 最上位ディレクトリ（<repo>-<commit>）を捨てる
            let mut parts = full
                .components()
                .filter(|c| matches!(c, Component::Normal(_)));
            parts.next();
            let rel: PathBuf = parts.collect();
            let Some(rel) = target(&rel, part) else {
                continue;
            };
            let out = dir.join(rel);
            if entry.is_dir() {
                fs::create_dir_all(&out).map_err(|e| e.to_string())?;
                continue;
            }
            if let Some(d) = out.parent() {
                fs::create_dir_all(d).map_err(|e| e.to_string())?;
            }
            let mut bytes = Vec::new();
            std::io::Read::read_to_end(&mut entry, &mut bytes).map_err(|e| e.to_string())?;
            // ClaRa の 1483 ファイルは UTF-8 の BOM で始まり、BOM つきだと構文の誤りになって例題が 0 本になる。
            // BOM を外して置く（ライブラリの中身はそれ以外変えない）
            if out.extension().is_some_and(|e| e == "mo") && bytes.starts_with(b"\xEF\xBB\xBF") {
                bytes.drain(..3);
            }
            fs::write(&out, bytes).map_err(|e| format!("{}: {e}", out.display()))?;
            count += 1;
        }
        let root = dir.join(part.package).join("package.mo");
        if !root.exists() {
            return Err(format!("{} が無い。部品の subdir を見直す", root.display()));
        }
    }
    let _ = fs::remove_dir_all(dir.join(".download"));
    fs::write(&marker, expected).map_err(|e| e.to_string())?;
    println!(
        "{} {} を展開した: {} ファイル → {}（`msl-funnel --library {} --package {}`）",
        lib.name,
        lib.version,
        count,
        dir.display(),
        dir.display(),
        lib.package
    );
    Ok(ExitCode::SUCCESS)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn table_is_well_formed() {
        for l in LIBS {
            assert!(l.parts.iter().any(|p| p.package == l.package), "{}", l.name);
            for p in l.parts {
                assert_eq!(p.sha256.len(), 64, "{}", l.name);
                assert!(p.url.ends_with(".zip"));
            }
        }
        assert_eq!(find("thermofluidstream").unwrap().name, "ThermofluidStream");
        assert!(find("Nope").is_err());
    }

    #[test]
    fn archive_paths_map_to_packages() {
        let nested = Part {
            url: "",
            sha256: "",
            subdir: "Modelica_models/DeltaRobot",
            package: "DeltaRobot",
        };
        let rel = Path::new("Modelica_models/DeltaRobot/Icons/X.mo");
        assert_eq!(
            target(rel, &nested),
            Some(PathBuf::from("DeltaRobot/Icons/X.mo"))
        );
        assert_eq!(
            target(Path::new("Modelica_models/StewartPlatform/a.mo"), &nested),
            None
        );
        assert_eq!(
            target(Path::new("Modelica_models/DeltaRobot"), &nested),
            None
        );
        let root = Part {
            subdir: "",
            package: "ClaRa",
            ..nested
        };
        assert_eq!(
            target(Path::new("package.mo"), &root),
            Some(PathBuf::from("ClaRa/package.mo"))
        );
    }
}

// --------------------------------------------------------------------------- 構文の誤りのあるファイル

// --------------------------------------------------------------------------- 集計

/// 1 ライブラリの集計（分割した複数の JSON を足したもの）。
#[derive(Debug, Default, Clone, PartialEq)]
struct Totals {
    shards: usize,
    examples: usize,
    ok: usize,
    gm_u: usize,
    panics: usize,
    funnel_seconds: f64,
    /// `msl-sim` の JSON があった分割の数
    sim_shards: usize,
    sim_total: usize,
    sim_ok: usize,
    sim_seconds: f64,
}

fn num(v: &serde_json::Value, ptr: &str) -> usize {
    v.pointer(ptr)
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(0) as usize
}

impl Totals {
    fn add_funnel(&mut self, j: &serde_json::Value) {
        self.shards += 1;
        self.examples += num(j, "/summary/total");
        self.ok += num(j, "/summary/ok");
        self.panics += num(j, "/summary/panics");
        self.gm_u += j
            .pointer("/summary/unsupported_by_code")
            .and_then(serde_json::Value::as_object)
            .map_or(0, |m| {
                m.values()
                    .filter_map(serde_json::Value::as_u64)
                    .sum::<u64>() as usize
            });
        self.funnel_seconds += j
            .pointer("/summary/seconds")
            .and_then(serde_json::Value::as_f64)
            .unwrap_or(0.0);
    }

    fn add_sim(&mut self, j: &serde_json::Value) {
        self.sim_shards += 1;
        self.sim_total += num(j, "/summary/total");
        self.sim_ok += num(j, "/summary/stop_time") + num(j, "/summary/terminated");
        self.sim_seconds += j
            .get("total_seconds")
            .and_then(serde_json::Value::as_f64)
            .unwrap_or(0.0);
    }
}

fn hours(seconds: f64) -> String {
    if seconds < 60.0 {
        return format!("{seconds:.0} s");
    }
    let m = (seconds / 60.0).round() as u64;
    format!("{}:{:02}", m / 60, m % 60)
}

fn render_libraries(rows: &std::collections::BTreeMap<String, Totals>, msl: &str) -> String {
    use std::fmt::Write as _;
    let mut md =
        String::from("# MSL 以外の公開ライブラリの検査（globe check の S0〜S10 と実行）\n\n");
    md.push_str(&format!("`.github/workflows/libraries.yml`（S058）が作った。手で編集しない。各ライブラリを `fetch-library` で取得し、**MSL {msl}** と一緒に読んで `msl-funnel` で例題に判定を出し、OK の例題を `msl-sim` で実行した（参照結果は無いので、管の比較はしない）。\n\n"));
    md.push_str("| ライブラリ | 版 | 分割 | 例題 | OK | NG | GM-U | panic | 実行した | 実行できた | 時間（検査 + 実行） |\n| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- |\n");
    for (name, t) in rows {
        let version = find(name).map_or("?", |l| l.version);
        let ng = t.examples.saturating_sub(t.ok + t.gm_u + t.panics);
        let sim = if t.sim_shards == 0 {
            "- | -".to_string()
        } else {
            format!("{} | {}", t.sim_total, t.sim_ok)
        };
        let _ = writeln!(
            md,
            "| {name} | {version} | {}/{} | {} | {} | {ng} | {} | {} | {sim} | {} + {} |",
            t.sim_shards,
            t.shards,
            t.examples,
            t.ok,
            t.gm_u,
            t.panics,
            hours(t.funnel_seconds),
            hours(t.sim_seconds),
        );
    }
    md.push_str("\n- 「分割」は `msl-sim` の JSON が揃った数 / `msl-funnel` の JSON が揃った数。落ちたジョブは欠けたまま数える。\n- 時間は各分割の足し算（並行に回すので壁時計より長い）。\n- 判定の意味は [msl-funnel.md](msl-funnel.md)（S0〜S10、GM-U は未対応）。\n");
    md
}

/// `cargo xtask libraries-report --dir <D>`: `<D>` の下の `<名前>-<分割>-funnel.json` と `<名前>-<分割>-sim.json` を集めて表にする。
pub(crate) fn libraries_report(dir: &Path, msl: &str, report: Option<&Path>) -> Result<ExitCode> {
    let mut rows: std::collections::BTreeMap<String, Totals> = std::collections::BTreeMap::new();
    let mut files = Vec::new();
    collect_json(dir, &mut files).map_err(|e| format!("{}: {e}", dir.display()))?;
    files.sort();
    for path in files {
        let file = path
            .file_name()
            .and_then(|f| f.to_str())
            .unwrap_or_default();
        let (stem, is_funnel) = match (
            file.strip_suffix("-funnel.json"),
            file.strip_suffix("-sim.json"),
        ) {
            (Some(s), _) => (s, true),
            (_, Some(s)) => (s, false),
            _ => continue,
        };
        let name = stem.split('-').next().unwrap_or(stem).to_string();
        let text = fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        let json: serde_json::Value =
            serde_json::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))?;
        let totals = rows.entry(name).or_default();
        if is_funnel {
            totals.add_funnel(&json);
        } else {
            totals.add_sim(&json);
        }
    }
    let md = render_libraries(&rows, msl);
    match report {
        Some(p) => fs::write(p, md).map_err(|e| e.to_string())?,
        None => print!("{md}"),
    }
    Ok(ExitCode::SUCCESS)
}

fn collect_json(dir: &Path, out: &mut Vec<PathBuf>) -> std::io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            collect_json(&path, out)?;
        } else if path.extension().is_some_and(|e| e == "json") {
            out.push(path);
        }
    }
    Ok(())
}

#[cfg(test)]
mod report_tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn totals_add_up_across_shards() {
        let mut t = Totals::default();
        let f = json!({"summary": {"total": 10, "ok": 6, "panics": 1, "seconds": 90.0,
            "unsupported_by_code": {"GM-U0001": 2, "GM-U0004": 1}}});
        t.add_funnel(&f);
        t.add_funnel(&f);
        t.add_sim(&json!({"summary": {"total": 6, "stop_time": 4, "terminated": 1}, "total_seconds": 3600.0}));
        assert_eq!(
            (t.shards, t.examples, t.ok, t.gm_u, t.panics),
            (2, 20, 12, 6, 2)
        );
        assert_eq!((t.sim_total, t.sim_ok), (6, 5));
        let md = render_libraries(
            &[("Chemical".to_string(), t)].into_iter().collect(),
            "4.0.0",
        );
        assert!(md.contains("**MSL 4.0.0**"), "{md}");
        assert!(md.contains("| Chemical | 2.0.0"), "{md}");
        assert!(
            md.contains("| 1/2 | 20 | 12 | 0 | 6 | 2 | 6 | 5 | 0:03 + 1:00 |"),
            "{md}"
        );
    }
}

//! MAP-LIB の参照結果（MSL 4.1.0）の取得と置き場（S011）。`cargo xtask fetch-msl-ref` と `msl-sim` が使う。
//!
//! - 取得元は `modelica/MAP-LIB_ReferenceResults` のブランチ `v4.1.0` で、commit を [`MAP_LIB_SHA`] に固定する。
//!   置き方は `Modelica/<パッケージのパス>/<名前>/<名前>.csv` と、同じ場所の `comparisonSignals.txt`（比べる信号の名前）。
//! - 置き場（既定は `data/msl-ref/`、git では無視する）は取得元と同じ形にする。根に [`MARKER`] を置き、固定した
//!   commit を書く。違う commit の取得物があれば止める（混ぜない）。
//! - 参照結果の無いモデル（566 本のうち 33 本ほど）は、モデルの場所に [`NO_REFERENCE`] を置いて覚える
//!   （commit を固定しているので、無いことも変わらない）。
//! - 取得は `curl` を呼ぶ（`fetch-msl` と同じ）。`.part` に書いてから名前を変える（途中で止めても壊れた
//!   ファイルを残さない）。すでにあるファイルは取り直さない。
//! - そのリポジトリには LICENSE が無い（再配布の条件は【未確認】）。取得は検査のときだけにし、コミットしない。

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;

use crate::Result;

/// 参照結果のリポジトリ（GitHub の `owner/name`）。
pub(crate) const MAP_LIB_REPO: &str = "modelica/MAP-LIB_ReferenceResults";
/// 参照結果のブランチ（MSL 4.1.0 の参照結果）。
pub(crate) const MAP_LIB_BRANCH: &str = "v4.1.0";
/// 固定した commit（`git ls-remote https://github.com/modelica/MAP-LIB_ReferenceResults refs/heads/v4.1.0`、
/// 2026-10-19 に確かめた）。
pub(crate) const MAP_LIB_SHA: &str = "dac67829004dd99d00778c995a00363df6088de1";

/// 置き場の根に置く印（中身は `<リポジトリ> <commit>`）。
pub(crate) const MARKER: &str = ".globe-mode-msl-ref";
/// 参照結果の無いモデルの場所に置く印。
pub(crate) const NO_REFERENCE: &str = ".globe-mode-no-reference";
/// 比べる信号の名前のファイル。
pub(crate) const SIGNALS_FILE: &str = "comparisonSignals.txt";

/// 置き場の印の中身。
fn marker_text() -> String {
    format!("{MAP_LIB_REPO} {MAP_LIB_SHA}\n")
}

/// モデルの参照結果の場所（取得元の根からの相対パス、`/` 区切り）。`Modelica.` で始まらない名前と、
/// 識別子でない区切り（引用符付きの名前、空の区切り）を含む名前は None。
pub(crate) fn model_dir(model: &str) -> Option<String> {
    let segments: Vec<&str> = model.split('.').collect();
    if segments.len() < 2 || segments[0] != "Modelica" {
        return None;
    }
    let plain = |s: &&str| {
        !s.is_empty()
            && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
            && !s.starts_with(|c: char| c.is_ascii_digit())
    };
    segments.iter().all(plain).then(|| segments.join("/"))
}

/// 参照結果の CSV の場所（`Modelica/Blocks/Examples/BooleanNetwork1/BooleanNetwork1.csv`）。
pub(crate) fn csv_path(model: &str) -> Option<String> {
    let dir = model_dir(model)?;
    let name = model.rsplit('.').next()?;
    Some(format!("{dir}/{name}.csv"))
}

/// 比べる信号の名前のファイルの場所（`Modelica/Blocks/Examples/BooleanNetwork1/comparisonSignals.txt`）。
pub(crate) fn signals_path(model: &str) -> Option<String> {
    Some(format!("{}/{SIGNALS_FILE}", model_dir(model)?))
}

/// 取得元の URL（commit を固定した raw.githubusercontent.com）。
pub(crate) fn raw_url(rel: &str) -> String {
    format!("https://raw.githubusercontent.com/{MAP_LIB_REPO}/{MAP_LIB_SHA}/{rel}")
}

/// 置き場にある 1 本の参照結果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Local {
    /// CSV と信号の名前の両方がある
    Present { csv: PathBuf, signals: PathBuf },
    /// 取得元に参照結果が無い（[`NO_REFERENCE`] がある）か、名前から場所を決められない
    Absent,
    /// まだ取得していない
    NotFetched,
}

/// 置き場（`dir`）にある `model` の参照結果。
pub(crate) fn local(dir: &Path, model: &str) -> Local {
    let (Some(rel_dir), Some(csv), Some(signals)) =
        (model_dir(model), csv_path(model), signals_path(model))
    else {
        return Local::Absent;
    };
    let (csv, signals) = (dir.join(csv), dir.join(signals));
    if csv.is_file() && signals.is_file() {
        Local::Present { csv, signals }
    } else if dir.join(rel_dir).join(NO_REFERENCE).is_file() {
        Local::Absent
    } else {
        Local::NotFetched
    }
}

/// 置き場の印を確かめる。固定した commit の印なら true、印が無ければ false（まだ何も取得していない）、
/// 違う commit の印なら誤り。
pub(crate) fn check_marker(dir: &Path) -> Result<bool> {
    match fs::read_to_string(dir.join(MARKER)) {
        Ok(text) if text == marker_text() => Ok(true),
        Ok(text) => Err(format!(
            "{} の参照結果は別の版（{}）。固定した版（{MAP_LIB_REPO} {MAP_LIB_SHA}）と混ぜないので、{} を消してから取り直す",
            dir.display(),
            text.trim(),
            dir.display()
        )),
        Err(_) => Ok(false),
    }
}

// --------------------------------------------------------------------------- 取得

/// 1 つのファイルの取得の結果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Got {
    /// すでにあった
    Cached,
    /// 取得した（バイト数）
    Fetched(u64),
    /// 取得元に無い（HTTP 404）
    NotFound,
}

/// `url` を `dest` に取得する。`dest` があれば何もしない。`.part` に書いてから名前を変える。
fn get(url: &str, dest: &Path) -> Result<Got> {
    if dest.is_file() {
        return Ok(Got::Cached);
    }
    if let Some(dir) = dest.parent() {
        fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    }
    let mut part = dest.as_os_str().to_owned();
    part.push(".part");
    let part = PathBuf::from(part);
    let out = Command::new("curl")
        .args([
            "--location",
            "--silent",
            "--show-error",
            "--retry",
            "3",
            "--retry-delay",
            "2",
            "--write-out",
            "%{http_code}",
            "--output",
        ])
        .arg(&part)
        .arg(url)
        .output()
        .map_err(|e| format!("curl を実行できない: {e}"))?;
    let status = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if !out.status.success() {
        let _ = fs::remove_file(&part);
        return Err(format!(
            "curl が失敗した（{}、HTTP {status}）: {}",
            out.status,
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    match status.as_str() {
        "200" => {
            let bytes = fs::metadata(&part).map(|m| m.len()).unwrap_or(0);
            fs::rename(&part, dest).map_err(|e| format!("{}: {e}", dest.display()))?;
            Ok(Got::Fetched(bytes))
        }
        "404" => {
            let _ = fs::remove_file(&part);
            Ok(Got::NotFound)
        }
        other => {
            let _ = fs::remove_file(&part);
            Err(format!("HTTP {other}: {url}"))
        }
    }
}

/// 1 本の取得の結果。
#[derive(Debug, Clone, PartialEq, Eq)]
enum ModelFetch {
    /// 両方ともあった（取得しなかった）
    Cached,
    /// 取得した（ファイルの数とバイト数）
    Fetched { files: usize, bytes: u64 },
    /// 取得元に参照結果が無い
    Absent,
}

fn fetch_model(dir: &Path, model: &str) -> Result<ModelFetch> {
    let (Some(rel_dir), Some(csv), Some(signals)) =
        (model_dir(model), csv_path(model), signals_path(model))
    else {
        return Ok(ModelFetch::Absent);
    };
    let absent_marker = dir.join(&rel_dir).join(NO_REFERENCE);
    if absent_marker.is_file() {
        return Ok(ModelFetch::Absent);
    }
    let mut files = 0usize;
    let mut bytes = 0u64;
    // 小さい信号の名前を先に取り、無ければ CSV を取りに行かない
    for rel in [signals, csv] {
        match get(&raw_url(&rel), &dir.join(&rel))? {
            Got::Cached => {}
            Got::Fetched(n) => {
                files += 1;
                bytes += n;
            }
            Got::NotFound => {
                let text = format!("{MAP_LIB_REPO} {MAP_LIB_SHA} に {rel} が無い\n");
                if let Some(d) = absent_marker.parent() {
                    fs::create_dir_all(d).map_err(|e| format!("{}: {e}", d.display()))?;
                }
                fs::write(&absent_marker, text)
                    .map_err(|e| format!("{}: {e}", absent_marker.display()))?;
                return Ok(ModelFetch::Absent);
            }
        }
    }
    Ok(if files == 0 {
        ModelFetch::Cached
    } else {
        ModelFetch::Fetched { files, bytes }
    })
}

/// 取得の集計。
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct FetchStats {
    pub requested: usize,
    /// すでに両方ともあった
    pub cached: usize,
    /// 取得したモデルの数、ファイルの数、バイト数
    pub fetched: usize,
    pub files: usize,
    pub bytes: u64,
    /// 取得元に参照結果が無いモデル（名前順）
    pub absent: Vec<String>,
    /// 取得に失敗したモデルと理由（名前順）
    pub failed: Vec<(String, String)>,
    pub seconds: f64,
}

impl FetchStats {
    /// 1 行の要約。
    pub(crate) fn line(&self) -> String {
        format!(
            "{} 本: 取得済み {}・取得 {}（{} ファイル、{}）・参照結果なし {}・失敗 {}（{:.1} s）",
            self.requested,
            self.cached,
            self.fetched,
            self.files,
            megabytes(self.bytes),
            self.absent.len(),
            self.failed.len(),
            self.seconds
        )
    }
}

pub(crate) fn megabytes(bytes: u64) -> String {
    format!("{:.1} MB", bytes as f64 / 1e6)
}

/// `models` の参照結果を `dir` に取得する（`jobs` 本ずつ並べて）。
pub(crate) fn fetch(dir: &Path, models: &[String], jobs: usize) -> Result<FetchStats> {
    if !check_marker(dir)? {
        fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        fs::write(dir.join(MARKER), marker_text()).map_err(|e| e.to_string())?;
    }
    let t = Instant::now();
    let next = AtomicUsize::new(0);
    let results: Mutex<Vec<(String, Result<ModelFetch>)>> = Mutex::new(Vec::new());
    let width = models.len().to_string().len();
    std::thread::scope(|scope| {
        for _ in 0..jobs.clamp(1, 16) {
            scope.spawn(|| {
                loop {
                    let i = next.fetch_add(1, Ordering::Relaxed);
                    let Some(model) = models.get(i) else {
                        break;
                    };
                    let r = fetch_model(dir, model);
                    match &r {
                        Ok(ModelFetch::Fetched { bytes, .. }) => println!(
                            "[{:>width$}/{}] 取得 {model}（{}）",
                            i + 1,
                            models.len(),
                            megabytes(*bytes)
                        ),
                        Ok(ModelFetch::Absent) => {
                            println!("[{:>width$}/{}] 参照結果なし {model}", i + 1, models.len())
                        }
                        Ok(ModelFetch::Cached) => {}
                        Err(e) => {
                            println!("[{:>width$}/{}] 失敗 {model}: {e}", i + 1, models.len())
                        }
                    }
                    if let Ok(mut all) = results.lock() {
                        all.push((model.clone(), r));
                    }
                }
            });
        }
    });
    let mut stats = FetchStats {
        requested: models.len(),
        seconds: t.elapsed().as_secs_f64(),
        ..FetchStats::default()
    };
    for (model, r) in results.into_inner().unwrap_or_default() {
        match r {
            Ok(ModelFetch::Cached) => stats.cached += 1,
            Ok(ModelFetch::Fetched { files, bytes }) => {
                stats.fetched += 1;
                stats.files += files;
                stats.bytes += bytes;
            }
            Ok(ModelFetch::Absent) => stats.absent.push(model),
            Err(e) => stats.failed.push((model, e)),
        }
    }
    stats.absent.sort();
    stats.failed.sort();
    Ok(stats)
}

/// `cargo xtask fetch-msl-ref`。
pub(crate) fn fetch_msl_ref(
    dir: &Path,
    funnel_json: &Path,
    models: &[String],
    jobs: usize,
) -> Result<ExitCode> {
    let models = if models.is_empty() {
        // partial のモデル（テンプレート）は msl-sim が実行しないので取得しない
        crate::sim::funnel_ok_models(funnel_json)?
            .into_iter()
            .filter(|t| !t.partial)
            .map(|t| t.model)
            .collect()
    } else {
        models.to_vec()
    };
    println!(
        "参照結果: {MAP_LIB_REPO} の {MAP_LIB_BRANCH}（commit {MAP_LIB_SHA}）→ {}。{} 本",
        dir.display(),
        models.len()
    );
    let stats = fetch(dir, &models, jobs)?;
    println!("{}", stats.line());
    if !stats.absent.is_empty() {
        println!("参照結果の無いモデル:");
        for m in &stats.absent {
            println!("  {m}");
        }
    }
    for (m, e) in &stats.failed {
        eprintln!("失敗: {m}: {e}");
    }
    Ok(if stats.failed.is_empty() {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn model_names_map_to_reference_paths() {
        let m = "Modelica.Blocks.Examples.BooleanNetwork1";
        assert_eq!(
            model_dir(m).as_deref(),
            Some("Modelica/Blocks/Examples/BooleanNetwork1")
        );
        assert_eq!(
            csv_path(m).as_deref(),
            Some("Modelica/Blocks/Examples/BooleanNetwork1/BooleanNetwork1.csv")
        );
        assert_eq!(
            signals_path(m).as_deref(),
            Some("Modelica/Blocks/Examples/BooleanNetwork1/comparisonSignals.txt")
        );
        assert_eq!(
            raw_url(&csv_path("Modelica.Electrical.Analog.Examples.ChuaCircuit").unwrap()),
            format!(
                "https://raw.githubusercontent.com/modelica/MAP-LIB_ReferenceResults/{MAP_LIB_SHA}/Modelica/Electrical/Analog/Examples/ChuaCircuit/ChuaCircuit.csv"
            )
        );
        // 固定した commit は 40 桁の 16 進
        assert_eq!(MAP_LIB_SHA.len(), 40);
        assert!(MAP_LIB_SHA.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn odd_names_have_no_reference_path() {
        for m in [
            "Modelica",
            "ModelicaTest.Foo",
            "Other.Examples.X",
            "Modelica..X",
            "Modelica.'quoted'.X",
            "Modelica.A/B",
            "Modelica.1A",
        ] {
            assert_eq!(model_dir(m), None, "{m}");
            assert_eq!(csv_path(m), None, "{m}");
        }
    }

    #[test]
    fn local_cache_states() {
        let dir =
            std::env::temp_dir().join(format!("globe-mode-msl-ref-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        let m = "Modelica.A.Examples.B";
        assert_eq!(local(&dir, m), Local::NotFetched);
        let d = dir.join("Modelica/A/Examples/B");
        fs::create_dir_all(&d).unwrap();
        fs::write(d.join("comparisonSignals.txt"), "time\nx\n").unwrap();
        // CSV がまだ無い（途中で止めた）なら取得していない扱い
        assert_eq!(local(&dir, m), Local::NotFetched);
        fs::write(d.join("B.csv"), "\"time\",\"x\"\n0,1\n").unwrap();
        assert_eq!(
            local(&dir, m),
            Local::Present {
                csv: d.join("B.csv"),
                signals: d.join("comparisonSignals.txt")
            }
        );
        let n = "Modelica.A.Examples.C";
        let e = dir.join("Modelica/A/Examples/C");
        fs::create_dir_all(&e).unwrap();
        fs::write(e.join(NO_REFERENCE), "none\n").unwrap();
        assert_eq!(local(&dir, n), Local::Absent);
        assert_eq!(local(&dir, "Other.X"), Local::Absent);
        // 置き場の印
        assert_eq!(check_marker(&dir), Ok(false));
        fs::write(dir.join(MARKER), marker_text()).unwrap();
        assert_eq!(check_marker(&dir), Ok(true));
        fs::write(dir.join(MARKER), "x/y 0000\n").unwrap();
        assert!(check_marker(&dir).is_err());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn fetch_stats_line_shows_counts() {
        let s = FetchStats {
            requested: 5,
            cached: 2,
            fetched: 2,
            files: 4,
            bytes: 2_500_000,
            absent: vec!["Modelica.X".into()],
            failed: vec![],
            seconds: 1.0,
        };
        let line = s.line();
        assert!(line.contains("5 本"), "{line}");
        assert!(line.contains("取得済み 2"), "{line}");
        assert!(line.contains("2.5 MB"), "{line}");
        assert!(line.contains("参照結果なし 1"), "{line}");
    }
}

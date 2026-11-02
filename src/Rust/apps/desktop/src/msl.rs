//! MSL 4.1.0 / 4.0.0 の取得（S069、S084 で版の選択・進捗・取り消し。ADR-0030）。tauri に依存しない。
//! 取得先・sha256・展開済みの目印は xtask の `fetch-msl`（`src/Rust/xtask/src/main.rs`）と同じ。
//! xtask は別 workspace の bin なので共有せず、ここに小さく写している（決めた理由は ADR-0030）。

use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use sha2::{Digest, Sha256};

/// 選べる版。先頭が既定（起動時に取得済みを探す順でもある）
pub const VERSIONS: [&str; 2] = ["4.1.0", "4.0.0"];
const URL: &str = "https://github.com/modelica/ModelicaStandardLibrary/releases/download/v4.1.0/ModelicaStandardLibrary_v4.1.0.zip";
const SHA256: &str = "5409cfa17797c52d866dc9d2675badc1378b4bbf88066e60899a489cb1cb8a2d";
const MSL_400_URL: &str = "https://github.com/modelica/ModelicaStandardLibrary/releases/download/v4.0.0/ModelicaStandardLibrary_v4.0.0.zip";
const MSL_400_SHA256: &str = "98d8e785d3383699008282b01728c2dad794317076eb31d341456e9ed053a783";
/// 展開済みの目印。最後に書くので、途中で落ちたら無く、次回やり直す
const MARKER: &str = ".globe-mode-msl";

/// 版の取得先と sha256（対応していない版は None）。
fn source(version: &str) -> Option<(&'static str, &'static str)> {
    match version {
        "4.1.0" => Some((URL, SHA256)),
        "4.0.0" => Some((MSL_400_URL, MSL_400_SHA256)),
        _ => None,
    }
}

/// 版の置き場（`<root>/<version>`。root は app data dir の `msl`）。
pub fn dest(root: &Path, version: &str) -> PathBuf {
    root.join(version)
}

/// `dest` にその版の MSL が展開済みか。
pub fn ready(dest: &Path, version: &str) -> bool {
    source(version).is_some_and(|(_, sha)| marker_is(dest, version, sha))
}

fn marker_is(dest: &Path, version: &str, sha256: &str) -> bool {
    fs::read_to_string(dest.join(MARKER)).ok().as_deref() == Some(&format!("{version} {sha256}\n"))
}

/// その版の MSL を `dest` に取得して展開する（展開済みなら何もしない）。ダウンロードは `curl`（Windows 10 以降と Linux の .deb の依存）。
/// 取得中は `progress(受信バイト, 全体)` を 0.25 秒ごとに呼ぶ（全体は Content-Length。無ければ None）。
/// `cancel` が立ったら curl を止め、途中のファイルを消して Err を返す。
pub fn fetch(
    dest: &Path,
    version: &str,
    mut progress: impl FnMut(u64, Option<u64>),
    cancel: &AtomicBool,
) -> Result<(), String> {
    let (url, sha256) = source(version)
        .ok_or_else(|| format!("対応していない MSL の版: {version}（4.1.0 か 4.0.0）"))?;
    if ready(dest, version) {
        return Ok(());
    }
    let zip = dest.join(".download").join("msl.zip");
    fs::create_dir_all(zip.parent().expect("parent")).map_err(|e| e.to_string())?;
    let total = content_length(url);
    let mut child = curl()
        .args([
            "--fail",
            "--location",
            "--silent",
            "--show-error",
            "--output",
        ])
        .arg(&zip)
        .arg(url)
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("curl を実行できない（MSL の取得に要る）: {e}"))?;
    let status = loop {
        if cancel.load(Ordering::Relaxed) {
            let _ = child.kill();
            let _ = child.wait();
            let _ = fs::remove_file(&zip);
            let _ = fs::remove_dir(dest.join(".download"));
            return Err("MSL の取得を取り消した".into());
        }
        if let Some(st) = child.try_wait().map_err(|e| e.to_string())? {
            break st;
        }
        progress(fs::metadata(&zip).map_or(0, |m| m.len()), total);
        std::thread::sleep(Duration::from_millis(250));
    };
    if !status.success() {
        let mut err = String::new();
        if let Some(mut e) = child.stderr.take() {
            let _ = e.read_to_string(&mut err);
        }
        let _ = fs::remove_file(&zip);
        return Err(format!("curl が失敗した（{status}）: {}", err.trim()));
    }
    install(&zip, version, sha256, dest)
}

fn curl() -> Command {
    let mut cmd = Command::new("curl");
    cmd.stdin(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    }
    cmd
}

/// HEAD で Content-Length を聞く（リダイレクトの最後の応答の値）。分からなければ None。
fn content_length(url: &str) -> Option<u64> {
    let out = curl()
        .args(["--fail", "--location", "--silent", "--head", url])
        .output()
        .ok()?;
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_once(':'))
        .filter(|(k, _)| k.eq_ignore_ascii_case("content-length"))
        .filter_map(|(_, v)| v.trim().parse().ok())
        .next_back()
}

/// 取得した zip の sha256 を確かめて `dest` に展開し、最後に目印を書く。zip は成否にかかわらず消す。
fn install(zip: &Path, version: &str, sha256: &str, dest: &Path) -> Result<(), String> {
    let result = extract(zip, version, sha256, dest);
    let _ = fs::remove_file(zip);
    let _ = fs::remove_dir(dest.join(".download"));
    result
}

fn extract(zip: &Path, version: &str, sha256: &str, dest: &Path) -> Result<(), String> {
    let got = sha256_file(zip)?;
    if got != sha256 {
        return Err(format!("sha256 が一致しない: 期待 {sha256}、実際 {got}"));
    }
    let file = fs::File::open(zip).map_err(|e| e.to_string())?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| e.to_string())?;
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).map_err(|e| e.to_string())?;
        // enclosed_name は「..」や絶対パスで展開先の外に出る名前を拒む
        let Some(rel) = entry.enclosed_name() else {
            return Err(format!("展開先の外を指す名前: {:?}", entry.name()));
        };
        let out = dest.join(rel);
        if entry.is_dir() {
            fs::create_dir_all(&out).map_err(|e| e.to_string())?;
            continue;
        }
        if let Some(dir) = out.parent() {
            fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        }
        let mut w = fs::File::create(&out).map_err(|e| format!("{}: {e}", out.display()))?;
        std::io::copy(&mut entry, &mut w).map_err(|e| e.to_string())?;
    }
    fs::write(dest.join(MARKER), format!("{version} {sha256}\n")).map_err(|e| e.to_string())
}

fn sha256_file(path: &Path) -> Result<String, String> {
    let mut f = fs::File::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; 1 << 16];
    loop {
        let n = f.read(&mut buf).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hasher
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    /// 1 ファイルだけの zip を作る（名前は `name`）。
    fn make_zip(path: &Path, name: &str) {
        let mut z = zip::ZipWriter::new(fs::File::create(path).unwrap());
        z.start_file(name, zip::write::SimpleFileOptions::default())
            .unwrap();
        z.write_all(b"within;").unwrap();
        z.finish().unwrap();
    }

    fn scratch(tag: &str) -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!("globe-msl-{tag}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn install_checks_sha_extracts_and_marks_last() {
        let d = scratch("ok");
        let zip = d.join("a.zip");
        make_zip(&zip, "Modelica 4.1.0/package.mo");
        let sha = sha256_file(&zip).unwrap();
        let dest = d.join("msl");
        // 違う sha256 なら目印も展開も無い
        let bad = install(&zip, "9.9.9", &"0".repeat(64), &dest);
        assert!(bad.unwrap_err().contains("sha256"));
        assert!(!dest.join(MARKER).exists() && !dest.join("Modelica 4.1.0").exists());
        // 消されたので作り直す
        make_zip(&zip, "Modelica 4.1.0/package.mo");
        install(&zip, "9.9.9", &sha, &dest).unwrap();
        assert!(dest.join("Modelica 4.1.0/package.mo").is_file());
        assert!(marker_is(&dest, "9.9.9", &sha));
        assert!(!marker_is(&dest, "9.9.8", &sha));
        assert!(!zip.exists());
        let _ = fs::remove_dir_all(d);
    }

    #[test]
    fn versions_have_sources_and_own_dirs() {
        let root = Path::new("data").join("msl");
        for v in VERSIONS {
            let (url, sha) = source(v).unwrap();
            assert!(url.contains(&format!("/v{v}/")) && sha.len() == 64, "{v}");
            assert_eq!(dest(&root, v), root.join(v));
        }
        assert!(source("3.2.3").is_none());
        // 置き場が無ければ展開済みではない（版の違う置き場を取り違えない）
        assert!(!ready(&dest(&root, "4.0.0"), "4.0.0"));
        let err = fetch(&root, "3.2.3", |_, _| {}, &AtomicBool::new(false)).unwrap_err();
        assert!(err.contains("3.2.3"));
    }

    #[test]
    fn install_refuses_names_outside_dest() {
        let d = scratch("esc");
        let zip = d.join("a.zip");
        make_zip(&zip, "../evil.mo");
        let sha = sha256_file(&zip).unwrap();
        let dest = d.join("msl");
        assert!(install(&zip, "9.9.9", &sha, &dest).is_err());
        assert!(!d.join("evil.mo").exists() && !marker_is(&dest, "9.9.9", &sha));
        let _ = fs::remove_dir_all(d);
    }
}

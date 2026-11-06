//! FMI 2.0 / 3.0 の Import の最初の一切れ（S204、P5。[ADR-0026]）: Co-Simulation を回す。
//!
//! - [`read_model_description`] は、`.fmu`（zip）から `modelDescription.xml` だけを読む。**ネイティブコードは実行しない**。
//! - [`Fmu::open`] は、`.fmu` を一時ディレクトリに展開する（zip slip を防ぐ）。この段階でもまだ何も実行しない。
//! - [`Fmu::instantiate_cs`] は、その OS の `binaries/` の共有ライブラリを**読み込み**（ここで FMU の中のネイティブコードが走りうる）、
//!   [`FmuCs`] を返す。実装は FMI の C ABI を自前で読む案 A（`abi2`・`abi3`。ADR-0026 の決定）。
//! - [`run_cs`] は、固定刻みで終了時刻まで進めて出力を集める。
//!
//! FMU は任意のネイティブコードを実行する。信頼できない FMU は開かないこと（[`TRUST_WARNING`]）。
//! 共有ライブラリを読めないので wasm32・Android では使えない（利用側が feature・cfg で外す）。
//!
//! [ADR-0026]: https://github.com/asari300/20261018_GlobeMode/blob/main/doc/adr/0026-fmi-import-placement.md

mod abi2;
mod abi3;

use std::fmt;
use std::io::Read;
use std::path::{Path, PathBuf};

use globe_mode_fmi::{Causality, ModelDescription, VarType, parse_model_description};

/// 実行の前に利用者に見せる 1 行。
pub const TRUST_WARNING: &str =
    "警告: FMU の中のネイティブコードを実行する。信頼できる FMU だけを開くこと";

/// 展開してよい大きさの上限（zip 爆弾への備え）。
const MAX_UNPACKED: u64 = 2 << 30;

#[derive(Debug)]
pub enum Error {
    Io(String),
    /// zip として壊れている、または危険なエントリ（`..`・絶対パス・シンボリックリンク）、大きすぎる。
    Zip(String),
    Description(globe_mode_fmi::Error),
    /// 共有ライブラリが無い・読めない・関数が無い。
    Library(String),
    /// FMU の関数が `OK` / `Warning` 以外を返した、または引数が合わない。
    Fmu(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Io(m) => write!(f, "入出力の失敗: {m}"),
            Error::Zip(m) => write!(f, "FMU（zip）を開けない: {m}"),
            Error::Description(e) => write!(f, "{e}"),
            Error::Library(m) => write!(f, "共有ライブラリを読めない: {m}"),
            Error::Fmu(m) => write!(f, "FMU が失敗を返した: {m}"),
        }
    }
}

impl std::error::Error for Error {}

pub type Result<T> = std::result::Result<T, Error>;

fn io(e: impl fmt::Display) -> Error {
    Error::Io(e.to_string())
}

fn zip_err(e: impl fmt::Display) -> Error {
    Error::Zip(e.to_string())
}

/// `.fmu` の `modelDescription.xml` だけを読む。共有ライブラリには触れない（`globe fmu info` の道）。
pub fn read_model_description(path: &Path) -> Result<ModelDescription> {
    let file = std::fs::File::open(path).map_err(io)?;
    let mut zip = zip::ZipArchive::new(file).map_err(zip_err)?;
    let entry = zip.by_name("modelDescription.xml").map_err(zip_err)?;
    let mut xml = String::new();
    // 上限つき（巨大な XML で落ちない）
    entry.take(64 << 20).read_to_string(&mut xml).map_err(io)?;
    parse_model_description(&xml).map_err(Error::Description)
}

/// 展開した FMU。drop で展開先を消す（読み込んだ [`FmuCs`] より後に drop されるよう、借用で結ぶ）。
#[derive(Debug)]
pub struct Fmu {
    dir: tempfile::TempDir,
    pub description: ModelDescription,
}

/// zip のエントリを `root` の下に展開する。名前が `root` の外へ出るもの（zip slip）、シンボリックリンク、
/// 展開後の合計が [`MAX_UNPACKED`] を超えるものは**黙って飛ばさず**失敗にする。
fn unpack(file: std::fs::File, root: &Path) -> Result<()> {
    let mut zip = zip::ZipArchive::new(file).map_err(zip_err)?;
    let mut total = 0u64;
    for i in 0..zip.len() {
        let mut entry = zip.by_index(i).map_err(zip_err)?;
        let name = entry.name().to_owned();
        let Some(rel) = entry.enclosed_name() else {
            return Err(Error::Zip(format!("危険なパス: {name:?}")));
        };
        if entry.unix_mode().is_some_and(|m| m & 0o170000 == 0o120000) {
            return Err(Error::Zip(format!("シンボリックリンク: {name:?}")));
        }
        let out = root.join(rel);
        if entry.is_dir() {
            std::fs::create_dir_all(&out).map_err(io)?;
            continue;
        }
        if let Some(parent) = out.parent() {
            std::fs::create_dir_all(parent).map_err(io)?;
        }
        let mut f = std::fs::File::create(&out).map_err(io)?;
        let n =
            std::io::copy(&mut (&mut entry).take(MAX_UNPACKED - total + 1), &mut f).map_err(io)?;
        total += n;
        if total > MAX_UNPACKED {
            return Err(Error::Zip("展開後が 2 GiB を超える".into()));
        }
    }
    Ok(())
}

impl Fmu {
    /// 一時ディレクトリに展開する。まだ何も実行しない。
    pub fn open(path: &Path) -> Result<Fmu> {
        let dir = tempfile::Builder::new()
            .prefix("globe-fmu-")
            .tempdir()
            .map_err(io)?;
        unpack(std::fs::File::open(path).map_err(io)?, dir.path())?;
        let xml = std::fs::read_to_string(dir.path().join("modelDescription.xml")).map_err(io)?;
        let description = parse_model_description(&xml).map_err(Error::Description)?;
        Ok(Fmu { dir, description })
    }

    pub fn root(&self) -> &Path {
        self.dir.path()
    }

    /// この OS・CPU の `binaries/<置き場>/<modelIdentifier>.<拡張子>`。2.0 は旧名（linux64 など）と新名（x86_64-linux）の両方を試す。
    fn library_path(&self, model_identifier: &str) -> Result<PathBuf> {
        // modelIdentifier は FMU の作者が書く。`binaries/` の外のファイルを読み込ませない
        if model_identifier.is_empty()
            || model_identifier.contains(['/', '\\', '\0'])
            || model_identifier.contains("..")
        {
            return Err(Error::Library(format!(
                "modelIdentifier {model_identifier:?} が不正（パスの区切りを含む）"
            )));
        }
        let major = self
            .description
            .major()
            .ok_or_else(|| Error::Library("fmiVersion が 2 でも 3 でもない".into()))?;
        let os = if cfg!(windows) {
            "windows"
        } else if cfg!(target_os = "macos") {
            "darwin"
        } else {
            "linux"
        };
        let triple = format!("{}-{os}", std::env::consts::ARCH);
        let mut dirs = vec![triple];
        if major == 2 && cfg!(target_arch = "x86_64") {
            dirs.push(
                if cfg!(windows) {
                    "win64"
                } else if cfg!(target_os = "macos") {
                    "darwin64"
                } else {
                    "linux64"
                }
                .into(),
            );
        }
        let file = format!("{model_identifier}.{}", std::env::consts::DLL_EXTENSION);
        dirs.iter()
            .map(|d| self.root().join("binaries").join(d).join(&file))
            .find(|p| p.is_file())
            .ok_or_else(|| Error::Library(format!("binaries/{{{}}}/{file} が無い（この OS・CPU 向けの共有ライブラリが入っていない）", dirs.join(","))))
    }

    /// 共有ライブラリを**読み込み**、Co-Simulation のインスタンスを作る。FMU のネイティブコードが走る。
    pub fn instantiate_cs(&self, instance_name: &str) -> Result<Box<dyn FmuCs + '_>> {
        let cs = self
            .description
            .co_simulation
            .as_ref()
            .ok_or_else(|| Error::Fmu("この FMU は Co-Simulation を持たない".into()))?;
        let lib = self.library_path(&cs.model_identifier)?;
        match self.description.major() {
            Some(2) => Ok(Box::new(abi2::Cs2::new(self, &lib, instance_name)?)),
            Some(3) => Ok(Box::new(abi3::Cs3::new(self, &lib, instance_name)?)),
            _ => Err(Error::Library("fmiVersion が 2 でも 3 でもない".into())),
        }
    }
}

/// Co-Simulation の FMU 1 つ。drop で `Terminate` と `FreeInstance` を呼ぶ。
/// 実装は FMI の版ごとの C ABI（`abi2`・`abi3`。案 A）。案 B（crate `fmi`）は入れていない（ADR-0026 の S204 の追記）。
pub trait FmuCs {
    /// 初期化を終える（2.0 は `SetupExperiment` → `Enter/ExitInitializationMode`、3.0 は `EnterInitializationMode` → `Exit…`）。
    /// パラメータの設定（[`FmuCs::set_real`]）はこの前に。
    fn initialize(&mut self, start_time: f64, stop_time: Option<f64>) -> Result<()>;
    fn get_real(&mut self, value_references: &[u32]) -> Result<Vec<f64>>;
    fn set_real(&mut self, value_references: &[u32], values: &[f64]) -> Result<()>;
    /// `[time, time + step]` を進める。FMU が打ち切りを求めたら `true`。
    fn do_step(&mut self, time: f64, step: f64) -> Result<bool>;
}

/// `status`（FMI 2.0 は `fmi2OK`..`fmi2Pending`、3.0 は `fmi3OK`..`fmi3Fatal`）。OK と Warning は成功。
pub(crate) fn check(what: &str, status: std::ffi::c_int) -> Result<()> {
    let name = match status {
        0 | 1 => return Ok(()),
        2 => "Discard",
        3 => "Error",
        4 => "Fatal",
        5 => "Pending",
        _ => "不明",
    };
    Err(Error::Fmu(format!("{what} が {name}（{status}）を返した")))
}

/// 空白なしの `file://` URI（FMI 2.0 の `fmuResourceLocation`）。
pub(crate) fn file_uri(dir: &Path) -> String {
    let p = dir.to_string_lossy().replace('\\', "/");
    let mut s = String::from(if p.starts_with('/') {
        "file://"
    } else {
        "file:///"
    });
    for b in p.bytes() {
        if b.is_ascii_alphanumeric() || b"-._~/:".contains(&b) {
            s.push(b as char);
        } else {
            s.push_str(&format!("%{b:02X}"));
        }
    }
    s
}

#[derive(Debug, Clone, Default)]
pub struct CsOptions {
    /// 省略時は `DefaultExperiment` の値（無ければ 0）。
    pub start_time: Option<f64>,
    /// 省略時は `DefaultExperiment` の値（無ければエラー）。
    pub stop_time: Option<f64>,
    /// 通信刻み。省略時は (stop - start) / 500。
    pub step: Option<f64>,
    /// 集める変数の名前。空なら causality が output の Float64 すべて。
    pub outputs: Vec<String>,
    /// 初期化の前に与える Float64 のパラメータ（名前, 値）。
    pub set: Vec<(String, f64)>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CsRun {
    pub names: Vec<String>,
    /// `(時刻, 各変数の値)`。先頭は開始時刻の値。
    pub rows: Vec<(f64, Vec<f64>)>,
}

fn find_float(md: &ModelDescription, name: &str) -> Result<u32> {
    let v = md
        .variables
        .iter()
        .find(|v| v.name == name)
        .ok_or_else(|| Error::Fmu(format!("変数 {name:?} が無い")))?;
    if v.ty != VarType::Float64 {
        return Err(Error::Fmu(format!(
            "変数 {name:?} は Float64（Real）でない（この版は Float64 だけ）"
        )));
    }
    Ok(v.value_reference)
}

fn num(s: &Option<String>) -> Option<f64> {
    s.as_deref().and_then(|s| s.trim().parse().ok())
}

/// 固定刻みの Co-Simulation を終了時刻まで回す。
pub fn run_cs(fmu: &Fmu, opts: &CsOptions) -> Result<CsRun> {
    let md = &fmu.description;
    let exp = md.default_experiment.clone().unwrap_or_default();
    let start = opts.start_time.or(num(&exp.start_time)).unwrap_or(0.0);
    let stop = opts
        .stop_time
        .or(num(&exp.stop_time))
        .ok_or_else(|| Error::Fmu("終了時刻が無い（DefaultExperiment にも無い）".into()))?;
    if stop.partial_cmp(&start) != Some(std::cmp::Ordering::Greater) {
        return Err(Error::Fmu("終了時刻が開始時刻以下".into()));
    }
    let step = opts.step.unwrap_or((stop - start) / 500.0);
    if step.partial_cmp(&0.0) != Some(std::cmp::Ordering::Greater) {
        return Err(Error::Fmu("刻みが正でない".into()));
    }
    let names: Vec<String> = if opts.outputs.is_empty() {
        md.variables
            .iter()
            .filter(|v| v.causality == Causality::Output && v.ty == VarType::Float64)
            .map(|v| v.name.clone())
            .collect()
    } else {
        opts.outputs.clone()
    };
    let vrs = names
        .iter()
        .map(|n| find_float(md, n))
        .collect::<Result<Vec<_>>>()?;
    let mut inst = fmu.instantiate_cs("globe")?;
    for (name, value) in &opts.set {
        inst.set_real(&[find_float(md, name)?], &[*value])?;
    }
    inst.initialize(start, Some(stop))?;
    let mut rows = vec![(start, inst.get_real(&vrs)?)];
    let n = ((stop - start) / step - 1e-9).ceil() as u64;
    for i in 0..n {
        let t = start + i as f64 * step;
        let h = step.min(stop - t);
        let terminated = inst.do_step(t, h)?;
        rows.push((t + h, inst.get_real(&vrs)?));
        if terminated {
            break;
        }
    }
    Ok(CsRun { names, rows })
}

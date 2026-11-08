//! FMU のワーカー（別プロセス。S210、ADR-0026 の (n)）の試験。`globe` 自身をワーカーにして確かめる。
//!
//! - 普通の FMU（Reference-FMUs。無ければ飛ばす）の結果が、プロセス内の実行と**全桁で同じ**になる
//! - 落ちる FMU（`cc` で小さな共有ライブラリを作る。`cc` が無い・Windows なら飛ばす）: `abort`・segfault・無限ループ・
//!   メモリの食いつぶし。親（この試験のプロセスと `globe`）は落ちず、誤りとして受け取り、`--timeout` で止まり、
//!   子のプロセスが残らない（zombie も孤児も無い）

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

use globe_mode::{CancelToken, FmuKind, FmuRunRequest, import_fmu_info, run_fmu};
use globe_mode_fmi_import::{
    Error, Kind, WorkerRequest, WorkerSpec, run_in_process, run_in_worker,
};

fn globe_spec() -> WorkerSpec {
    WorkerSpec::new(env!("CARGO_BIN_EXE_globe"), &["fmu", "__worker"])
}

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

#[test]
fn worker_gives_the_same_result_as_in_process() {
    for (version, name, kind) in [
        ("2.0", "Dahlquist", Kind::Cs),
        ("3.0", "Dahlquist", Kind::Cs),
        ("2.0", "BouncingBall", Kind::Me),
        ("3.0", "BouncingBall", Kind::Me),
        ("3.0", "VanDerPol", Kind::Me),
    ] {
        let Some(file) = reference_fmu(version, name) else {
            return;
        };
        let req = WorkerRequest {
            file,
            kind: Some(kind),
            ..Default::default()
        };
        let direct = run_in_process(&req, &mut |_| {}).unwrap();
        let worker = run_in_worker(&globe_spec(), &req, &mut |_| {}).unwrap();
        // ビット単位で同じ（f64 を {:?} で往復させている）
        assert_eq!(direct, worker, "{version} {name} {kind:?}");
        assert_eq!(worker.me.is_some(), kind == Kind::Me);
    }
}

/// S229: facade の `import_fmu_info` / `run_fmu`。結果の列は実行の結果と同じ形（`time` が先頭）で、CLI の `fmu run` と同じ値。
#[test]
fn facade_reads_info_and_runs_in_a_worker() {
    let Some(file) = reference_fmu("3.0", "Dahlquist") else {
        return;
    };
    let info = import_fmu_info(&file).unwrap();
    assert_eq!(info.model_name, "Dahlquist");
    assert!(info.interfaces.iter().any(|i| i.kind == "CoSimulation"));
    assert!(info.variables.iter().any(|v| v.name == "k"));
    let json = serde_json::to_value(&info).unwrap();
    assert_eq!(json["model_name"], "Dahlquist");
    assert!(import_fmu_info(Path::new("/no/such.fmu")).is_err());

    let exe = Path::new(env!("CARGO_BIN_EXE_globe"));
    let request = FmuRunRequest {
        kind: Some(FmuKind::Cs),
        stop_time: Some(1.0),
        set: [("k".to_owned(), 2.0)].into(),
        ..Default::default()
    };
    let run = run_fmu(exe, &file, &request, &mut |_| {}, None).unwrap();
    let (layout, columns) = run.results.columns();
    assert_eq!(layout.names, ["time", "x"]);
    assert_eq!(layout.defaults, ["x"]);
    assert_eq!(layout.points, 501); // CS の既定の刻みは (終了 - 開始) / 500
    assert!(run.me.is_none());
    // 内部刻み 0.1 の前進 Euler（run_prints_a_warning_and_the_csv と同じ）
    assert!((columns[1][500] - 0.8f64.powi(10)).abs() < 1e-12);

    // JSON からも読める（LSP・Python の request）。未知の鍵は誤り
    let request: FmuRunRequest = serde_json::from_str(r#"{"kind": "me", "stop_time": 1}"#).unwrap();
    let run = run_fmu(exe, &file, &request, &mut |_| {}, None).unwrap();
    assert!(run.me.is_some());
    assert!(serde_json::from_str::<FmuRunRequest>(r#"{"stop": 1}"#).is_err());

    // FMU の中の失敗はワーカーの文面のまま、timeout の不正は実行の前に断る
    let bad = FmuRunRequest {
        outputs: vec!["nope".into()],
        ..Default::default()
    };
    assert!(
        run_fmu(exe, &file, &bad, &mut |_| {}, None)
            .unwrap_err()
            .contains("nope")
    );
    let bad = FmuRunRequest {
        timeout: Some(-1.0),
        ..Default::default()
    };
    assert!(
        run_fmu(exe, &file, &bad, &mut |_| {}, None)
            .unwrap_err()
            .contains("timeout")
    );
}

#[test]
fn worker_reports_errors_from_inside_without_the_crash_wording() {
    let Some(file) = reference_fmu("3.0", "Dahlquist") else {
        return;
    };
    let req = WorkerRequest {
        file,
        outputs: vec!["nope".into()],
        ..Default::default()
    };
    let e = run_in_worker(&globe_spec(), &req, &mut |_| {}).unwrap_err();
    assert!(matches!(e, Error::Child(_)), "{e}");
    assert!(
        e.to_string().contains("nope") && !e.to_string().contains("ワーカー"),
        "{e}"
    );
    let e = run_in_worker(&WorkerSpec::new("/no/such/exe", &[]), &req, &mut |_| {}).unwrap_err();
    assert!(matches!(e, Error::Worker(_)), "{e}");
}

// ---- 落ちる FMU ----

const C: &str = r#"
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
typedef void *C;
/* MODE 0: 普通（1 歩 3 ms かかる）、1: Instantiate で abort、2: DoStep が戻らない、3: GetReal がメモリを食い尽くす、4: DoStep で segfault */
C fmi2Instantiate(const char *a, int b, const char *c, const char *d, const void *e, int f, int g) {
#if MODE == 1
  abort();
#endif
  return malloc(16);
}
void fmi2FreeInstance(C c) { free(c); }
int fmi2SetupExperiment(C c, int a, double b, double d, int e, double f) { return 0; }
int fmi2EnterInitializationMode(C c) { return 0; }
int fmi2ExitInitializationMode(C c) { return 0; }
int fmi2Terminate(C c) { return 0; }
int fmi2SetReal(C c, const unsigned *vr, size_t n, const double *v) { return 0; }
int fmi2GetReal(C c, const unsigned *vr, size_t n, double *v) {
#if MODE == 3
  for (;;) { char *p = malloc(1 << 20); if (!p) abort(); memset(p, 1, 1 << 20); }
#endif
  for (size_t i = 0; i < n; i++) v[i] = 0;
  return 0;
}
int fmi2DoStep(C c, double t, double h, int nosetprior) {
#if MODE == 2
  for (;;) { }
#elif MODE == 4
  *(volatile int *)0 = 1;
#else
  usleep(3000);
#endif
  return 0;
}
"#;

const XML: &str = r#"<fmiModelDescription fmiVersion="2.0" modelName="Crash" guid="{GUID}" numberOfEventIndicators="0">
<CoSimulation modelIdentifier="crash"/>
<DefaultExperiment startTime="0" stopTime="2" stepSize="0.01"/>
<ModelVariables><ScalarVariable name="y" valueReference="0" causality="output" variability="continuous"><Real/></ScalarVariable></ModelVariables>
<ModelStructure><Outputs><Unknown index="1"/></Outputs></ModelStructure></fmiModelDescription>"#;

/// 落ちる FMU を組み立てる。`cc` が無い・Windows なら None。
fn crash_fmu(mode: u32) -> Option<PathBuf> {
    crash_fmu_tagged(mode, "1")
}

/// `tag` は modelDescription.xml の guid に入る（展開先が残っていないかを、ほかの試験と取り違えずに探すため）。
fn crash_fmu_tagged(mode: u32, tag: &str) -> Option<PathBuf> {
    if cfg!(windows) || Command::new("cc").arg("--version").output().is_err() {
        eprintln!("skip: Windows か、cc が無い");
        return None;
    }
    let dir =
        PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("globe-fmu-worker-{mode}-{tag}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("crash.c"), C).unwrap();
    let lib = dir.join(format!("crash.{}", std::env::consts::DLL_EXTENSION));
    let o = Command::new("cc")
        .args(["-shared", "-fPIC", &format!("-DMODE={mode}"), "-o"])
        .arg(&lib)
        .arg(dir.join("crash.c"))
        .output()
        .unwrap();
    assert!(o.status.success(), "cc: {o:?}");
    let os = if cfg!(target_os = "macos") {
        "darwin"
    } else {
        "linux"
    };
    let fmu = dir.join("crash.fmu");
    let mut w = zip::ZipWriter::new(std::fs::File::create(&fmu).unwrap());
    let opts = zip::write::SimpleFileOptions::default();
    w.start_file("modelDescription.xml", opts).unwrap();
    w.write_all(XML.replace("{GUID}", &format!("{{{tag}}}")).as_bytes())
        .unwrap();
    // 2.0 は x86_64 だけ旧名（linux64 など）も試すが、新名（<arch>-<os>）で足りる
    w.start_file(
        format!(
            "binaries/{}-{os}/{}",
            std::env::consts::ARCH,
            lib.file_name().unwrap().to_string_lossy()
        ),
        opts,
    )
    .unwrap();
    w.write_all(&std::fs::read(&lib).unwrap()).unwrap();
    w.finish().unwrap();
    Some(fmu)
}

fn request(file: &Path) -> WorkerRequest {
    WorkerRequest {
        file: file.to_owned(),
        ..Default::default()
    }
}

/// この試験のプロセスの子の数（Linux だけ。zombie も数える）。`/proc/self/task/*/children` は CONFIG_PROC_CHILDREN が要る。
fn children() -> Option<usize> {
    let mut n = 0;
    for t in std::fs::read_dir("/proc/self/task").ok()? {
        let text = std::fs::read_to_string(t.ok()?.path().join("children")).ok()?;
        n += text.split_whitespace().count();
    }
    Some(n)
}

/// #49: 展開した FMU（guid が `tag` のもの）の一時ディレクトリが残っている数。
fn leaked_unpacked(tag: &str) -> usize {
    std::fs::read_dir(std::env::temp_dir())
        .unwrap()
        .flatten()
        .filter(|e| e.file_name().to_string_lossy().starts_with("globe-fmu-"))
        .filter(|e| {
            std::fs::read_to_string(e.path().join("modelDescription.xml"))
                .is_ok_and(|x| x.contains(tag))
        })
        .count()
}

fn assert_no_children() {
    if let Some(n) = children() {
        assert_eq!(n, 0, "子のプロセスが残っている（zombie か孤児）");
    }
}

/// 落ちる FMU は 1 つの #[test] の中で順に回す（子の数を数えるため。ほかの試験と同じプロセスで並行に子を作らない）。
#[test]
fn crashing_fmus_are_reported_and_leave_no_process() {
    if crash_fmu(0).is_none() {
        return;
    }
    // 普通の（遅い）FMU は通り、進捗が流れる。ここが通らなければ、下の失敗は FMU のせいでなく起動の失敗
    let ok = crash_fmu(0).unwrap();
    let mut progress = Vec::new();
    let run = run_in_worker(&globe_spec(), &request(&ok), &mut |t| progress.push(t)).unwrap();
    assert_eq!(run.rows.len(), 501); // CS の既定の刻みは (終了 - 開始) / 500
    assert!(!progress.is_empty(), "進捗が来ない");
    assert_no_children();

    // S229: 中断の旗が立てば 50 ms おきの確認で kill して「中断した」。子は残らない（run_in_worker と facade の run_fmu の両方）
    let flag = CancelToken::new();
    let mut spec = globe_spec();
    spec.cancel = Some(flag.flag());
    let stopper = {
        let flag = flag.clone();
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(400));
            flag.cancel();
        })
    };
    let started = Instant::now();
    let e = run_in_worker(&spec, &request(&ok), &mut |_| {}).unwrap_err();
    stopper.join().unwrap();
    assert!(
        matches!(e, Error::Worker(_)) && e.to_string().contains("中断した"),
        "{e}"
    );
    assert!(
        started.elapsed() < Duration::from_millis(1400),
        "中断が遅い: {:?}",
        started.elapsed()
    );
    assert_no_children();
    let e = run_fmu(
        Path::new(env!("CARGO_BIN_EXE_globe")),
        &ok,
        &FmuRunRequest::default(),
        &mut |_| {},
        Some(&flag), // 既に立っている
    )
    .unwrap_err();
    assert!(e.contains("中断した"), "{e}");
    assert_no_children();

    // abort（Instantiate）と segfault（DoStep）: 終了の状態が文面に入る
    // segfault は macOS では SIGBUS のこともある
    // #49: kill・クラッシュで終わっても、展開した FMU の一時ディレクトリが残らない
    let tag = format!("leak-{}", std::process::id());
    for (mode, signal) in [(1, "SIGABRT"), (4, "SIG")] {
        let e = run_in_worker(
            &globe_spec(),
            &request(&crash_fmu_tagged(mode, &tag).unwrap()),
            &mut |_| {},
        )
        .unwrap_err();
        let m = e.to_string();
        assert!(matches!(e, Error::Worker(_)), "{m}");
        assert!(m.contains("落ちた"), "{m}");
        if cfg!(unix) {
            assert!(m.contains(signal), "mode {mode}: {m}");
        }
        eprintln!("mode {mode}: {m}");
        assert_no_children();
        assert_eq!(
            leaked_unpacked(&tag),
            0,
            "mode {mode}: 展開した FMU が残っている"
        );
    }

    // 無限ループ: 時間の上限で kill される
    let mut spec = globe_spec();
    spec.timeout = Some(Duration::from_millis(1500));
    let started = Instant::now();
    let e = run_in_worker(
        &spec,
        &request(&crash_fmu_tagged(2, &tag).unwrap()),
        &mut |_| {},
    )
    .unwrap_err();
    assert!(matches!(e, Error::Worker(_)), "{e}");
    assert!(e.to_string().contains("終わらないので止めた"), "{e}");
    assert!(
        started.elapsed() < Duration::from_secs(10),
        "{:?}",
        started.elapsed()
    );
    eprintln!("無限ループ: {e}");
    assert_no_children();
    assert_eq!(leaked_unpacked(&tag), 0, "kill: 展開した FMU が残っている");

    // メモリの食いつぶし（Linux だけ。上限なしで回すとホストのメモリを食うので、上限つきでだけ回す）
    if cfg!(target_os = "linux") {
        let mut spec = globe_spec();
        spec.timeout = Some(Duration::from_secs(120)); // 保険
        spec.memory_limit = Some(1 << 30);
        // 同じ上限で普通の FMU が通る（globe 自身が上限に収まる）ことを先に確かめる
        run_in_worker(&spec, &request(&ok), &mut |_| {}).unwrap();
        let e = run_in_worker(&spec, &request(&crash_fmu(3).unwrap()), &mut |_| {}).unwrap_err();
        assert!(matches!(e, Error::Worker(_)), "{e}");
        assert!(e.to_string().contains("SIGABRT"), "{e}");
        eprintln!("メモリ: {e}");
        assert_no_children();
    }
}

/// CLI の `--timeout` と、落ちたときの終了コード 1・親の `globe` が生きていること。
#[test]
fn cli_reports_a_crash_and_honours_timeout() {
    let (Some(abort), Some(hang)) = (crash_fmu(1), crash_fmu(2)) else {
        return;
    };
    let globe = |args: &[&str]| {
        Command::new(env!("CARGO_BIN_EXE_globe"))
            .args(args)
            .output()
            .unwrap()
    };
    let o = globe(&["fmu", "run", abort.to_str().unwrap()]);
    let err = String::from_utf8_lossy(&o.stderr);
    assert_eq!(o.status.code(), Some(1), "{o:?}");
    assert!(
        err.contains("別プロセス") && err.contains("落ちた"),
        "{err}"
    );
    let started = Instant::now();
    let o = globe(&["fmu", "run", hang.to_str().unwrap(), "--timeout", "1"]);
    assert_eq!(o.status.code(), Some(1), "{o:?}");
    assert!(
        String::from_utf8_lossy(&o.stderr).contains("終わらないので止めた"),
        "{o:?}"
    );
    assert!(started.elapsed() < Duration::from_secs(10));
}

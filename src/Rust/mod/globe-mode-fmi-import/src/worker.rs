//! FMU を別のプロセス（ワーカー）で回す（S210、ADR-0026 の (n)）。
//!
//! FMU は任意のネイティブコードで、落ちる（segfault・abort）・止まらない・メモリを食いつぶすことがある。
//! 呼び出し側のプロセスの中で読み込むと、それごと落ちる。そこで **読み込みと実行を子プロセスに隔離し**、
//! 落ちたら終了の状態と標準エラーの末尾を添えた [`Error::Worker`] として受け取る。
//!
//! - 親（[`run_in_worker`]）: 呼び出し側が渡した実行ファイル（例: `globe` 自身）を、ワーカーの副命令で起動し、
//!   標準入力へ要求（[`WorkerRequest`] の JSON 1 行）を渡し、標準出力から結果を受け取る。時間の上限で kill する。
//! - 子（[`worker_main`]）: 呼び出し側の実行ファイルの副命令（`globe fmu __worker`）から呼ぶ。要求を読んで [`run_in_process`] を回す。
//!
//! 標準出力の約束: 1 行 1 メッセージで、行頭の印 `\u{1}GW ` の後に 1 文字の種類と本文。印の無い行は FMU が
//! `printf` したものとして捨てる（標準エラーの末尾と同じ所に溜め、落ちたときの文面に添える）。
//! `P 時刻`（進捗）、`H 変数名の JSON`、`R 時刻 値…`（空白区切り。f64 の `{:?}` なので NaN・inf も含め往復で同じ値）、
//! `M 要約の JSON`（ME だけ）、`D`（正常に終わった印。無いまま閉じたら落ちたと見なす）、`E 文面`（実行の失敗）。
//!
//! 隔離するのはプロセスのクラッシュ・時間・メモリ（Linux だけ）。**ディスクとネットワークは隔離しない**（sandbox は持ち越し）。

use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use crate::{CsOptions, Error, EventKind, Fmu, MeOptions, Result, run_cs_with, run_me_with};

const TAG: &str = "\u{1}GW ";
/// 落ちたときの文面に添える、標準エラーと印の無い標準出力の末尾のバイト数。
const TAIL: usize = 4096;
/// 子が進捗を流す間隔の下限。
const PROGRESS_EVERY: Duration = Duration::from_millis(250);

/// 回し方。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Kind {
    /// Co-Simulation
    Cs,
    /// Model Exchange
    Me,
}

/// ワーカーへの要求。[`CsOptions`]・[`MeOptions`] の共通部分（ME の積分器と許容誤差は既定）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WorkerRequest {
    pub file: PathBuf,
    /// 省略時は Co-Simulation を持てばそれ、無ければ Model Exchange。
    pub kind: Option<Kind>,
    pub start_time: Option<f64>,
    pub stop_time: Option<f64>,
    pub step: Option<f64>,
    pub outputs: Vec<String>,
    pub set: Vec<(String, f64)>,
    /// FMU の展開先。[`run_in_worker`] が親の側で一時ディレクトリを作って埋める（子が kill・クラッシュで終わっても、親が消す。#49）。
    /// 空なら [`run_in_process`] が自分で作って消す
    #[serde(default)]
    pub work_dir: Option<PathBuf>,
}

/// ME の要約（事象の中身は数だけ）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MeSummary {
    pub steps: u64,
    pub terminated_by_fmu: bool,
    pub state_events: usize,
    pub time_events: usize,
    pub step_events: usize,
}

/// 結果。`me` は Model Exchange を回したときだけ `Some`。
#[derive(Debug, Clone, PartialEq)]
pub struct WorkerRun {
    pub names: Vec<String>,
    pub rows: Vec<(f64, Vec<f64>)>,
    pub me: Option<MeSummary>,
}

/// どの実行ファイルをワーカーにするか、と上限。
#[derive(Debug, Clone)]
pub struct WorkerSpec {
    /// ワーカーの実行ファイル（例: `std::env::current_exe()`）。
    pub exe: PathBuf,
    /// その副命令の引数（例: `["fmu", "__worker"]`。中で [`worker_main`] を呼ぶ）。
    pub args: Vec<std::ffi::OsString>,
    /// 超えたら kill する。`None` は無し。
    pub timeout: Option<Duration>,
    /// 子の仮想メモリの上限（バイト。`ulimit -v`）。Linux だけ。ほかの OS で指定すると [`Error::Worker`]。
    pub memory_limit: Option<u64>,
}

impl WorkerSpec {
    pub fn new(exe: impl Into<PathBuf>, args: &[&str]) -> WorkerSpec {
        WorkerSpec {
            exe: exe.into(),
            args: args.iter().map(Into::into).collect(),
            timeout: None,
            memory_limit: None,
        }
    }
}

/// 要求を**このプロセスの中で**回す（ワーカーの中身。落ちると呼び出し側ごと落ちる）。
pub fn run_in_process(req: &WorkerRequest, progress: &mut dyn FnMut(f64)) -> Result<WorkerRun> {
    let fmu = match &req.work_dir {
        Some(dir) => Fmu::open_in(&req.file, dir)?,
        None => Fmu::open(&req.file)?,
    };
    let kind = req
        .kind
        .unwrap_or(if fmu.description.co_simulation.is_some() {
            Kind::Cs
        } else {
            Kind::Me
        });
    Ok(match kind {
        Kind::Cs => {
            let r = run_cs_with(
                &fmu,
                &CsOptions {
                    start_time: req.start_time,
                    stop_time: req.stop_time,
                    step: req.step,
                    outputs: req.outputs.clone(),
                    set: req.set.clone(),
                },
                progress,
            )?;
            WorkerRun {
                names: r.names,
                rows: r.rows,
                me: None,
            }
        }
        Kind::Me => {
            let r = run_me_with(
                &fmu,
                &MeOptions {
                    start_time: req.start_time,
                    stop_time: req.stop_time,
                    step: req.step,
                    outputs: req.outputs.clone(),
                    set: req.set.clone(),
                    ..Default::default()
                },
                progress,
            )?;
            let n = |k| r.events.iter().filter(|e| e.kind == k).count();
            WorkerRun {
                me: Some(MeSummary {
                    steps: r.steps,
                    terminated_by_fmu: r.terminated_by_fmu,
                    state_events: n(EventKind::State),
                    time_events: n(EventKind::Time),
                    step_events: n(EventKind::Step),
                }),
                names: r.names,
                rows: r.rows,
            }
        }
    })
}

// ---- 子の側 ----

fn line(tag: char, payload: &str) -> String {
    // 先頭の改行は、FMU が改行なしで printf した行の後ろでも印を行頭に置くため
    format!("\n{TAG}{tag} {payload}\n")
}

fn say(tag: char, payload: &str) {
    let mut out = std::io::stdout().lock();
    let _ = out.write_all(line(tag, payload).as_bytes());
    let _ = out.flush();
}

/// ワーカーの本体。標準入力の 1 行目の要求を読んで回し、結果を標準出力へ書く。終了コード（0＝結果を返した、2＝実行の失敗、3＝親が消えた）を返す。
///
/// 親が消えたら（標準入力が閉じたら）自分も終わる。親が SIGKILL で落ちたときに孤児が残らないように。
pub fn worker_main() -> i32 {
    let mut text = String::new();
    let request = std::io::stdin()
        .lock()
        .read_line(&mut text)
        .map_err(|e| e.to_string())
        .and_then(|_| serde_json::from_str::<WorkerRequest>(&text).map_err(|e| e.to_string()));
    let req = match request {
        Ok(r) => r,
        Err(e) => {
            say('E', &format!("ワーカーが要求を読めない: {e}"));
            return 2;
        }
    };
    std::thread::spawn(|| {
        let _ = std::io::copy(&mut std::io::stdin(), &mut std::io::sink());
        std::process::exit(3);
    });
    let mut last = Instant::now();
    let result = run_in_process(&req, &mut |t| {
        if last.elapsed() >= PROGRESS_EVERY {
            last = Instant::now();
            say('P', &format!("{t:?}"));
        }
    });
    match result {
        Err(e) => {
            say('E', &e.to_string().replace('\n', " "));
            2
        }
        Ok(run) => {
            say('H', &serde_json::to_string(&run.names).unwrap_or_default());
            if let Some(me) = &run.me {
                say('M', &serde_json::to_string(me).unwrap_or_default());
            }
            let mut out = std::io::BufWriter::with_capacity(1 << 16, std::io::stdout().lock());
            for (t, values) in &run.rows {
                let _ = write!(out, "{TAG}R {t:?}");
                for v in values {
                    let _ = write!(out, " {v:?}");
                }
                let _ = writeln!(out);
            }
            let _ = out.write_all(line('D', "").as_bytes());
            let _ = out.flush();
            0
        }
    }
}

// ---- 親の側 ----

/// drop で必ず kill して回収する（早く return しても zombie を残さない）。
struct Reaper(std::process::Child);

impl Drop for Reaper {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn push_tail(tail: &Mutex<Vec<u8>>, bytes: &[u8]) {
    let mut t = tail.lock().unwrap_or_else(|e| e.into_inner());
    t.extend_from_slice(bytes);
    if t.len() > TAIL {
        let cut = t.len() - TAIL;
        t.drain(..cut);
    }
}

fn parse_row(payload: &str) -> Option<(f64, Vec<f64>)> {
    let mut it = payload.split(' ').map(|c| c.parse::<f64>());
    let t = it.next()?.ok()?;
    Some((t, it.collect::<std::result::Result<_, _>>().ok()?))
}

/// `req` をワーカー（別プロセス）で回す。ワーカーが落ちた・時間切れ・起動できない・約束と違う出力は [`Error::Worker`]、
/// ワーカーの中の実行の失敗は [`Error::Child`]。`progress` は子が流す時刻（0.25 秒以上の間隔）を受け取る。
pub fn run_in_worker(
    spec: &WorkerSpec,
    req: &WorkerRequest,
    progress: &mut dyn FnMut(f64),
) -> Result<WorkerRun> {
    let worker = |m: String| Error::Worker(m);
    let mut cmd;
    match spec.memory_limit {
        // shortcut: `sh` の ulimit で上限を付ける（unsafe も libc も要らない）。Linux 以外は未対応、必要になったら setrlimit / Job Object へ
        Some(bytes) if cfg!(target_os = "linux") => {
            cmd = Command::new("sh");
            cmd.arg("-c")
                .arg(r#"ulimit -v "$0" && exec "$@""#)
                .arg((bytes / 1024).max(1).to_string())
                .arg(&spec.exe);
        }
        Some(_) => return Err(worker("メモリの上限は Linux だけで使える".into())),
        None => cmd = Command::new(&spec.exe),
    }
    // #49: 展開先は親が作る。子が SIGKILL・segfault・exit で終わっても drop が走らず残る、ということが無い（`work` は child より先に宣言したので、wait のあとで落ちる）
    let work = tempfile::Builder::new()
        .prefix("globe-fmu-")
        .tempdir()
        .map_err(|e| worker(format!("一時ディレクトリを作れない: {e}")))?;
    let mut child = Reaper(
        cmd.args(&spec.args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| worker(format!("{} を起動できない: {e}", spec.exe.display())))?,
    );
    let tail = Arc::new(Mutex::new(Vec::new()));
    // 親が消えたら子が自分で終われるよう、終わるまで標準入力を開けておく
    let mut stdin = child.0.stdin.take();
    let request = WorkerRequest {
        work_dir: Some(work.path().to_owned()),
        ..req.clone()
    };
    let request = serde_json::to_string(&request).map_err(|e| worker(e.to_string()))?;
    if let Some(i) = stdin.as_mut() {
        // 子が既に死んでいて書けないときは、下で終了の状態を報告する
        let _ = writeln!(i, "{request}");
        let _ = i.flush();
    }
    let (mut stdout, mut stderr) = (
        child
            .0
            .stdout
            .take()
            .ok_or_else(|| worker("標準出力を取れない".into()))?,
        child
            .0
            .stderr
            .take()
            .ok_or_else(|| worker("標準エラーを取れない".into()))?,
    );
    let err_thread = {
        let tail = Arc::clone(&tail);
        std::thread::spawn(move || {
            let mut buf = [0u8; 4096];
            while let Ok(n @ 1..) = std::io::Read::read(&mut stderr, &mut buf) {
                push_tail(&tail, &buf[..n]);
            }
        })
    };
    let (tx, rx) = mpsc::channel::<String>();
    {
        let tail = Arc::clone(&tail);
        std::thread::spawn(move || {
            let mut r = BufReader::new(&mut stdout);
            let mut buf = Vec::new();
            loop {
                buf.clear();
                if !matches!(r.read_until(b'\n', &mut buf), Ok(1..)) {
                    break;
                }
                let text = String::from_utf8_lossy(&buf);
                match text.find(TAG) {
                    Some(i) => {
                        push_tail(&tail, text[..i].as_bytes());
                        if tx
                            .send(text[i + TAG.len()..].trim_end().to_owned())
                            .is_err()
                        {
                            break;
                        }
                    }
                    None => push_tail(&tail, &buf),
                }
            }
        });
    }

    let deadline = spec.timeout.map(|t| Instant::now() + t);
    let (mut names, mut rows, mut me) = (None, Vec::new(), None);
    let (mut done, mut failed, mut last_t) = (false, None, None);
    let mut timed_out = false;
    loop {
        let msg = match deadline {
            Some(d) => rx.recv_timeout(d.saturating_duration_since(Instant::now())),
            None => rx.recv().map_err(|_| RecvTimeoutError::Disconnected),
        };
        let text = match msg {
            Ok(t) => t,
            Err(RecvTimeoutError::Timeout) => {
                timed_out = true;
                break;
            }
            Err(RecvTimeoutError::Disconnected) => break,
        };
        let (kind, payload) = text.split_at(text.len().min(1));
        let payload = payload.trim_start();
        let bad = || worker(format!("ワーカーが読めない行を返した: {text:?}"));
        match kind {
            "P" => {
                let t: f64 = payload.parse().map_err(|_| bad())?;
                last_t = Some(t);
                progress(t);
            }
            "H" => names = Some(serde_json::from_str(payload).map_err(|_| bad())?),
            "M" => me = Some(serde_json::from_str(payload).map_err(|_| bad())?),
            "R" => rows.push(parse_row(payload).ok_or_else(bad)?),
            "D" => {
                done = true;
                break;
            }
            "E" => {
                failed = Some(payload.to_owned());
                break;
            }
            _ => return Err(bad()),
        }
    }

    // 回収。結果を受け取ったなら自分で終わるのを少し待ち、それ以外は止める（kill してから wait。終わっていた子には無害）
    let grace = Instant::now() + Duration::from_secs(5);
    drop(stdin);
    while (done || failed.is_some()) && Instant::now() < grace {
        if matches!(child.0.try_wait(), Ok(Some(_))) {
            break;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    let _ = child.0.kill();
    let status = child.0.wait().map_err(|e| worker(e.to_string()))?;
    // shortcut: 子の孫が管を握り続けると join が戻らない。FMU が子プロセスを作る場合は Job Object / プロセスグループへ
    let _ = err_thread.join();
    let tail_text = {
        let t = tail.lock().unwrap_or_else(|e| e.into_inner());
        String::from_utf8_lossy(&t).trim().to_owned()
    };
    let with_tail = |m: String| {
        if tail_text.is_empty() {
            m
        } else {
            format!("{m}\n--- 標準エラーなどの末尾 ---\n{tail_text}")
        }
    };
    if let Some(m) = failed {
        return Err(Error::Child(m));
    }
    if timed_out {
        let at = last_t.map_or(String::new(), |t| format!("（最後の進捗は t = {t}）"));
        return Err(worker(with_tail(format!(
            "{:.3} s で終わらないので止めた{at}",
            spec.timeout.unwrap_or_default().as_secs_f64()
        ))));
    }
    if !done {
        return Err(worker(with_tail(format!(
            "結果を返す前に終わった（{status}）。FMU が落ちたとみられる"
        ))));
    }
    Ok(WorkerRun {
        names: names.ok_or_else(|| worker("名前の行が無い".into()))?,
        rows,
        me,
    })
}

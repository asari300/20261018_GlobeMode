//! sidecar の `globe lsp` との stdio の JSON-RPC（ADR-0023 の (a)(b)）。tauri に依存しない。
//!
//! - 要求は 1 度に 1 つ（`Worker` を `Mutex` に入れて呼ぶ。待たされた要求は順に処理される）。
//! - 予算（`globe/simulate` は 180 s）を超えたら、プロセスを kill する。次の要求で作り直し、
//!   開いているファイル（`textDocument/didOpen`・`didChange`、`globe/simulate` の `files`）を再生する。
//! - 落ちた（panic・OOM）ときも同じ。その要求は Err を返し、次の要求で作り直す。
//! - 通知（`publishDiagnostics` など）は読み取りのスレッドから `Sink` へ渡す。

use std::collections::BTreeMap;
use std::io::{self, BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::Arc;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::thread;
use std::time::{Duration, Instant};

use serde_json::{Value, json};

/// サーバからの通知の渡し先。
pub type Sink = Arc<dyn Fn(Value) + Send + Sync>;

/// 実行の予算（ADR-0017・ADR-0023）。検査には予算を付けない（ADR-0018）
const SIMULATE_BUDGET: Duration = Duration::from_secs(180);
/// `initialize` の応答まで（ライブラリを読む。MSL で約 3 s、遅い機械の余裕を見る）
const START_BUDGET: Duration = Duration::from_secs(300);

pub struct Worker {
    program: PathBuf,
    libraries: Vec<PathBuf>,
    sink: Sink,
    live: Option<Live>,
    /// 開いているファイル（URI → 本文）。作り直したときに送り直す
    docs: BTreeMap<String, String>,
    next_id: u64,
    simulate_budget: Duration,
    start_budget: Duration,
}

struct Live {
    child: Child,
    stdin: ChildStdin,
    /// 応答（`id` があって `method` が無いメッセージ）
    rx: Receiver<Value>,
}

impl Drop for Live {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

impl Worker {
    pub fn new(program: PathBuf, libraries: Vec<PathBuf>, sink: Sink) -> Self {
        Worker {
            program,
            libraries,
            sink,
            live: None,
            docs: BTreeMap::new(),
            next_id: 0,
            simulate_budget: SIMULATE_BUDGET,
            start_budget: START_BUDGET,
        }
    }

    /// sidecar の pid（動いていなければ None）。
    pub fn pid(&self) -> Option<u32> {
        self.live.as_ref().map(|l| l.child.id())
    }

    /// 動いていなければ立ち上げ、`initialize` を済ませ、開いているファイルを送り直す。
    pub fn ensure_live(&mut self) -> Result<(), String> {
        if let Some(l) = &mut self.live {
            if matches!(l.child.try_wait(), Ok(None)) {
                return Ok(());
            }
            self.live = None;
        }
        self.live = Some(
            self.spawn()
                .map_err(|e| format!("{}: {e}", self.program.display()))?,
        );
        let budget = Some(self.start_budget);
        self.exchange("initialize", json!({"capabilities": {}}), budget)?;
        self.send(&json!({"jsonrpc": "2.0", "method": "initialized", "params": {}}))?;
        for (uri, text) in self.docs.clone() {
            self.send(&did_open(&uri, &text))?;
        }
        Ok(())
    }

    /// 要求を 1 つ送り、応答の `result` を返す（JSON-RPC のエラーは `message` を Err にする）。
    pub fn request(&mut self, method: &str, params: Value) -> Result<Value, String> {
        self.ensure_live()?;
        let budget = (method == "globe/simulate").then_some(self.simulate_budget);
        if method == "globe/simulate" {
            // `files` はサーバのワークスペースに残る。作り直しで失わないよう覚える
            if let Some(files) = params["request"]["files"].as_object() {
                for (name, text) in files {
                    self.docs
                        .insert(name.clone(), text.as_str().unwrap_or("").to_string());
                }
            }
        }
        self.exchange(method, params, budget)
    }

    /// 通知を送る（応答は無い）。ファイルの開閉は覚える。
    pub fn notify(&mut self, method: &str, params: Value) -> Result<(), String> {
        self.ensure_live()?;
        let uri = params["textDocument"]["uri"]
            .as_str()
            .unwrap_or("")
            .to_string();
        match method {
            "textDocument/didOpen" => {
                let text = params["textDocument"]["text"].as_str().unwrap_or("");
                self.docs.insert(uri, text.to_string());
            }
            "textDocument/didChange" => {
                let text = params["contentChanges"].as_array().and_then(|a| a.last());
                let text = text.and_then(|c| c["text"].as_str()).unwrap_or("");
                self.docs.insert(uri, text.to_string());
            }
            "textDocument/didClose" => {
                self.docs.remove(&uri);
            }
            _ => {}
        }
        self.send(&json!({"jsonrpc": "2.0", "method": method, "params": params}))
    }

    fn spawn(&self) -> io::Result<Live> {
        let mut cmd = Command::new(&self.program);
        cmd.arg("lsp");
        for lib in &self.libraries {
            cmd.arg("--library").arg(lib);
        }
        cmd.stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit());
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW: コンソールを出さない
        }
        let mut child = cmd.spawn()?;
        let stdin = child.stdin.take().expect("piped");
        let mut out = BufReader::new(child.stdout.take().expect("piped"));
        let (tx, rx) = mpsc::channel();
        let sink = self.sink.clone();
        thread::spawn(move || {
            while let Some(m) = read_frame(&mut out) {
                if m.get("method").is_some() {
                    sink(m);
                } else if tx.send(m).is_err() {
                    break;
                }
            }
        });
        Ok(Live { child, stdin, rx })
    }

    fn send(&mut self, msg: &Value) -> Result<(), String> {
        let body = msg.to_string();
        let live = self.live.as_mut().ok_or("sidecar が動いていない")?;
        let sent = write!(live.stdin, "Content-Length: {}\r\n\r\n{body}", body.len())
            .and_then(|()| live.stdin.flush());
        sent.map_err(|e| {
            self.live = None;
            format!("sidecar に書けない（次の要求で作り直す）: {e}")
        })
    }

    /// 要求を送って、同じ `id` の応答まで待つ。予算を超えたら kill する（次の要求で作り直す）。
    fn exchange(
        &mut self,
        method: &str,
        params: Value,
        budget: Option<Duration>,
    ) -> Result<Value, String> {
        self.next_id += 1;
        let id = self.next_id;
        self.send(&json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params}))?;
        let deadline = budget.map(|b| Instant::now() + b);
        loop {
            let rx = &self.live.as_ref().ok_or("sidecar が動いていない")?.rx;
            let got = match deadline {
                Some(d) => rx.recv_timeout(d.saturating_duration_since(Instant::now())),
                None => rx.recv().map_err(|_| RecvTimeoutError::Disconnected),
            };
            match got {
                Ok(m) if m["id"] == id => {
                    return match m.get("error") {
                        Some(e) => Err(e["message"].as_str().unwrap_or("error").to_string()),
                        None => Ok(m["result"].clone()),
                    };
                }
                Ok(_) => {} // 前の要求の遅れた応答は捨てる
                Err(e) => {
                    self.live = None; // kill（Drop）。次の要求で作り直す
                    return Err(match e {
                        RecvTimeoutError::Timeout => format!(
                            "{method}: 予算 {} s を超えたので sidecar を止めた（次の要求で作り直す）",
                            budget.map_or(0, |b| b.as_secs())
                        ),
                        RecvTimeoutError::Disconnected => {
                            format!("{method}: sidecar が落ちた（次の要求で作り直す）")
                        }
                    });
                }
            }
        }
    }
}

fn did_open(uri: &str, text: &str) -> Value {
    json!({"jsonrpc": "2.0", "method": "textDocument/didOpen", "params": {"textDocument": {
        "uri": uri, "languageId": "modelica", "version": 1, "text": text}}})
}

/// `Content-Length` の枠 1 つ。入力の終わりと壊れた枠は None。
fn read_frame(input: &mut impl BufRead) -> Option<Value> {
    let mut len = None;
    loop {
        let mut line = String::new();
        if input.read_line(&mut line).ok()? == 0 {
            return None;
        }
        let line = line.trim_end();
        if line.is_empty() {
            break;
        }
        if let Some((k, v)) = line.split_once(':')
            && k.eq_ignore_ascii_case("content-length")
        {
            len = v.trim().parse::<usize>().ok();
        }
    }
    let mut buf = vec![0; len?];
    input.read_exact(&mut buf).ok()?;
    serde_json::from_slice(&buf).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    const MODEL: &str =
        "model D\n  Real x(start = 1, fixed = true);\nequation\n  der(x) = -x;\nend D;\n";

    fn collecting() -> (Sink, Arc<Mutex<Vec<Value>>>) {
        let got = Arc::new(Mutex::new(Vec::new()));
        let g = got.clone();
        (Arc::new(move |m| g.lock().unwrap().push(m)), got)
    }

    /// 本物の `globe`（環境変数 GLOBE_BIN。desktop.yml が設定する）に check・simulate・graph の誤りを頼む。
    #[test]
    fn talks_to_globe_lsp_and_recovers_from_a_kill() {
        let Some(bin) = std::env::var_os("GLOBE_BIN") else {
            eprintln!("GLOBE_BIN が無いので省く");
            return;
        };
        let (sink, got) = collecting();
        let mut w = Worker::new(bin.into(), vec![], sink);
        w.notify(
            "textDocument/didOpen",
            json!({"textDocument": {"uri": "D.mo", "languageId": "modelica", "version": 1, "text": MODEL}}),
        )
        .unwrap();
        let r = w.request("globe/check", json!({"model": "D"})).unwrap();
        assert_eq!(r["passed"], true);
        let r = w
            .request(
                "globe/simulate",
                json!({"request": {"model": "D", "no_csv": true}}),
            )
            .unwrap();
        assert_eq!(r["ok"], true);
        assert!(w.request("globe/graph", json!({"model": "Nope"})).is_err());
        assert!(
            got.lock()
                .unwrap()
                .iter()
                .any(|m| m["method"] == "textDocument/publishDiagnostics")
        );

        // 外から殺されても、次の要求で作り直し、開いていたファイルを再生する
        let old = w.pid().unwrap();
        w.live = None;
        let r = w.request("globe/check", json!({"model": "D"})).unwrap();
        assert_eq!(r["passed"], true);
        assert_ne!(w.pid().unwrap(), old);
    }

    /// 応答しない sidecar は予算で止め、次の要求でまた立ち上げようとする。
    #[cfg(unix)]
    #[test]
    fn budget_kills_a_silent_sidecar() {
        use std::os::unix::fs::PermissionsExt;
        let dir = std::env::temp_dir().join(format!("globe-desktop-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let script = dir.join("silent");
        std::fs::write(&script, "#!/bin/sh\ncat >/dev/null\n").unwrap();
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
        let (sink, _) = collecting();
        let mut w = Worker::new(script, vec![], sink);
        w.start_budget = Duration::from_millis(300);
        let e = w.request("globe/check", json!({"model": "D"})).unwrap_err();
        assert!(e.contains("予算"), "{e}");
        assert!(w.pid().is_none());
        // 2 回目も立ち上げ直して、また予算で止まる
        assert!(w.request("globe/check", json!({"model": "D"})).is_err());
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn frames_are_read_by_content_length() {
        let body = r#"{"id":1,"method":"x"}"#;
        let raw = format!("Content-Length: {}\r\n\r\n{body}", body.len());
        let mut input = raw.as_bytes();
        assert_eq!(read_frame(&mut input).unwrap()["method"], "x");
        assert!(read_frame(&mut input).is_none());
    }
}

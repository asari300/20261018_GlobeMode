//! `cargo xtask omc-compare`: `msl-funnel --json` の結果を omc（OpenModelica のコンパイラ）と突き合わせる（S008）。
//!
//! - omc は**別プロセス**で呼ぶ（[CLAUDE.md](../../../../CLAUDE.md) の 3。リンクもコードの複製もしない）。
//!   N 本ごとに 1 つの `.mos` スクリプトを書き、`omc script.mos` を実行して標準出力を読む。
//!   MSL は omc 同梱の `Modelica 4.1.0+maint.om` ではなく、`data/msl` の素の 4.1.0（globe check と同じもの）を読ませる。
//! - 1 本ごとに `checkModel`（方程式数と変数数。S5 の照合）と `translateModel`（バックエンドまで。構造特異と初期条件の過剰の照合）を呼ぶ。
//!   `translateModel` を `print(String(...))` のように式の中に入れると失敗時の `getErrorString()` が空になるので、文の形で呼び、印だけを `print` する。
//! - omc が途中で落ちたり時間切れになったりしたら、印の無いモデルを 1 本ずつ再実行し、それでも駄目なら「omc 異常終了」にして続ける。
//! - 照合の規則は [ADR-0010](../../../../doc/adr/0010-omc-comparison.md)。

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::fs;
use std::path::Path;
use std::process::{Command, ExitCode, Stdio};
use std::time::{Duration, Instant};

use serde_json::{Value, json};

use crate::{MSL_VERSION, Result};

/// `omc-compare` の引数。
#[derive(Debug)]
pub(crate) struct Options<'a> {
    pub msl: &'a Path,
    /// `msl-funnel --json` が書いたファイル
    pub funnel_json: &'a Path,
    /// omc の実行ファイル（名前かパス）
    pub omc: &'a str,
    pub report: Option<&'a Path>,
    /// omc の生の結果と照合の結果を書く JSON
    pub json: Option<&'a Path>,
    /// omc を呼ばず、前に `--json` で書いた結果を読んで照合だけやり直す
    pub reuse: Option<&'a Path>,
    /// 1 つの omc プロセスで検査する本数
    pub chunk: usize,
    /// omc プロセス 1 つの制限時間（秒）
    pub timeout_secs: u64,
    pub filter: Option<&'a str>,
    pub limit: Option<usize>,
    /// スクリプトと omc の生成物を置く場所
    pub work_dir: &'a Path,
    /// `translateModel` を呼ばない（`checkModel` だけ。S5 の照合だけなら速い）
    pub no_translate: bool,
}

// --------------------------------------------------------------------------- globe check の結果（msl-funnel の JSON）

/// `msl-funnel --json` の 1 本分のうち、照合に使う部分。
#[derive(Debug, Clone)]
pub(crate) struct Ours {
    pub model: String,
    pub partial: bool,
    /// `Ok` / `Ng` / `Unsupported` / `Panic`
    pub verdict: String,
    /// 止まった段階（0〜8）
    pub stage: Option<u8>,
    pub gm_code: Option<String>,
    pub rumoca_code: Option<String>,
    /// (方程式数, 未知数)。どちらもスカラーの総数（globe check の S5 の数え方）
    pub counts: Option<(usize, usize)>,
    pub s6_regular: Option<bool>,
    pub s7_index: Option<usize>,
    pub s8: Option<OursS8>,
}

#[derive(Debug, Clone)]
pub(crate) struct OursS8 {
    pub required: usize,
    pub provided: usize,
    /// 初期化系をマッチングできた（S009）。そのときは `extra`（冗長）と `missing`（決まらない）が確か
    pub matched: bool,
    pub extra: usize,
    pub missing: usize,
    /// `合う` / `過剰` / `不足`
    pub balance: String,
    pub uncertain: bool,
}

impl OursS8 {
    /// 過剰（冗長な初期条件がある）。マッチングがあれば `extra > 0`、無ければ与えた数 > 要る数。
    pub(crate) fn is_over(&self) -> bool {
        if self.matched {
            self.extra > 0
        } else {
            self.provided > self.required
        }
    }
}

impl Ours {
    /// S5 の判定。`Some(true)` は方程式数と未知数が合う（S5 を通った）、`Some(false)` は合わない（S5 で止まった）、
    /// `None` は S5 の判定が無い（それより前で止まった、GM-U、panic）。
    pub(crate) fn s5_balanced(&self) -> Option<bool> {
        match self.verdict.as_str() {
            "Ok" => Some(true),
            "Ng" => match self.stage {
                Some(5) => Some(false),
                Some(s) if s > 5 => Some(true),
                _ => None,
            },
            _ => None,
        }
    }

    fn label(&self) -> String {
        match self.verdict.as_str() {
            "Ok" => "OK".to_string(),
            "Ng" => format!(
                "NG S{} {}",
                self.stage.map(|s| s.to_string()).unwrap_or_default(),
                self.gm_code.as_deref().unwrap_or("-")
            ),
            "Unsupported" => format!(
                "{} {}",
                self.gm_code.as_deref().unwrap_or("GM-U"),
                self.rumoca_code.as_deref().unwrap_or("-")
            ),
            other => other.to_string(),
        }
    }
}

fn read_funnel_json(path: &Path) -> Result<Vec<Ours>> {
    let text = fs::read_to_string(path).map_err(|e| {
        format!(
            "{} を読めない: {e}。先に `cargo xtask msl-funnel --json {}` を実行する",
            path.display(),
            path.display()
        )
    })?;
    let root: Value = serde_json::from_str(&text)
        .map_err(|e| format!("{} の JSON が壊れている: {e}", path.display()))?;
    let models = root
        .get("models")
        .and_then(Value::as_array)
        .ok_or_else(|| format!("{} に models が無い", path.display()))?;
    Ok(models.iter().map(ours_from_json).collect())
}

fn ours_from_json(v: &Value) -> Ours {
    let s = |k: &str| v.get(k).and_then(Value::as_str).map(str::to_string);
    let stage = s("stage").and_then(|t| t.trim_start_matches('S').parse::<u8>().ok());
    // 方程式数と未知数はどちらもスカラーの総数（msl-funnel の `counts.unknowns` は S008 から。無い JSON では数を比べない）
    let counts = v.get("counts").filter(|c| !c.is_null()).and_then(|c| {
        let n = |k: &str| c.get(k).and_then(Value::as_u64).map(|x| x as usize);
        Some((n("equations")?, n("unknowns")?))
    });
    let s6_regular = v
        .get("s6")
        .and_then(|x| x.get("regular"))
        .and_then(Value::as_bool);
    let s7_index = v
        .get("s7")
        .and_then(|x| x.get("index"))
        .and_then(Value::as_u64)
        .map(|i| i as usize);
    let s8 = v.get("s8").filter(|x| !x.is_null()).map(|x| OursS8 {
        required: x.get("required").and_then(Value::as_u64).unwrap_or(0) as usize,
        provided: x.get("provided").and_then(Value::as_u64).unwrap_or(0) as usize,
        matched: x.get("matched").and_then(Value::as_bool).unwrap_or(false),
        extra: x.get("extra").and_then(Value::as_u64).unwrap_or(0) as usize,
        missing: x.get("missing").and_then(Value::as_u64).unwrap_or(0) as usize,
        balance: x
            .get("balance")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string(),
        uncertain: x.get("uncertain").and_then(Value::as_bool).unwrap_or(false),
    });
    Ours {
        model: s("model").unwrap_or_default(),
        partial: v.get("partial").and_then(Value::as_bool).unwrap_or(false),
        verdict: s("verdict").unwrap_or_default(),
        stage,
        gm_code: s("gm_code"),
        rumoca_code: s("rumoca_code"),
        counts,
        s6_regular,
        s7_index,
        s8,
    }
}

// --------------------------------------------------------------------------- omc の結果

/// `checkModel` の結果。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct OmcCheck {
    /// 「completed successfully」が出た
    pub ok: bool,
    pub equations: Option<usize>,
    pub variables: Option<usize>,
    pub trivial: Option<usize>,
    /// `checkModel` が返した文字列
    pub text: String,
    /// その後の `getErrorString()`
    pub errors: String,
}

impl OmcCheck {
    /// 方程式数と変数数が合う。数が無ければ `None`。
    pub(crate) fn balanced(&self) -> Option<bool> {
        Some(self.equations? == self.variables?)
    }
}

/// `translateModel` の失敗の種類。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TranslateKind {
    /// 通った（警告はあってもよい）
    Ok,
    /// 構造特異（Pantelides 法の失敗を含む）
    Singular,
    /// 部分系の方程式数と変数数が合わない
    Imbalanced,
    /// それ以外の失敗（未対応の関数、外部コードなど）
    Other,
}

impl TranslateKind {
    fn label(self) -> &'static str {
        match self {
            TranslateKind::Ok => "通る",
            TranslateKind::Singular => "特異",
            TranslateKind::Imbalanced => "部分系が不均衡",
            TranslateKind::Other => "失敗",
        }
    }
}

/// `translateModel` の結果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OmcTranslate {
    pub ok: bool,
    pub kind: TranslateKind,
    pub errors: String,
    /// 「The initial conditions are over specified」が出た。`-d=initialization` なら余分な初期方程式の本数も分かる
    pub over_specified: Option<Option<usize>>,
}

/// 1 本分の omc の結果。
#[derive(Debug, Clone, Default)]
pub(crate) struct OmcModel {
    pub model: String,
    pub check: Option<OmcCheck>,
    pub translate: Option<OmcTranslate>,
    /// 印（`@@END`）まで出力が無かった（omc が落ちた、または時間切れ）
    pub crashed: bool,
}

/// `.mos` スクリプトを組み立てる。
pub(crate) fn script(msl: &Path, models: &[&str], translate: bool) -> String {
    let mut s = String::new();
    let _ = writeln!(
        s,
        "setModelicaPath(\"{}\");",
        msl.display().to_string().replace('\\', "/")
    );
    let _ = writeln!(
        s,
        "loadModel(Modelica, {{\"{MSL_VERSION}\"}}); getErrorString();"
    );
    // 初期条件の過剰で、余分な初期方程式の本数を出させる
    let _ = writeln!(s, "setCommandLineOptions(\"-d=initialization\");");
    let _ = writeln!(s, "print(\"@@LOADED\\n\");");
    for m in models {
        let _ = writeln!(s, "print(\"@@BEGIN {m}\\n\");");
        let _ = writeln!(s, "checkModel({m});");
        let _ = writeln!(s, "print(\"@@ERR\\n\"); getErrorString();");
        if translate {
            let _ = writeln!(s, "print(\"@@TRANSLATE\\n\");");
            let _ = writeln!(s, "translateModel({m});");
            let _ = writeln!(s, "print(\"@@ERR\\n\"); getErrorString();");
        }
        let _ = writeln!(s, "print(\"@@END\\n\");");
    }
    s
}

/// omc の echo（`"..."`）を外す。中の `\"` と `\\` を戻す。
pub(crate) fn unquote(s: &str) -> String {
    let t = s.trim();
    if t.len() >= 2 && t.starts_with('"') && t.ends_with('"') {
        let inner = &t[1..t.len() - 1];
        let mut out = String::with_capacity(inner.len());
        let mut chars = inner.chars();
        while let Some(c) = chars.next() {
            if c == '\\' {
                match chars.next() {
                    Some('"') => out.push('"'),
                    Some('\\') => out.push('\\'),
                    Some('n') => out.push('\n'),
                    Some(o) => {
                        out.push('\\');
                        out.push(o);
                    }
                    None => out.push('\\'),
                }
            } else {
                out.push(c);
            }
        }
        out
    } else {
        t.to_string()
    }
}

/// 「… has N equation(s) and M variable(s).」から数を読む。
fn parse_counts(text: &str) -> (Option<usize>, Option<usize>, Option<usize>) {
    let number_before = |hay: &str, marker: &str| -> Option<usize> {
        let end = hay.find(marker)?;
        let head = &hay[..end];
        let start = head
            .rfind(|c: char| !c.is_ascii_digit())
            .map_or(0, |i| i + 1);
        head[start..].parse().ok()
    };
    let equations = number_before(text, " equation(s) and ");
    let variables = number_before(text, " variable(s)");
    let trivial = number_before(text, " of these are trivial");
    (equations, variables, trivial)
}

pub(crate) fn parse_check(text: &str, errors: &str) -> OmcCheck {
    let (equations, variables, trivial) = parse_counts(text);
    OmcCheck {
        ok: text.contains("completed successfully"),
        equations,
        variables,
        trivial,
        text: text.to_string(),
        errors: errors.to_string(),
    }
}

pub(crate) fn classify_translate(ok: bool, errors: &str) -> TranslateKind {
    if ok {
        return TranslateKind::Ok;
    }
    let e = errors;
    if e.contains("structurally singular")
        || e.contains("pantelidesIndexReduction failed")
        || e.contains("Method Pantelides failed")
        || e.contains("error found sorting equations")
    {
        TranslateKind::Singular
    } else if e.contains("imbalanced number of equations")
        || e.contains("Too many equations")
        || e.contains("Too few equations")
    {
        TranslateKind::Imbalanced
    } else {
        TranslateKind::Other
    }
}

fn parse_over_specified(errors: &str) -> Option<Option<usize>> {
    if !errors.contains("initial conditions are over specified") {
        return None;
    }
    let count = errors.find("The following ").and_then(|i| {
        let rest = &errors[i + "The following ".len()..];
        let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
        if rest[digits.len()..].starts_with(" initial equation") {
            digits.parse().ok()
        } else {
            None
        }
    });
    Some(count)
}

pub(crate) fn parse_translate(result: &str, errors: &str) -> OmcTranslate {
    // `-d=initialization` を付けると、結果の `true` / `false` の前に反復変数の一覧などが標準出力に出る。結果は最後の行
    let ok = result.lines().map(str::trim).rfind(|l| !l.is_empty()) == Some("true");
    OmcTranslate {
        ok,
        kind: classify_translate(ok, errors),
        errors: errors.to_string(),
        over_specified: parse_over_specified(errors),
    }
}

/// omc の標準出力を印で切って、モデルごとの結果にする。`@@END` まで無いモデルは `crashed`。
pub(crate) fn parse_output(text: &str) -> Vec<OmcModel> {
    let mut out = Vec::new();
    for block in text.split("@@BEGIN ").skip(1) {
        let (model, rest) = block.split_once('\n').unwrap_or((block, ""));
        let model = model.trim().to_string();
        let mut m = OmcModel {
            model,
            ..Default::default()
        };
        let Some((body, _)) = rest.split_once("@@END\n") else {
            m.crashed = true;
            // 途中まで読めていれば残す
            if let Some((check, after)) = rest.split_once("@@ERR\n") {
                let errors = after.split("@@TRANSLATE\n").next().unwrap_or("");
                m.check = Some(parse_check(&unquote(check), &unquote(errors)));
            }
            out.push(m);
            continue;
        };
        let (check_part, translate_part) = body
            .split_once("@@TRANSLATE\n")
            .map_or((body, None), |(a, b)| (a, Some(b)));
        let (check, check_err) = check_part.split_once("@@ERR\n").unwrap_or((check_part, ""));
        m.check = Some(parse_check(&unquote(check), &unquote(check_err)));
        if let Some(t) = translate_part {
            let (result, err) = t.split_once("@@ERR\n").unwrap_or((t, ""));
            m.translate = Some(parse_translate(&unquote(result), &unquote(err)));
        }
        out.push(m);
    }
    out
}

/// omc を 1 回実行する。標準出力（と標準エラー）を文字列で返す。制限時間を超えたら kill して `timed_out = true`。
struct Run {
    output: String,
    timed_out: bool,
    exit: Option<i32>,
    seconds: f64,
}

fn run_omc(omc: &str, script_path: &Path, cwd: &Path, timeout: Duration) -> Result<Run> {
    let started = Instant::now();
    let out_path = script_path.with_extension("out");
    let out_file = fs::File::create(&out_path)
        .map_err(|e| format!("{} を作れない: {e}", out_path.display()))?;
    let err_file = out_file.try_clone().map_err(|e| e.to_string())?;
    let mut child = Command::new(omc)
        .arg(script_path)
        .current_dir(cwd)
        .stdin(Stdio::null())
        .stdout(Stdio::from(out_file))
        .stderr(Stdio::from(err_file))
        .spawn()
        .map_err(|e| {
            format!(
                "`{omc}` を起動できない: {e}（OpenModelica を入れて PATH に通すか --omc で指す）"
            )
        })?;
    let mut timed_out = false;
    let exit = loop {
        match child.try_wait().map_err(|e| e.to_string())? {
            Some(status) => break status.code(),
            None if started.elapsed() > timeout => {
                let _ = child.kill();
                let _ = child.wait();
                timed_out = true;
                break None;
            }
            None => std::thread::sleep(Duration::from_millis(100)),
        }
    };
    let output = fs::read_to_string(&out_path).unwrap_or_default();
    Ok(Run {
        output,
        timed_out,
        exit,
        seconds: started.elapsed().as_secs_f64(),
    })
}

fn omc_version(omc: &str) -> Result<String> {
    let out = Command::new(omc)
        .arg("--version")
        .output()
        .map_err(|e| format!("`{omc} --version` を実行できない: {e}（OpenModelica を入れて PATH に通すか --omc で指す）"))?;
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

/// 生成物（translateModel が書く C ソースなど）を消して作り直す。
fn fresh_dir(dir: &Path) -> Result<()> {
    if dir.exists() {
        fs::remove_dir_all(dir).map_err(|e| format!("{} を消せない: {e}", dir.display()))?;
    }
    fs::create_dir_all(dir).map_err(|e| format!("{} を作れない: {e}", dir.display()))
}

/// モデルの一覧を omc で検査する。chunk 本ごとに 1 プロセス。落ちたモデルは 1 本ずつやり直す。
fn run_all(
    opts: &Options<'_>,
    models: &[String],
) -> Result<(BTreeMap<String, OmcModel>, RunStats)> {
    let msl = fs::canonicalize(opts.msl).map_err(|e| format!("{}: {e}", opts.msl.display()))?;
    fs::create_dir_all(opts.work_dir)
        .map_err(|e| format!("{} を作れない: {e}", opts.work_dir.display()))?;
    // omc はスクリプトのパスを cwd（生成物の置き場）から解決するので、絶対パスにしておく
    let work_dir =
        fs::canonicalize(opts.work_dir).map_err(|e| format!("{}: {e}", opts.work_dir.display()))?;
    let gen_dir = work_dir.join("gen");
    let chunk = opts.chunk.max(1);
    let mut results: BTreeMap<String, OmcModel> = BTreeMap::new();
    let mut stats = RunStats::default();
    let chunks: Vec<&[String]> = models.chunks(chunk).collect();
    for (i, group) in chunks.iter().enumerate() {
        let names: Vec<&str> = group.iter().map(String::as_str).collect();
        let script_path = work_dir.join(format!("chunk-{i:03}.mos"));
        fs::write(&script_path, script(&msl, &names, !opts.no_translate))
            .map_err(|e| e.to_string())?;
        fresh_dir(&gen_dir)?;
        let run = run_omc(
            opts.omc,
            &script_path,
            &gen_dir,
            Duration::from_secs(opts.timeout_secs),
        )?;
        stats.processes += 1;
        stats.seconds += run.seconds;
        let parsed = parse_output(&run.output);
        let mut done = 0;
        for m in parsed {
            if !m.crashed {
                done += 1;
                results.insert(m.model.clone(), m);
            }
        }
        let missing: Vec<&str> = names
            .iter()
            .copied()
            .filter(|n| !results.contains_key(*n))
            .collect();
        eprintln!(
            "omc {}/{}: {} 本中 {} 本 {:.1} s{}{}",
            i + 1,
            chunks.len(),
            names.len(),
            done,
            run.seconds,
            if run.timed_out {
                "（時間切れ）"
            } else {
                ""
            },
            match run.exit {
                Some(0) | None => String::new(),
                Some(c) => format!("（終了コード {c}）"),
            }
        );
        if missing.is_empty() {
            continue;
        }
        if run.timed_out {
            stats.timeouts += 1;
        } else if run.exit != Some(0) {
            stats.crashes += 1;
        }
        // 1 本ずつやり直す
        let single_timeout = Duration::from_secs((opts.timeout_secs / 3).max(120));
        for (j, name) in missing.iter().enumerate() {
            let script_path = work_dir.join(format!("chunk-{i:03}-retry-{j:03}.mos"));
            fs::write(&script_path, script(&msl, &[name], !opts.no_translate))
                .map_err(|e| e.to_string())?;
            fresh_dir(&gen_dir)?;
            let run = run_omc(opts.omc, &script_path, &gen_dir, single_timeout)?;
            stats.processes += 1;
            stats.seconds += run.seconds;
            stats.retries += 1;
            let parsed = parse_output(&run.output);
            match parsed.into_iter().find(|m| m.model == *name) {
                Some(m) if !m.crashed => {
                    eprintln!("  再実行 {name}: {:.1} s", run.seconds);
                    results.insert(m.model.clone(), m);
                }
                Some(mut m) => {
                    eprintln!(
                        "  再実行 {name}: omc が{}（{:.1} s）",
                        if run.timed_out {
                            "時間切れ"
                        } else {
                            "落ちた"
                        },
                        run.seconds
                    );
                    m.crashed = true;
                    stats.crashed_models += 1;
                    results.insert(m.model.clone(), m);
                }
                None => {
                    eprintln!(
                        "  再実行 {name}: omc が{}（出力なし、{:.1} s）",
                        if run.timed_out {
                            "時間切れ"
                        } else {
                            "落ちた"
                        },
                        run.seconds
                    );
                    stats.crashed_models += 1;
                    results.insert(
                        (*name).to_string(),
                        OmcModel {
                            model: (*name).to_string(),
                            crashed: true,
                            ..Default::default()
                        },
                    );
                }
            }
        }
    }
    let _ = fs::remove_dir_all(&gen_dir);
    Ok((results, stats))
}

#[derive(Debug, Default, Clone)]
struct RunStats {
    processes: usize,
    seconds: f64,
    timeouts: usize,
    crashes: usize,
    retries: usize,
    crashed_models: usize,
}

// --------------------------------------------------------------------------- 照合

/// S5 の照合の結果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum S5Agreement {
    /// 両方「合う」または両方「合わない」
    Agree,
    /// 片方だけ「合う」
    Disagree,
    /// どちらかに判定が無い
    NotCompared,
}

/// 1 本分の照合。
#[derive(Debug, Clone)]
pub(crate) struct Compared {
    pub ours: Ours,
    pub omc: OmcModel,
}

impl Compared {
    pub(crate) fn s5(&self) -> S5Agreement {
        let o = self.ours.s5_balanced();
        let c = self.omc.check.as_ref().and_then(OmcCheck::balanced);
        match (o, c) {
            (Some(a), Some(b)) if a == b => S5Agreement::Agree,
            (Some(_), Some(_)) => S5Agreement::Disagree,
            _ => S5Agreement::NotCompared,
        }
    }

    /// 方程式数と未知数が omc と同じ数か（両方に数があるときだけ）。
    pub(crate) fn counts_equal(&self) -> Option<bool> {
        let (e, u) = self.ours.counts?;
        let c = self.omc.check.as_ref()?;
        Some(e == c.equations? && u == c.variables?)
    }

    fn omc_translate_kind(&self) -> Option<TranslateKind> {
        self.omc.translate.as_ref().map(|t| t.kind)
    }

    /// S6 の照合: globe の S6 の判定（特異かどうか）と omc の translateModel。
    /// `Some(true)` は一致、`Some(false)` は不一致、`None` は比べられない（どちらかに判定が無い、omc が別の理由で失敗）。
    pub(crate) fn s6(&self) -> Option<bool> {
        let ours_singular = match (self.ours.verdict.as_str(), self.ours.stage) {
            ("Ng", Some(6)) => true,
            ("Ok", _) => false,
            ("Ng", Some(s)) if s > 6 => false,
            _ => return None,
        };
        match self.omc_translate_kind()? {
            TranslateKind::Ok => Some(!ours_singular),
            TranslateKind::Singular | TranslateKind::Imbalanced => Some(ours_singular),
            TranslateKind::Other => None,
        }
    }

    /// S8 の照合: 初期条件の過剰。omc は不足を黙って埋める（警告を出さない）ので、過剰だけを比べる。
    /// `Some(true)` は一致（両方が過剰、または両方が過剰でない）、`Some(false)` は不一致、`None` は比べられない。
    pub(crate) fn s8_over(&self) -> Option<bool> {
        let ours = self.ours.s8.as_ref()?;
        let t = self.omc.translate.as_ref()?;
        if t.kind != TranslateKind::Ok {
            return None;
        }
        Some(ours.is_over() == t.over_specified.is_some())
    }
}

#[derive(Debug, Default)]
struct Summary {
    total: usize,
    omc_check_ok: usize,
    omc_check_failed: usize,
    omc_crashed: usize,
    s5_compared: usize,
    s5_agree: usize,
    s5_disagree: Vec<String>,
    counts_compared: usize,
    counts_equal: usize,
    counts_differ: Vec<String>,
    /// GM-U のモデルのうち、omc の checkModel が通ったもの（方程式数と変数数が合う）
    unsupported_total: usize,
    unsupported_omc_balanced: usize,
    unsupported_omc_translate_ok: usize,
    s6_compared: usize,
    s6_agree: usize,
    s6_disagree: Vec<String>,
    s8_compared: usize,
    s8_agree: usize,
    s8_disagree: Vec<String>,
    /// globe が OK なのに omc の translateModel が失敗したもの（種類ごと）
    ok_but_omc_failed: BTreeMap<&'static str, Vec<String>>,
}

impl Summary {
    fn from_rows(rows: &[Compared]) -> Self {
        let mut s = Summary {
            total: rows.len(),
            ..Default::default()
        };
        for r in rows {
            if r.omc.crashed && r.omc.check.is_none() {
                s.omc_crashed += 1;
            } else if r.omc.check.as_ref().is_some_and(|c| c.ok) {
                s.omc_check_ok += 1;
            } else {
                s.omc_check_failed += 1;
            }
            match r.s5() {
                S5Agreement::Agree => {
                    s.s5_compared += 1;
                    s.s5_agree += 1;
                }
                S5Agreement::Disagree => {
                    s.s5_compared += 1;
                    s.s5_disagree.push(r.ours.model.clone());
                }
                S5Agreement::NotCompared => {}
            }
            if let Some(eq) = r.counts_equal() {
                s.counts_compared += 1;
                if eq {
                    s.counts_equal += 1;
                } else {
                    s.counts_differ.push(r.ours.model.clone());
                }
            }
            if r.ours.verdict == "Unsupported" {
                s.unsupported_total += 1;
                if r.omc.check.as_ref().and_then(OmcCheck::balanced) == Some(true) {
                    s.unsupported_omc_balanced += 1;
                }
                if r.omc_translate_kind() == Some(TranslateKind::Ok) {
                    s.unsupported_omc_translate_ok += 1;
                }
            }
            if let Some(a) = r.s6() {
                s.s6_compared += 1;
                if a {
                    s.s6_agree += 1;
                } else {
                    s.s6_disagree.push(r.ours.model.clone());
                }
            }
            if let Some(a) = r.s8_over() {
                s.s8_compared += 1;
                if a {
                    s.s8_agree += 1;
                } else {
                    s.s8_disagree.push(r.ours.model.clone());
                }
            }
            if r.ours.verdict == "Ok"
                && let Some(t) = &r.omc.translate
                && t.kind != TranslateKind::Ok
            {
                s.ok_but_omc_failed
                    .entry(t.kind.label())
                    .or_default()
                    .push(r.ours.model.clone());
            }
        }
        s
    }

    fn s5_rate(&self) -> Option<f64> {
        (self.s5_compared > 0).then(|| self.s5_agree as f64 * 100.0 / self.s5_compared as f64)
    }
}

fn percent(n: usize, d: usize) -> String {
    if d == 0 {
        "-".to_string()
    } else {
        format!("{:.1}%", n as f64 * 100.0 / d as f64)
    }
}

fn short_name(model: &str) -> String {
    model.strip_prefix("Modelica.").unwrap_or(model).to_string()
}

fn one_line(s: &str, max: usize) -> String {
    let line = s
        .lines()
        .map(str::trim)
        .find(|l| !l.is_empty() && !l.starts_with("Notification"))
        .unwrap_or("");
    let line = line.replace('|', "\\|");
    if line.chars().count() > max {
        let cut: String = line.chars().take(max).collect();
        format!("{cut}…")
    } else {
        line
    }
}

/// omc の失敗の文言の先頭行（Error: から）。
fn first_error_line(errors: &str) -> String {
    let line = errors
        .lines()
        .map(str::trim)
        .find(|l| l.contains("Error:"))
        .or_else(|| errors.lines().map(str::trim).find(|l| !l.is_empty()))
        .unwrap_or("");
    let line = line.split("Error:").nth(1).map_or(line, str::trim);
    one_line(line, 100)
}

struct RunInfo<'a> {
    omc_version: &'a str,
    msl: &'a Path,
    funnel_json: &'a Path,
    stats: &'a RunStats,
    reused: bool,
    translate: bool,
    chunk: usize,
    timeout_secs: u64,
}

fn render_report(rows: &[Compared], s: &Summary, run: &RunInfo<'_>) -> String {
    let mut o = String::new();
    let p = |o: &mut String, line: &str| {
        o.push_str(line);
        o.push('\n');
    };
    p(
        &mut o,
        "# MSL 4.1.0 の全例題の omc との照合（globe check ↔ OpenModelica）",
    );
    p(&mut o, "");
    p(
        &mut o,
        "`cargo xtask omc-compare` が書く。`msl-funnel --json` の結果（globe check の S0〜S8 の判定）を、omc（OpenModelica のコンパイラ、別プロセス）の `checkModel`（方程式数と変数数）と `translateModel`（バックエンドまで。構造特異と初期条件の過剰）と突き合わせる。規則は [ADR-0010](adr/0010-omc-comparison.md)。",
    );
    p(&mut o, "");
    p(&mut o, "| 項目 | 値 |");
    p(&mut o, "| --- | --- |");
    p(&mut o, &format!("| omc | `{}` |", run.omc_version));
    p(
        &mut o,
        &format!(
            "| MSL | `{}`（{}。omc 同梱の `+maint.om` ではなく、globe check と同じ素の MSL） |",
            run.msl.display(),
            MSL_VERSION
        ),
    );
    p(
        &mut o,
        &format!("| globe check の結果 | `{}` |", run.funnel_json.display()),
    );
    p(&mut o, &format!("| 例題 | {} 本 |", s.total));
    p(
        &mut o,
        &format!(
            "| omc の実行 | {} プロセス（{} 本ごと、制限 {} s）、{:.1} s、時間切れ {} 回、異常終了 {} 回、1 本ずつの再実行 {} 本、結果の無いモデル {} 本{} |",
            run.stats.processes,
            run.chunk,
            run.timeout_secs,
            run.stats.seconds,
            run.stats.timeouts,
            run.stats.crashes,
            run.stats.retries,
            run.stats.crashed_models,
            if run.reused {
                "（前回の omc の結果を `--reuse` で再利用し、照合だけやり直した）"
            } else {
                ""
            }
        ),
    );
    p(
        &mut o,
        &format!(
            "| translateModel | {} |",
            if run.translate {
                "呼んだ"
            } else {
                "呼んでいない（`--no-translate`）"
            }
        ),
    );
    p(&mut o, "");
    p(&mut o, "## 集計");
    p(&mut o, "");
    p(&mut o, "| 項目 | 本数 |");
    p(&mut o, "| --- | --- |");
    p(
        &mut o,
        &format!(
            "| omc の checkModel が通った（数が出た） | {} |",
            s.omc_check_ok
        ),
    );
    p(
        &mut o,
        &format!("| omc の checkModel が失敗 | {} |", s.omc_check_failed),
    );
    p(
        &mut o,
        &format!(
            "| omc の結果が無い（落ちた・時間切れ） | {} |",
            s.omc_crashed
        ),
    );
    p(&mut o, "");
    p(
        &mut o,
        "### S5（方程式数と未知数が合うか）の一致率 — P2 の完了条件",
    );
    p(&mut o, "");
    p(
        &mut o,
        "globe check に S5 の判定があるモデル（OK、または S5 以降で止まったもの。GM-U と S4 以前で止まったものは判定が無い）と、omc の `checkModel` の数が出たモデルを比べる。「合う / 合わない」の判定が同じなら一致。",
    );
    p(&mut o, "");
    p(&mut o, "| 項目 | 本数 |");
    p(&mut o, "| --- | --- |");
    p(&mut o, &format!("| 比べたモデル | {} |", s.s5_compared));
    p(&mut o, &format!("| 判定が一致 | {} |", s.s5_agree));
    p(
        &mut o,
        &format!("| 判定が不一致 | {} |", s.s5_disagree.len()),
    );
    p(
        &mut o,
        &format!(
            "| **一致率** | **{}**（完了条件 95% 以上: {}） |",
            s.s5_rate().map_or("-".to_string(), |r| format!("{r:.1}%")),
            match s.s5_rate() {
                Some(r) if r >= 95.0 => "達成",
                Some(_) => "未達",
                None => "測れない",
            }
        ),
    );
    p(
        &mut o,
        &format!(
            "| 方程式数と未知数が omc と同じ数 | {} / {}（{}） |",
            s.counts_equal,
            s.counts_compared,
            percent(s.counts_equal, s.counts_compared)
        ),
    );
    p(&mut o, "");
    if !s.s5_disagree.is_empty() {
        p(&mut o, "判定が不一致のモデル:");
        p(&mut o, "");
        p(&mut o, "| モデル | globe check | omc の数 |");
        p(&mut o, "| --- | --- | --- |");
        for name in &s.s5_disagree {
            if let Some(r) = rows.iter().find(|r| &r.ours.model == name) {
                p(
                    &mut o,
                    &format!(
                        "| `{}` | {} {} | {} |",
                        short_name(name),
                        r.ours.label(),
                        counts_cell(&r.ours),
                        omc_check_cell(&r.omc)
                    ),
                );
            }
        }
        p(&mut o, "");
    }
    if !s.counts_differ.is_empty() {
        p(
            &mut o,
            "判定は一致したが数が違うモデル（globe check の方程式数 / 未知数 ↔ omc の方程式数 / 変数数）:",
        );
        p(&mut o, "");
        p(&mut o, "| モデル | globe check | omc |");
        p(&mut o, "| --- | --- | --- |");
        for name in &s.counts_differ {
            if let Some(r) = rows.iter().find(|r| &r.ours.model == name)
                && r.s5() == S5Agreement::Agree
            {
                p(
                    &mut o,
                    &format!(
                        "| `{}` | {} | {} |",
                        short_name(name),
                        counts_cell(&r.ours),
                        omc_check_cell(&r.omc)
                    ),
                );
            }
        }
        p(&mut o, "");
    }
    p(
        &mut o,
        "### GM-U（コンパイラが未対応）のモデルは omc で通るか",
    );
    p(&mut o, "");
    p(
        &mut o,
        "globe check が GM-U にしたモデルが omc で通れば、それは Rumoca の制約であって、モデルの誤りではない（[ADR-0007](adr/0007-unsupported-classification.md) の C）。",
    );
    p(&mut o, "");
    p(&mut o, "| 項目 | 本数 |");
    p(&mut o, "| --- | --- |");
    p(
        &mut o,
        &format!("| GM-U のモデル | {} |", s.unsupported_total),
    );
    p(
        &mut o,
        &format!(
            "| omc の checkModel で数が合う | {}（{}） |",
            s.unsupported_omc_balanced,
            percent(s.unsupported_omc_balanced, s.unsupported_total)
        ),
    );
    if run.translate {
        p(
            &mut o,
            &format!(
                "| omc の translateModel も通る | {}（{}） |",
                s.unsupported_omc_translate_ok,
                percent(s.unsupported_omc_translate_ok, s.unsupported_total)
            ),
        );
    }
    p(&mut o, "");
    let u3: Vec<&Compared> = rows
        .iter()
        .filter(|r| r.ours.gm_code.as_deref() == Some("GM-U0003"))
        .collect();
    if !u3.is_empty() {
        p(
            &mut o,
            &format!(
                "既知の制約の一覧（`GM-U0003`、`src/Modelica/msl-known-unsupported.txt`）に当たった {} 本:",
                u3.len()
            ),
        );
        p(&mut o, "");
        p(
            &mut o,
            "| モデル | Rumoca のコード | omc の checkModel | omc の translateModel |",
        );
        p(&mut o, "| --- | --- | --- | --- |");
        for r in &u3 {
            p(
                &mut o,
                &format!(
                    "| `{}` | {} | {} | {} |",
                    short_name(&r.ours.model),
                    r.ours.rumoca_code.as_deref().unwrap_or("-"),
                    omc_check_cell(&r.omc),
                    omc_translate_cell(&r.omc)
                ),
            );
        }
        p(&mut o, "");
        let not_passing: Vec<&&Compared> = u3
            .iter()
            .filter(|r| {
                r.omc.check.as_ref().and_then(OmcCheck::balanced) != Some(true)
                    || r.omc_translate_kind()
                        .is_some_and(|k| k != TranslateKind::Ok)
            })
            .collect();
        if not_passing.is_empty() {
            p(
                &mut o,
                "一覧の全部が omc で通る。一覧は Rumoca の制約として正しい。",
            );
        } else {
            p(
                &mut o,
                &format!(
                    "omc でも通らないもの {} 本（モデルの誤り、または omc 側の制約。一覧から外すかは個別に見る）:",
                    not_passing.len()
                ),
            );
            p(&mut o, "");
            for r in not_passing {
                p(
                    &mut o,
                    &format!(
                        "- `{}`: {}",
                        short_name(&r.ours.model),
                        r.omc.translate.as_ref().map_or_else(
                            || "translateModel なし".to_string(),
                            |t| first_error_line(&t.errors)
                        )
                    ),
                );
            }
        }
        p(&mut o, "");
    }
    if run.translate {
        p(&mut o, "### S6（構造特異）の照合");
        p(&mut o, "");
        p(
            &mut o,
            "globe check の S6（完全マッチングの有無）と、omc の `translateModel` の結果（「structurally singular」「pantelidesIndexReduction failed」「imbalanced number of equations」を特異とみなす）を比べる。omc が別の理由（未対応の関数など）で失敗したものは比べない。",
        );
        p(&mut o, "");
        p(&mut o, "| 項目 | 本数 |");
        p(&mut o, "| --- | --- |");
        p(&mut o, &format!("| 比べたモデル | {} |", s.s6_compared));
        p(
            &mut o,
            &format!(
                "| 一致 | {}（{}） |",
                s.s6_agree,
                percent(s.s6_agree, s.s6_compared)
            ),
        );
        p(&mut o, &format!("| 不一致 | {} |", s.s6_disagree.len()));
        p(&mut o, "");
        let ng6: Vec<&Compared> = rows
            .iter()
            .filter(|r| r.ours.verdict == "Ng" && r.ours.stage == Some(6))
            .collect();
        if !ng6.is_empty() {
            p(
                &mut o,
                &format!("globe check が S6 で止めた {} 本:", ng6.len()),
            );
            p(&mut o, "");
            p(
                &mut o,
                "| モデル | omc の checkModel | omc の translateModel | omc の文言 |",
            );
            p(&mut o, "| --- | --- | --- | --- |");
            for r in &ng6 {
                p(
                    &mut o,
                    &format!(
                        "| `{}` | {} | {} | {} |",
                        short_name(&r.ours.model),
                        omc_check_cell(&r.omc),
                        omc_translate_cell(&r.omc),
                        r.omc
                            .translate
                            .as_ref()
                            .map_or_else(|| "-".to_string(), |t| first_error_line(&t.errors))
                    ),
                );
            }
            p(&mut o, "");
        }
        if !s.s6_disagree.is_empty() {
            p(&mut o, "不一致のモデル:");
            p(&mut o, "");
            p(
                &mut o,
                "| モデル | globe check | omc の translateModel | omc の文言 |",
            );
            p(&mut o, "| --- | --- | --- | --- |");
            for name in &s.s6_disagree {
                if let Some(r) = rows.iter().find(|r| &r.ours.model == name) {
                    p(
                        &mut o,
                        &format!(
                            "| `{}` | {} | {} | {} |",
                            short_name(name),
                            r.ours.label(),
                            omc_translate_cell(&r.omc),
                            r.omc
                                .translate
                                .as_ref()
                                .map_or_else(|| "-".to_string(), |t| first_error_line(&t.errors))
                        ),
                    );
                }
            }
            p(&mut o, "");
        }
        p(&mut o, "### S8（初期条件の過剰）の照合");
        p(&mut o, "");
        p(
            &mut o,
            "globe check の S8 の判定（要る数と与えられた数）と、omc の `translateModel` の警告「The initial conditions are over specified」を比べる。omc は不足を黙って start 値で埋める（`-d=initialization` でも警告が出ない）ので、**過剰だけ**を比べる。omc の translateModel が失敗したものは比べない。",
        );
        p(&mut o, "");
        p(&mut o, "| 項目 | 本数 |");
        p(&mut o, "| --- | --- |");
        p(&mut o, &format!("| 比べたモデル | {} |", s.s8_compared));
        p(
            &mut o,
            &format!(
                "| 一致（両方が過剰、または両方が過剰でない） | {}（{}） |",
                s.s8_agree,
                percent(s.s8_agree, s.s8_compared)
            ),
        );
        p(&mut o, &format!("| 不一致 | {} |", s.s8_disagree.len()));
        p(&mut o, "");
        let over: Vec<&Compared> = rows
            .iter()
            .filter(|r| {
                r.ours.s8.as_ref().is_some_and(OursS8::is_over)
                    || r.omc
                        .translate
                        .as_ref()
                        .is_some_and(|t| t.over_specified.is_some())
            })
            .collect();
        if !over.is_empty() {
            p(&mut o, "どちらかが過剰としたモデル:");
            p(&mut o, "");
            p(
                &mut o,
                "| モデル | globe check の S8（与えられた / 要る） | omc |",
            );
            p(&mut o, "| --- | --- | --- |");
            for r in &over {
                p(
                    &mut o,
                    &format!(
                        "| `{}` | {} | {} |",
                        short_name(&r.ours.model),
                        s8_cell(&r.ours),
                        omc_s8_cell(&r.omc)
                    ),
                );
            }
            p(&mut o, "");
        }
        p(
            &mut o,
            "### globe check が OK なのに omc の translateModel が失敗したモデル",
        );
        p(&mut o, "");
        if s.ok_but_omc_failed.is_empty() {
            p(&mut o, "なし。");
        } else {
            for (kind, names) in &s.ok_but_omc_failed {
                p(&mut o, &format!("- {}: {} 本", kind, names.len()));
                for name in names {
                    if let Some(r) = rows.iter().find(|r| &r.ours.model == name) {
                        p(
                            &mut o,
                            &format!(
                                "  - `{}`: {}",
                                short_name(name),
                                r.omc
                                    .translate
                                    .as_ref()
                                    .map_or_else(String::new, |t| first_error_line(&t.errors))
                            ),
                        );
                    }
                }
            }
        }
        p(&mut o, "");
    }
    p(&mut o, "## モデルごとの結果");
    p(&mut o, "");
    p(
        &mut o,
        "globe check の列は msl-funnel の判定（OK、NG と止まった段階、GM-U と Rumoca のコード）と方程式数 / 未知数。omc の列は checkModel の方程式数 / 変数数（○ は合う、× は合わない）と translateModel の結果。S5 は判定の一致（○ / ×、- は比べられない）。S8 は globe check の判定 ↔ omc の過剰の警告。",
    );
    p(&mut o, "");
    p(
        &mut o,
        "| # | モデル | globe check | 数 | omc checkModel | S5 | omc translateModel | S8 |",
    );
    p(&mut o, "| --- | --- | --- | --- | --- | --- | --- | --- |");
    for (i, r) in rows.iter().enumerate() {
        p(
            &mut o,
            &format!(
                "| {} | `{}`{} | {} | {} | {} | {} | {} | {} |",
                i + 1,
                short_name(&r.ours.model),
                if r.ours.partial { "（partial）" } else { "" },
                r.ours.label(),
                counts_cell(&r.ours),
                omc_check_cell(&r.omc),
                match r.s5() {
                    S5Agreement::Agree => "○",
                    S5Agreement::Disagree => "**×**",
                    S5Agreement::NotCompared => "-",
                },
                omc_translate_cell(&r.omc),
                if r.ours.s8.is_some()
                    || r.omc
                        .translate
                        .as_ref()
                        .is_some_and(|t| t.over_specified.is_some())
                {
                    format!("{} ↔ {}", s8_cell(&r.ours), omc_s8_cell(&r.omc))
                } else {
                    "-".to_string()
                }
            ),
        );
    }
    o
}

fn counts_cell(ours: &Ours) -> String {
    ours.counts
        .map_or_else(|| "-".to_string(), |(e, u)| format!("{e} / {u}"))
}

fn omc_check_cell(omc: &OmcModel) -> String {
    match &omc.check {
        None => {
            if omc.crashed {
                "落ちた".to_string()
            } else {
                "-".to_string()
            }
        }
        Some(c) => match (c.equations, c.variables) {
            (Some(e), Some(v)) => format!("{e} / {v} {}", if e == v { "○" } else { "×" }),
            _ => {
                if c.ok {
                    "数なし".to_string()
                } else {
                    format!(
                        "失敗: {}",
                        first_error_line(if c.errors.trim().is_empty() {
                            &c.text
                        } else {
                            &c.errors
                        })
                    )
                }
            }
        },
    }
}

fn omc_translate_cell(omc: &OmcModel) -> String {
    match &omc.translate {
        None => {
            if omc.crashed {
                "落ちた".to_string()
            } else {
                "-".to_string()
            }
        }
        Some(t) => t.kind.label().to_string(),
    }
}

fn s8_cell(ours: &Ours) -> String {
    ours.s8.as_ref().map_or_else(
        || "-".to_string(),
        |x| {
            let mut s = format!("{} {} / {}", x.balance, x.provided, x.required);
            if x.matched && (x.extra > 0 || x.missing > 0) {
                s.push_str(&format!("（冗長 {}・決まらない {}）", x.extra, x.missing));
            }
            if x.uncertain {
                s.push_str("（確定できない）");
            }
            s
        },
    )
}

fn omc_s8_cell(omc: &OmcModel) -> String {
    match &omc.translate {
        None => "-".to_string(),
        Some(t) if t.kind != TranslateKind::Ok => "（失敗）".to_string(),
        Some(t) => match t.over_specified {
            None => "過剰でない".to_string(),
            Some(None) => "過剰".to_string(),
            Some(Some(n)) => format!("過剰 {n}"),
        },
    }
}

fn render_json(rows: &[Compared], s: &Summary, run: &RunInfo<'_>) -> Value {
    let models: Vec<Value> = rows
        .iter()
        .map(|r| {
            json!({
                "model": r.ours.model,
                "globe": {
                    "verdict": r.ours.verdict,
                    "stage": r.ours.stage,
                    "gm_code": r.ours.gm_code,
                    "rumoca_code": r.ours.rumoca_code,
                    "counts": r.ours.counts.map(|(e, u)| json!({"equations": e, "unknowns": u})),
                    "s5_balanced": r.ours.s5_balanced(),
                    "s6_regular": r.ours.s6_regular,
                    "s7_index": r.ours.s7_index,
                    "s8": r.ours.s8.as_ref().map(|x| json!({
                        "required": x.required, "provided": x.provided, "balance": x.balance, "uncertain": x.uncertain,
                        "matched": x.matched, "extra": x.extra, "missing": x.missing,
                    })),
                },
                "omc": {
                    "crashed": r.omc.crashed,
                    "check": r.omc.check.as_ref().map(|c| json!({
                        "ok": c.ok, "equations": c.equations, "variables": c.variables, "trivial": c.trivial,
                        "text": c.text, "errors": c.errors,
                    })),
                    "translate": r.omc.translate.as_ref().map(|t| json!({
                        "ok": t.ok, "kind": t.kind.label(), "errors": t.errors,
                        "over_specified": t.over_specified.map(|n| json!(n)),
                    })),
                },
                "s5": match r.s5() {
                    S5Agreement::Agree => "agree",
                    S5Agreement::Disagree => "disagree",
                    S5Agreement::NotCompared => "not_compared",
                },
                "counts_equal": r.counts_equal(),
                "s6": r.s6(),
                "s8_over": r.s8_over(),
            })
        })
        .collect();
    json!({
        "omc_version": run.omc_version,
        "msl": run.msl.display().to_string(),
        "funnel_json": run.funnel_json.display().to_string(),
        "translate": run.translate,
        "run": {
            "processes": run.stats.processes,
            "seconds": run.stats.seconds,
            "timeouts": run.stats.timeouts,
            "crashes": run.stats.crashes,
            "retries": run.stats.retries,
            "crashed_models": run.stats.crashed_models,
            "reused": run.reused,
        },
        "summary": {
            "total": s.total,
            "omc_check_ok": s.omc_check_ok,
            "omc_check_failed": s.omc_check_failed,
            "omc_crashed": s.omc_crashed,
            "s5_compared": s.s5_compared,
            "s5_agree": s.s5_agree,
            "s5_disagree": s.s5_disagree,
            "s5_rate": s.s5_rate(),
            "counts_compared": s.counts_compared,
            "counts_equal": s.counts_equal,
            "unsupported_total": s.unsupported_total,
            "unsupported_omc_balanced": s.unsupported_omc_balanced,
            "unsupported_omc_translate_ok": s.unsupported_omc_translate_ok,
            "s6_compared": s.s6_compared,
            "s6_agree": s.s6_agree,
            "s6_disagree": s.s6_disagree,
            "s8_compared": s.s8_compared,
            "s8_agree": s.s8_agree,
            "s8_disagree": s.s8_disagree,
            "ok_but_omc_failed": s.ok_but_omc_failed,
        },
        "models": models,
    })
}

/// 前回の `--json` から omc の結果（と実行の統計）を読む（`--reuse`）。
fn read_reuse(path: &Path) -> Result<(BTreeMap<String, OmcModel>, String, bool, RunStats)> {
    let text =
        fs::read_to_string(path).map_err(|e| format!("{} を読めない: {e}", path.display()))?;
    let root: Value = serde_json::from_str(&text)
        .map_err(|e| format!("{} の JSON が壊れている: {e}", path.display()))?;
    let version = root
        .get("omc_version")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let translate = root
        .get("translate")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let stats = root.get("run").map_or_else(RunStats::default, |r| {
        let n = |k: &str| r.get(k).and_then(Value::as_u64).unwrap_or(0) as usize;
        RunStats {
            processes: n("processes"),
            seconds: r.get("seconds").and_then(Value::as_f64).unwrap_or(0.0),
            timeouts: n("timeouts"),
            crashes: n("crashes"),
            retries: n("retries"),
            crashed_models: n("crashed_models"),
        }
    });
    let mut out = BTreeMap::new();
    for m in root
        .get("models")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        let name = m
            .get("model")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        let omc = m.get("omc").cloned().unwrap_or(Value::Null);
        let check = omc.get("check").filter(|c| !c.is_null()).map(|c| {
            let text = c.get("text").and_then(Value::as_str).unwrap_or("");
            let errors = c.get("errors").and_then(Value::as_str).unwrap_or("");
            parse_check(text, errors)
        });
        let translate_result = omc.get("translate").filter(|t| !t.is_null()).map(|t| {
            let ok = t.get("ok").and_then(Value::as_bool).unwrap_or(false);
            let errors = t.get("errors").and_then(Value::as_str).unwrap_or("");
            parse_translate(if ok { "true" } else { "false" }, errors)
        });
        out.insert(
            name.clone(),
            OmcModel {
                model: name,
                check,
                translate: translate_result,
                crashed: omc.get("crashed").and_then(Value::as_bool).unwrap_or(false),
            },
        );
    }
    Ok((out, version, translate, stats))
}

pub(crate) fn omc_compare(opts: &Options<'_>) -> Result<ExitCode> {
    let mut ours = read_funnel_json(opts.funnel_json)?;
    if let Some(f) = opts.filter {
        ours.retain(|o| o.model.contains(f));
    }
    if let Some(n) = opts.limit {
        ours.truncate(n);
    }
    if ours.is_empty() {
        return Err("照合するモデルが無い".to_string());
    }
    let names: Vec<String> = ours.iter().map(|o| o.model.clone()).collect();
    let (results, stats, version, translate, reused) = match opts.reuse {
        Some(path) => {
            let (r, v, t, stats) = read_reuse(path)?;
            eprintln!(
                "omc の結果を {} から読んだ（{} 本）",
                path.display(),
                r.len()
            );
            (r, stats, v, t, true)
        }
        None => {
            if !opts.msl.join(".globe-mode-msl").exists() {
                return Err(format!(
                    "{} に MSL が無い。先に `cargo xtask fetch-msl` を実行する",
                    opts.msl.display()
                ));
            }
            let version = omc_version(opts.omc)?;
            eprintln!("omc: {version}");
            eprintln!(
                "{} 本を {} 本ごとに検査する（制限 {} s / プロセス）",
                names.len(),
                opts.chunk.max(1),
                opts.timeout_secs
            );
            let (r, stats) = run_all(opts, &names)?;
            (r, stats, version, !opts.no_translate, false)
        }
    };
    let rows: Vec<Compared> = ours
        .into_iter()
        .map(|o| {
            let omc = results.get(&o.model).cloned().unwrap_or_else(|| OmcModel {
                model: o.model.clone(),
                crashed: true,
                ..Default::default()
            });
            Compared { ours: o, omc }
        })
        .collect();
    let summary = Summary::from_rows(&rows);
    let run = RunInfo {
        omc_version: &version,
        msl: opts.msl,
        funnel_json: opts.funnel_json,
        stats: &stats,
        reused,
        translate,
        chunk: opts.chunk.max(1),
        timeout_secs: opts.timeout_secs,
    };
    println!(
        "omc との照合: {} 本。S5 の一致 {} / {}（{}）、数が同じ {} / {}、GM-U {} 本中 omc で数が合う {}、S6 の一致 {} / {}、S8（過剰）の一致 {} / {}、omc の結果なし {}",
        summary.total,
        summary.s5_agree,
        summary.s5_compared,
        summary
            .s5_rate()
            .map_or("-".to_string(), |r| format!("{r:.1}%")),
        summary.counts_equal,
        summary.counts_compared,
        summary.unsupported_total,
        summary.unsupported_omc_balanced,
        summary.s6_agree,
        summary.s6_compared,
        summary.s8_agree,
        summary.s8_compared,
        summary.omc_crashed
    );
    if let Some(path) = opts.report {
        fs::write(path, render_report(&rows, &summary, &run))
            .map_err(|e| format!("{} を書けない: {e}", path.display()))?;
        eprintln!("報告を {} に書いた", path.display());
    }
    if let Some(path) = opts.json {
        let text = serde_json::to_string_pretty(&render_json(&rows, &summary, &run))
            .map_err(|e| e.to_string())?;
        fs::write(path, text).map_err(|e| format!("{} を書けない: {e}", path.display()))?;
        eprintln!("JSON を {} に書いた", path.display());
    }
    Ok(ExitCode::SUCCESS)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"true
""
true
@@LOADED
@@BEGIN Modelica.A.Examples.Good

"Check of Modelica.A.Examples.Good completed successfully.
Class Modelica.A.Examples.Good has 44 equation(s) and 44 variable(s).
25 of these are trivial equation(s)."
@@ERR

""
@@TRANSLATE

true
@@ERR

"Warning: The initial conditions are over specified. The following 2 initial equations are redundant, so they are removed from the initialization system:
         a.y = $START.a.y
         b.y = $START.b.y.
"
@@END

@@BEGIN Modelica.A.Examples.Dumped

"Check of Modelica.A.Examples.Dumped completed successfully.
Class Modelica.A.Examples.Dumped has 28 equation(s) and 28 variable(s).
20 of these are trivial equation(s)."
@@ERR

""
@@TRANSLATE


Nonlinear iteration variables with predefined start attribute in NLSJac24. (1)
========================================
1: SineVoltage1.i:VARIABLE(start = 0.0 unit = "A" )  "Current flowing from pin p to pin n" type: Real

true
@@ERR

""
@@END

@@BEGIN Modelica.A.Examples.Partial

"Check of Modelica.A.Examples.Partial completed successfully.
Class Modelica.A.Examples.Partial has 23 equation(s) and 27 variable(s).
15 of these are trivial equation(s)."
@@ERR

""
@@TRANSLATE

false
@@ERR

"Error: Internal error IndexReduction.pantelidesIndexReduction failed! System is structurally singular and cannot be reduced.
Error: Internal error Transformation Module PFPlusExt index Reduction Method Pantelides failed!
"
@@END

@@BEGIN Modelica.A.Examples.Crashed

"Check of Modelica.A.Examples.Crashed completed successfully.
Class Modelica.A.Examples.Crashed has 3 equation(s) and 3 variable(s).
0 of these are trivial equation(s)."
@@ERR

""
@@TRANSLATE
"#;

    #[test]
    fn parses_blocks_counts_and_markers() {
        let models = parse_output(SAMPLE);
        assert_eq!(models.len(), 4);
        let good = &models[0];
        assert_eq!(good.model, "Modelica.A.Examples.Good");
        assert!(!good.crashed);
        let c = good.check.as_ref().unwrap();
        assert!(c.ok);
        assert_eq!(
            (c.equations, c.variables, c.trivial),
            (Some(44), Some(44), Some(25))
        );
        assert_eq!(c.balanced(), Some(true));
        let t = good.translate.as_ref().unwrap();
        assert!(t.ok);
        assert_eq!(t.kind, TranslateKind::Ok);
        assert_eq!(t.over_specified, Some(Some(2)));

        let dumped = &models[1];
        let t = dumped.translate.as_ref().unwrap();
        assert!(t.ok, "dump の後の true を結果として読む");
        assert_eq!(t.kind, TranslateKind::Ok);

        let partial = &models[2];
        let c = partial.check.as_ref().unwrap();
        assert_eq!(c.balanced(), Some(false));
        let t = partial.translate.as_ref().unwrap();
        assert!(!t.ok);
        assert_eq!(t.kind, TranslateKind::Singular);
        assert_eq!(t.over_specified, None);

        let crashed = &models[3];
        assert!(crashed.crashed);
        assert_eq!(crashed.check.as_ref().unwrap().equations, Some(3));
        assert!(crashed.translate.is_none());
    }

    #[test]
    fn unquotes_omc_echo() {
        assert_eq!(unquote("  \"a \\\"b\\\" c\"\n"), "a \"b\" c");
        assert_eq!(unquote("true"), "true");
        assert_eq!(unquote("\"\""), "");
    }

    #[test]
    fn classifies_translate_errors() {
        assert_eq!(
            classify_translate(true, "Error: whatever"),
            TranslateKind::Ok
        );
        assert_eq!(
            classify_translate(
                false,
                "Error: Model is structurally singular, error found sorting equations"
            ),
            TranslateKind::Singular
        );
        assert_eq!(
            classify_translate(
                false,
                "Error: An independent subset of the model has imbalanced number of equations (1) and variables (2)."
            ),
            TranslateKind::Imbalanced
        );
        assert_eq!(
            classify_translate(false, "Error: Function foo not found"),
            TranslateKind::Other
        );
        assert_eq!(classify_translate(false, ""), TranslateKind::Other);
    }

    #[test]
    fn script_calls_check_and_translate_as_statements() {
        let s = script(Path::new("/x/msl"), &["Modelica.A.B"], true);
        assert!(s.contains("setModelicaPath(\"/x/msl\");"));
        assert!(s.contains("loadModel(Modelica, {\"4.1.0\"});"));
        assert!(s.contains("\ncheckModel(Modelica.A.B);\n"));
        assert!(s.contains("\ntranslateModel(Modelica.A.B);\n"));
        assert!(!s.contains("String(translateModel"));
        let s = script(Path::new("/x/msl"), &["Modelica.A.B"], false);
        assert!(!s.contains("translateModel"));
    }

    fn ours(verdict: &str, stage: Option<u8>, counts: Option<(usize, usize)>) -> Ours {
        Ours {
            model: "M".to_string(),
            partial: false,
            verdict: verdict.to_string(),
            stage,
            gm_code: None,
            rumoca_code: None,
            counts,
            s6_regular: None,
            s7_index: None,
            s8: None,
        }
    }

    fn omc(equations: usize, variables: usize, translate: Option<(bool, &str)>) -> OmcModel {
        OmcModel {
            model: "M".to_string(),
            check: Some(OmcCheck {
                ok: true,
                equations: Some(equations),
                variables: Some(variables),
                trivial: None,
                text: String::new(),
                errors: String::new(),
            }),
            translate: translate
                .map(|(ok, e)| parse_translate(if ok { "true" } else { "false" }, e)),
            crashed: false,
        }
    }

    #[test]
    fn s5_agreement_rules() {
        // OK ↔ omc で数が合う
        let r = Compared {
            ours: ours("Ok", None, Some((44, 44))),
            omc: omc(44, 44, None),
        };
        assert_eq!(r.s5(), S5Agreement::Agree);
        assert_eq!(r.counts_equal(), Some(true));
        // S6 で止まった（S5 は通った） ↔ omc で合う
        let r = Compared {
            ours: ours("Ng", Some(6), Some((58, 58))),
            omc: omc(58, 58, None),
        };
        assert_eq!(r.s5(), S5Agreement::Agree);
        // S5 で止まった ↔ omc で合わない
        let r = Compared {
            ours: ours("Ng", Some(5), Some((23, 27))),
            omc: omc(23, 27, None),
        };
        assert_eq!(r.s5(), S5Agreement::Agree);
        // S5 で止まったのに omc は合う → 不一致
        let r = Compared {
            ours: ours("Ng", Some(5), Some((23, 27))),
            omc: omc(27, 27, None),
        };
        assert_eq!(r.s5(), S5Agreement::Disagree);
        assert_eq!(r.counts_equal(), Some(false));
        // GM-U は比べない
        let r = Compared {
            ours: ours("Unsupported", Some(5), None),
            omc: omc(10, 10, None),
        };
        assert_eq!(r.s5(), S5Agreement::NotCompared);
        assert_eq!(r.counts_equal(), None);
        // omc が落ちたら比べない
        let r = Compared {
            ours: ours("Ok", None, Some((1, 1))),
            omc: OmcModel {
                model: "M".into(),
                crashed: true,
                ..Default::default()
            },
        };
        assert_eq!(r.s5(), S5Agreement::NotCompared);
    }

    #[test]
    fn s6_and_s8_rules() {
        let singular = "Error: Internal error IndexReduction.pantelidesIndexReduction failed! Found empty set of continuous equations";
        let r = Compared {
            ours: ours("Ng", Some(6), Some((58, 58))),
            omc: omc(58, 58, Some((false, singular))),
        };
        assert_eq!(r.s6(), Some(true));
        let r = Compared {
            ours: ours("Ok", None, Some((58, 58))),
            omc: omc(58, 58, Some((false, singular))),
        };
        assert_eq!(r.s6(), Some(false));
        let r = Compared {
            ours: ours("Ok", None, Some((58, 58))),
            omc: omc(58, 58, Some((false, "Error: Function foo not found"))),
        };
        assert_eq!(r.s6(), None);
        let r = Compared {
            ours: ours("Ok", None, Some((58, 58))),
            omc: omc(58, 58, Some((true, ""))),
        };
        assert_eq!(r.s6(), Some(true));

        let over = "Warning: The initial conditions are over specified. The following 2 initial equations are redundant, so they are removed from the initialization system:";
        let mut o = ours("Ok", None, Some((8, 8)));
        o.s8 = Some(OursS8 {
            required: 6,
            provided: 8,
            matched: false,
            extra: 0,
            missing: 0,
            balance: "過剰".into(),
            uncertain: true,
        });
        let r = Compared {
            ours: o.clone(),
            omc: omc(8, 8, Some((true, over))),
        };
        assert_eq!(r.s8_over(), Some(true));
        let r = Compared {
            ours: o.clone(),
            omc: omc(8, 8, Some((true, ""))),
        };
        assert_eq!(r.s8_over(), Some(false));
        o.s8 = Some(OursS8 {
            required: 7,
            provided: 0,
            matched: false,
            extra: 0,
            missing: 0,
            balance: "不足".into(),
            uncertain: false,
        });
        let r = Compared {
            ours: o.clone(),
            omc: omc(8, 8, Some((true, ""))),
        };
        assert_eq!(r.s8_over(), Some(true));
        // マッチングがあれば、数の差ではなく冗長の数で過剰を決める（NonCircularPipes: 不足 4 / 40 だが冗長 2）
        o.s8 = Some(OursS8 {
            required: 40,
            provided: 36,
            matched: true,
            extra: 2,
            missing: 6,
            balance: "過剰".into(),
            uncertain: true,
        });
        let r = Compared {
            ours: o.clone(),
            omc: omc(8, 8, Some((true, over))),
        };
        assert_eq!(r.s8_over(), Some(true));
        assert!(
            s8_cell(&o).contains("冗長 2・決まらない 6"),
            "{}",
            s8_cell(&o)
        );
        let r = Compared {
            ours: o,
            omc: omc(8, 8, Some((false, singular))),
        };
        assert_eq!(r.s8_over(), None);
    }

    #[test]
    fn reads_funnel_json_rows() {
        let v: Value = serde_json::from_str(
            r#"{"model":"Modelica.X","partial":false,"verdict":"Ng","stage":"S6","gm_code":"GM-E6001","rumoca_code":null,
                "counts":{"states":1,"algebraics":42,"outputs":0,"equations":43,"unknowns":43},"s6":{"regular":false},"s7":null,"s8":null}"#,
        )
        .unwrap();
        let o = ours_from_json(&v);
        assert_eq!(o.stage, Some(6));
        assert_eq!(o.counts, Some((43, 43)));
        assert_eq!(o.s5_balanced(), Some(true));
        assert_eq!(o.s6_regular, Some(false));
        assert_eq!(o.label(), "NG S6 GM-E6001");
    }
}

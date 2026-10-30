//! `globe simulate`（S011、ADR-0013）。検査（S0〜S10）を通してから実行し、結果を CSV か MAT v4 に書く。
//!
//! S012 から: `--param 名前=値`（パラメータの上書き）、`--step`（固定刻みの幅）、`--list-parameters`
//! （再コンパイル無しに変えられるパラメータの一覧）、`--list-solvers`（ソルバの登録表）、結果の形式
//! （`-o x.mat` は MAT v4、それ以外は CSV。`--format` で上書き）。

use std::collections::BTreeMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use globe_mode_core::Diagnostic;
use globe_mode_exec::{CompiledModel, SimReport};
use globe_mode_ir::{
    ResolvedSettings, ResultParameter, SettingOrigin, SimRun, SimSettings, Termination,
    TunableParameter, VariableRole,
};
use globe_mode_sim::solvers::{SOLVERS, SolverStatus};

use super::{Loaded, load_db, print_diagnostics, report_early_failure};

/// `globe simulate` の引数（clap の外で受け渡す形）。
#[derive(Debug)]
pub(crate) struct SimulateArgs {
    pub files: Vec<PathBuf>,
    pub library: Vec<PathBuf>,
    pub model: String,
    pub settings: SimSettings,
    /// 結果の書き先（`-` は標準出力）
    pub output: Option<PathBuf>,
    /// 結果の形式
    pub format: OutputFormat,
    /// パラメータの一覧を表示して終わる（実行しない）
    pub list_parameters: bool,
    /// S0〜S10 に Error があっても実行する
    pub force: bool,
    pub json: bool,
}

/// 結果の形式。
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub(crate) enum OutputFormat {
    /// CSV（MAP-LIB の参照結果と同じ形）
    Csv,
    /// MAT v4（OpenModelica・Dymola の dsres 形式、binTrans）
    Mat,
}

impl OutputFormat {
    /// `--format` があればそれを、無ければ書き先の拡張子（`.mat` なら MAT、それ以外は CSV）で決める。
    pub(crate) fn resolve(format: Option<OutputFormat>, output: Option<&Path>) -> OutputFormat {
        format.unwrap_or_else(|| match output.and_then(Path::extension) {
            Some(ext) if ext.eq_ignore_ascii_case("mat") => OutputFormat::Mat,
            _ => OutputFormat::Csv,
        })
    }
}

/// `--param 名前=値` を読む。名前の前後の空白は除く。同じ名前を 2 回指定したら誤り。
/// 値が有限でないことは、設定を決めるところ（GM-E10006）で報告する。
pub(crate) fn parse_params(params: &[String]) -> Result<BTreeMap<String, f64>, String> {
    let mut out = BTreeMap::new();
    for p in params {
        let Some((name, value)) = p.split_once('=') else {
            return Err(format!("--param {p}: 「名前=値」の形でない"));
        };
        let name = name.trim();
        if name.is_empty() {
            return Err(format!("--param {p}: 名前が空"));
        }
        let value: f64 = value
            .trim()
            .parse()
            .map_err(|_| format!("--param {p}: 値「{}」が数でない", value.trim()))?;
        if out.insert(name.to_string(), value).is_some() {
            return Err(format!("--param: パラメータ {name} を 2 回指定した"));
        }
    }
    Ok(out)
}

/// `--list-solvers`: ソルバの登録表を表示する。
pub(crate) fn list_solvers(json: bool) -> ExitCode {
    if json {
        let text = serde_json::to_string_pretty(SOLVERS).expect("serialize solvers");
        println!("{text}");
    } else {
        for line in solver_lines() {
            println!("{line}");
        }
    }
    ExitCode::SUCCESS
}

/// ソルバの表（名前・状態・説明）。
fn solver_lines() -> Vec<String> {
    let rows: Vec<(&str, String, &str)> = SOLVERS
        .iter()
        .map(|e| {
            let status = match e.status {
                SolverStatus::Available(id) if id.is_fixed_step() => {
                    "使える（固定刻み）".to_string()
                }
                SolverStatus::Available(_) => "使える".to_string(),
                SolverStatus::Alias(id) => format!("別名 → {}", id.name()),
                SolverStatus::Planned => match e.instead {
                    Some(id) => format!("未実装（代わり: {}）", id.name()),
                    None => "未実装".to_string(),
                },
            };
            (e.name, status, e.description)
        })
        .collect();
    let name_width = rows
        .iter()
        .map(|r| width(r.0))
        .max()
        .unwrap_or(0)
        .max(width("名前"));
    let status_width = rows.iter().map(|r| width(&r.1)).max().unwrap_or(0);
    let pad = |s: &str, w: usize| format!("{s}{}", " ".repeat(w.saturating_sub(width(s))));
    let mut lines = vec![format!(
        "{}  {}  説明",
        pad("名前", name_width),
        pad("状態", status_width)
    )];
    lines.extend(rows.iter().map(|(name, status, description)| {
        format!(
            "{}  {}  {description}",
            pad(name, name_width),
            pad(status, status_width)
        )
    }));
    lines
}

/// 端末での表示の幅（全角の文字を 2 とする大まかなもの。矢印などの記号は 1）。
fn width(s: &str) -> usize {
    s.chars()
        .map(|c| match c as u32 {
            0x1100.. if !(0x2000..0x2c00).contains(&(c as u32)) => 2,
            _ => 1,
        })
        .sum()
}

/// `--list-parameters` の表（名前・既定値・単位・範囲・説明）。
fn parameter_lines(model: &str, parameters: &[TunableParameter]) -> Vec<String> {
    let mut lines = vec![format!(
        "{model}: 再コンパイル無しに変えられるパラメータ {} 個（--param 名前=値 で上書きできる）",
        parameters.len()
    )];
    for p in parameters {
        let mut line = format!("    {} = {}", p.name, p.default);
        if let Some(u) = &p.unit {
            line.push_str(&format!(" [{u}]"));
        }
        match (p.min, p.max) {
            (None, None) => {}
            (min, max) => line.push_str(&format!(
                "（範囲 {} 〜 {}）",
                min.map_or("-∞".to_string(), |x| x.to_string()),
                max.map_or("∞".to_string(), |x| x.to_string())
            )),
        }
        if let Some(d) = &p.description {
            line.push_str(&format!("  \"{d}\""));
        }
        lines.push(line);
    }
    lines
}

/// MAT に添えるパラメータ（実行系が返さなかったときに、tunable なパラメータの既定値と上書きから作る）。
fn fallback_parameters(
    parameters: &[TunableParameter],
    overrides: &BTreeMap<String, f64>,
) -> Vec<ResultParameter> {
    parameters
        .iter()
        .map(|p| ResultParameter {
            name: p.name.clone(),
            value: overrides.get(&p.name).copied().unwrap_or(p.default),
            unit: p.unit.clone(),
            description: p.description.clone(),
        })
        .collect()
}

/// `--list-parameters`: DAE まで変換して、tunable なパラメータを表示する（検査の Error では止めない）。
fn list_parameters(
    args: &SimulateArgs,
    texts: &std::collections::HashMap<String, String>,
    compiled: &CompiledModel,
) -> ExitCode {
    let model = args.model.as_str();
    let (parameters, diagnostics) = match globe_mode_exec::tunable_parameters(compiled) {
        Ok(p) => (Some(p), Vec::new()),
        Err(d) => (None, d),
    };
    if args.json {
        let value = serde_json::json!({
            "model": model,
            "parameters": parameters,
            "diagnostics": diagnostics,
        });
        println!(
            "{}",
            serde_json::to_string_pretty(&value).expect("serialize parameters")
        );
    } else {
        print_diagnostics(&diagnostics, texts, false);
        if let Some(p) = &parameters {
            for line in parameter_lines(model, p) {
                println!("{line}");
            }
        } else {
            println!("{model}: パラメータの一覧を作れない");
        }
    }
    if parameters.is_some() {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}

pub(crate) fn simulate(args: SimulateArgs) -> ExitCode {
    let model = args.model.as_str();
    let Loaded { mut db, texts } = match load_db(&args.files, &args.library) {
        Ok(Ok(v)) => v,
        Ok(Err(diags)) => return report_early_failure(model, &diags, args.json),
        Err(code) => return code,
    };

    if args.list_parameters {
        return match db.compiled_model(model) {
            Ok(compiled) => list_parameters(&args, &texts, &compiled),
            Err(ds) => finish(
                &args,
                &texts,
                &ds,
                None,
                Some("DAE まで変換できないのでパラメータの一覧を作れない".to_string()),
            ),
        };
    }

    // assert_simulatable: S0〜S10 で Error があれば、--force が無い限り実行しない
    let checked = db.check_model(model);
    let mut diagnostics: Vec<Diagnostic> = checked.diagnostics.clone();
    if let Some(stage) = checked.failed_stage()
        && !args.force
    {
        return finish(
            &args,
            &texts,
            &diagnostics,
            None,
            Some(format!(
                "実行しない（{stage:?} {} で Error。--force で強行できる）",
                stage.title()
            )),
        );
    }
    let compiled = match db.compiled_model_with_overrides(model, &args.settings.parameters) {
        Ok(c) => c,
        Err(ds) => {
            diagnostics.extend(ds);
            return finish(
                &args,
                &texts,
                &diagnostics,
                None,
                Some("DAE まで変換できないので実行できない".to_string()),
            );
        }
    };
    let SimReport {
        diagnostics: ran,
        run,
        settings,
    } = globe_mode_exec::run(&compiled, &args.settings);
    diagnostics.extend(ran);

    if let (Some(run), Some(path)) = (&run, &args.output)
        && let Err(e) = write_result(run, path, args.format, &compiled)
    {
        eprintln!("error: 結果を {} に書けない: {e}", path.display());
        return ExitCode::from(2);
    }
    let refused = match (&run, &settings) {
        // 実行に失敗しても、使った設定は表示する
        (None, Some(s)) => Some(format!("実行に失敗した（{}）", settings_text(s))),
        _ => None,
    };
    finish(&args, &texts, &diagnostics, run.as_ref(), refused)
}

fn write_result(
    run: &SimRun,
    path: &Path,
    format: OutputFormat,
    compiled: &CompiledModel,
) -> std::io::Result<()> {
    // MAT にはパラメータの値を添える。実行系が返さなかったら tunable なパラメータから補う
    let filled;
    let run = if format == OutputFormat::Mat && run.parameters.is_empty() {
        match globe_mode_exec::tunable_parameters(compiled) {
            Ok(p) if !p.is_empty() => {
                filled = SimRun {
                    parameters: fallback_parameters(&p, &run.settings.parameters),
                    ..run.clone()
                };
                &filled
            }
            _ => run,
        }
    } else {
        run
    };
    let write = |out: &mut dyn Write| match format {
        OutputFormat::Csv => globe_mode_sim::store::write_csv(&run.trajectory, out),
        OutputFormat::Mat => globe_mode_sim::mat::write_mat4(run, out),
    };
    if path == Path::new("-") {
        let stdout = std::io::stdout();
        let mut out = std::io::BufWriter::new(stdout.lock());
        write(&mut out)?;
        out.flush()
    } else {
        let file = std::fs::File::create(path)?;
        let mut out = std::io::BufWriter::new(file);
        write(&mut out)?;
        out.flush()
    }
}

/// 結果を表示して終了コードを返す。`refused` は実行しなかった理由。
fn finish(
    args: &SimulateArgs,
    texts: &std::collections::HashMap<String, String>,
    diagnostics: &[Diagnostic],
    run: Option<&SimRun>,
    refused: Option<String>,
) -> ExitCode {
    let failed = run.is_none() || diagnostics.iter().any(Diagnostic::is_error);
    let model = args.model.as_str();
    // CSV を標準出力に書いたときは、ほかの表示を標準エラーに回す
    let to_stdout = args.output.as_deref() != Some(Path::new("-"));
    if args.json {
        let value = serde_json::json!({
            "model": model,
            "succeeded": !failed,
            "refused": refused,
            "settings": run.map(|r| &r.settings),
            "termination": run.map(|r| &r.termination),
            "stats": run.map(|r| &r.stats),
            "variables": run.map(|r| r.trajectory.variables.len()),
            "final": run.map(final_values),
            "diagnostics": diagnostics,
        });
        let text = serde_json::to_string_pretty(&value).expect("serialize simulation report");
        if to_stdout {
            println!("{text}");
        } else {
            eprintln!("{text}");
        }
    } else {
        let mut lines = Vec::new();
        if let Some(reason) = &refused {
            lines.push(format!("{model}: {reason}"));
        }
        if let Some(run) = run {
            lines.extend(summary_lines(run));
        }
        if to_stdout {
            print_diagnostics(diagnostics, texts, false);
            for l in &lines {
                println!("{l}");
            }
        } else {
            for d in diagnostics {
                eprintln!(
                    "{}[{}] {:?} {}: {}",
                    severity(d),
                    d.code,
                    d.stage,
                    d.stage.title(),
                    d.message
                );
            }
            for l in &lines {
                eprintln!("{l}");
            }
        }
    }
    if failed {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    }
}

fn severity(d: &Diagnostic) -> String {
    if d.is_unsupported() {
        "unsupported".to_string()
    } else {
        serde_json::to_value(d.severity)
            .ok()
            .and_then(|v| v.as_str().map(str::to_owned))
            .unwrap_or_default()
    }
}

/// 設定の 1 行（値と出どころ）。
fn settings_text(s: &ResolvedSettings) -> String {
    let origin = |o: SettingOrigin| match o {
        SettingOrigin::Requested => "指定",
        SettingOrigin::Experiment => "experiment",
        SettingOrigin::Default => "既定",
    };
    let mut text = format!(
        "ソルバ {}（{}）・区間 {} → {}（{}・{}）・許容誤差 {:e}（{}）・出力の間隔 {}（{}）",
        s.solver.name(),
        origin(s.origins.solver),
        s.start_time,
        s.stop_time,
        origin(s.origins.start_time),
        origin(s.origins.stop_time),
        s.tolerance,
        origin(s.origins.tolerance),
        s.interval,
        origin(s.origins.interval),
    );
    if let Some(step) = s.step {
        let o = s.origins.step.map_or("既定", origin);
        text.push_str(&format!("・刻み幅 {step}（{o}）"));
    }
    if !s.parameters.is_empty() {
        let list: Vec<String> = s
            .parameters
            .iter()
            .map(|(name, value)| format!("{name} = {value}"))
            .collect();
        text.push_str(&format!("・パラメータの上書き {}", list.join(", ")));
    }
    text
}

/// 設定・統計・終わり方・最後の値の表示。
fn summary_lines(run: &SimRun) -> Vec<String> {
    let s = &run.settings;
    let mut lines = vec![
        format!("{}: {}", run.model, settings_text(s)),
        format!(
            "{}: 状態 {}・変数 {}・出力 {} 行・準備 {:.3} s・実行 {:.3} s",
            run.model,
            run.stats.states,
            run.trajectory.variables.len(),
            run.stats.points,
            run.stats.build_seconds,
            run.stats.run_seconds
        ),
    ];
    match &run.termination {
        Termination::StopTime => lines.push(format!(
            "{}: 終了時刻 {} まで進んだ",
            run.model, s.stop_time
        )),
        Termination::Terminated { time, message } => lines.push(format!(
            "{}: terminate() で t = {time} に終わった: {message}",
            run.model
        )),
    }
    let finals = final_values(run);
    if !finals.is_empty() {
        lines.push(format!(
            "{}: 最後の値（状態を先に、最大 10 個）:",
            run.model
        ));
        for (name, value) in finals.iter().take(10) {
            lines.push(format!("    {name} = {value}"));
        }
    }
    lines
}

/// 最後の時刻の値（状態を先に）。
fn final_values(run: &SimRun) -> Vec<(String, f64)> {
    let mut vars: Vec<_> = run.trajectory.variables.iter().collect();
    vars.sort_by_key(|v| v.role != VariableRole::State);
    vars.iter()
        .filter_map(|v| v.values.last().map(|x| (v.name.clone(), *x)))
        .collect()
}

#[cfg(test)]
mod tests {
    use globe_mode_ir::{SettingOrigins, SolverId};

    use super::*;

    fn strings(xs: &[&str]) -> Vec<String> {
        xs.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn params_are_name_equals_value() {
        let p = parse_params(&strings(&["k=2", " R = 1e3 ", "x[1]=-0.5", "a=nan"])).unwrap();
        assert_eq!(p.get("k"), Some(&2.0));
        assert_eq!(p.get("R"), Some(&1000.0));
        assert_eq!(p.get("x[1]"), Some(&-0.5));
        // 有限でない値は設定を決めるところ（GM-E10006）で報告する
        assert!(p.get("a").unwrap().is_nan());
        assert_eq!(
            parse_params(&strings(&["k"])).unwrap_err(),
            "--param k: 「名前=値」の形でない"
        );
        assert_eq!(
            parse_params(&strings(&["=1"])).unwrap_err(),
            "--param =1: 名前が空"
        );
        assert_eq!(
            parse_params(&strings(&["k=two"])).unwrap_err(),
            "--param k=two: 値「two」が数でない"
        );
        assert_eq!(
            parse_params(&strings(&["k=1", "k=2"])).unwrap_err(),
            "--param: パラメータ k を 2 回指定した"
        );
        assert!(parse_params(&[]).unwrap().is_empty());
    }

    #[test]
    fn output_format_follows_the_extension_unless_given() {
        let r = OutputFormat::resolve;
        assert_eq!(r(None, Some(Path::new("x.mat"))), OutputFormat::Mat);
        assert_eq!(r(None, Some(Path::new("dir/X.MAT"))), OutputFormat::Mat);
        assert_eq!(r(None, Some(Path::new("x.csv"))), OutputFormat::Csv);
        assert_eq!(r(None, Some(Path::new("-"))), OutputFormat::Csv);
        assert_eq!(r(None, None), OutputFormat::Csv);
        assert_eq!(
            r(Some(OutputFormat::Csv), Some(Path::new("x.mat"))),
            OutputFormat::Csv
        );
        assert_eq!(
            r(Some(OutputFormat::Mat), Some(Path::new("-"))),
            OutputFormat::Mat
        );
    }

    #[test]
    fn solver_table_lists_every_entry_with_its_status() {
        let lines = solver_lines();
        assert_eq!(lines.len(), SOLVERS.len() + 1);
        assert!(lines[0].starts_with("名前"));
        let line = |name: &str| {
            lines
                .iter()
                .find(|l| l.split_whitespace().next() == Some(name))
                .unwrap_or_else(|| panic!("{name}"))
                .clone()
        };
        assert!(line("bdf").contains("使える"));
        assert!(line("rk4").contains("使える（固定刻み）"));
        assert!(line("dassl").contains("別名 → bdf"));
        assert!(line("radau5").contains("使える"));
        assert!(line("ida").contains("未実装（代わり: bdf）"));
        assert!(line("qss").contains("使える"));
        // cvode は feature `sundials` を立てたビルドだけ使える（S032）
        if cfg!(feature = "sundials") {
            assert!(line("cvode").contains("使える"));
        } else {
            assert!(line("cvode").contains("未実装（代わり: bdf）"));
        }
        // 説明の列の始まりがそろう（全角を 2 と数える）
        let d = |l: &str| width(&l[..l.rfind("  ").unwrap()]);
        let first = d(&lines[1]);
        assert!(lines[1..].iter().all(|l| d(l) == first), "{lines:#?}");
    }

    fn settings(solver: SolverId, step: Option<f64>) -> ResolvedSettings {
        let o = SettingOrigin::Default;
        ResolvedSettings {
            start_time: 0.0,
            stop_time: 1.0,
            tolerance: 1e-6,
            interval: 0.002,
            solver,
            max_wall_seconds: None,
            step,
            parameters: BTreeMap::new(),
            origins: SettingOrigins {
                start_time: o,
                stop_time: SettingOrigin::Experiment,
                tolerance: o,
                interval: o,
                solver: SettingOrigin::Requested,
                step: step.map(|_| SettingOrigin::Requested),
            },
        }
    }

    #[test]
    fn settings_text_shows_step_and_overrides() {
        let s = settings(SolverId::Bdf, None);
        assert_eq!(
            settings_text(&s),
            "ソルバ bdf（指定）・区間 0 → 1（既定・experiment）・許容誤差 1e-6（既定）・出力の間隔 0.002（既定）"
        );
        let mut s = settings(SolverId::Rk4, Some(0.01));
        s.parameters.insert("k".into(), 2.0);
        s.parameters.insert("R".into(), 1000.0);
        assert!(
            settings_text(&s)
                .ends_with("・刻み幅 0.01（指定）・パラメータの上書き R = 1000, k = 2"),
            "{}",
            settings_text(&s)
        );
    }

    fn tunable(name: &str, default: f64) -> TunableParameter {
        TunableParameter {
            name: name.into(),
            default,
            unit: None,
            min: None,
            max: None,
            description: None,
        }
    }

    #[test]
    fn mat_parameters_fall_back_to_tunable_defaults_with_overrides() {
        let mut k = tunable("k", 1.0);
        k.unit = Some("1/s".into());
        let mut overrides = BTreeMap::new();
        overrides.insert("k".to_string(), 2.0);
        let p = fallback_parameters(&[k, tunable("c", 2.0)], &overrides);
        assert_eq!(p.len(), 2);
        assert_eq!((p[0].name.as_str(), p[0].value), ("k", 2.0));
        assert_eq!(p[0].unit.as_deref(), Some("1/s"));
        assert_eq!((p[1].name.as_str(), p[1].value), ("c", 2.0));
    }

    #[test]
    fn parameter_lines_show_unit_range_and_description() {
        let mut k = tunable("k", 1.0);
        k.unit = Some("1/s".into());
        k.min = Some(0.0);
        k.description = Some("rate".into());
        let lines = parameter_lines("M", &[k, tunable("c", 2.5)]);
        assert_eq!(
            lines,
            [
                "M: 再コンパイル無しに変えられるパラメータ 2 個（--param 名前=値 で上書きできる）",
                "    k = 1 [1/s]（範囲 0 〜 ∞）  \"rate\"",
                "    c = 2.5",
            ]
        );
    }
}

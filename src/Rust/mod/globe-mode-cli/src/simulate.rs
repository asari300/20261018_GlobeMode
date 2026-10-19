//! `globe simulate`（S011、ADR-0013）。検査（S0〜S10）を通してから実行し、結果を CSV に書く。

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use globe_mode_core::Diagnostic;
use globe_mode_exec::SimReport;
use globe_mode_ir::{
    ResolvedSettings, SettingOrigin, SimRun, SimSettings, Termination, VariableRole,
};

use super::{Loaded, load_db, print_diagnostics, report_early_failure};

/// `globe simulate` の引数（clap の外で受け渡す形）。
#[derive(Debug)]
pub(crate) struct SimulateArgs {
    pub files: Vec<PathBuf>,
    pub library: Vec<PathBuf>,
    pub model: String,
    pub settings: SimSettings,
    /// 結果の CSV の書き先（`-` は標準出力）
    pub output: Option<PathBuf>,
    /// S0〜S10 に Error があっても実行する
    pub force: bool,
    pub json: bool,
}

pub(crate) fn simulate(args: SimulateArgs) -> ExitCode {
    let model = args.model.as_str();
    let Loaded { mut db, texts } = match load_db(&args.files, &args.library) {
        Ok(Ok(v)) => v,
        Ok(Err(diags)) => return report_early_failure(model, &diags, args.json),
        Err(code) => return code,
    };

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
    let compiled = match db.compiled_model(model) {
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
        && let Err(e) = write_result(run, path)
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

fn write_result(run: &SimRun, path: &Path) -> std::io::Result<()> {
    if path == Path::new("-") {
        let stdout = std::io::stdout();
        let mut out = std::io::BufWriter::new(stdout.lock());
        globe_mode_sim::store::write_csv(&run.trajectory, &mut out)?;
        out.flush()
    } else {
        let file = std::fs::File::create(path)?;
        let mut out = std::io::BufWriter::new(file);
        globe_mode_sim::store::write_csv(&run.trajectory, &mut out)?;
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
    format!(
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
    )
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

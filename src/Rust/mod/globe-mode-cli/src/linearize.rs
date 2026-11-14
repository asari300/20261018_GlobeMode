//! `globe linearize`（S254）。モデルを指定時刻で線形化し、状態空間の行列 A・B・C・D を出す。

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::process::ExitCode;

use globe_mode::Linearization;

use super::{Loaded, failure_code, load_db, print_diagnostics, report_early_failure};

pub(crate) struct LinearizeArgs {
    pub files: Vec<PathBuf>,
    pub library: Vec<PathBuf>,
    pub model: String,
    pub time: Option<f64>,
    pub params: BTreeMap<String, f64>,
    pub json: bool,
}

pub(crate) fn linearize(args: LinearizeArgs) -> ExitCode {
    let model = args.model.as_str();
    let Loaded { mut ws, texts } = match load_db(&args.files, &args.library) {
        Ok(Ok(v)) => v,
        Ok(Err(diags)) => return report_early_failure(model, &diags, args.json, false),
        Err(code) => return code,
    };
    match ws.linearize(model, &args.params, args.time) {
        Ok(l) => {
            if args.json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&globe_mode::linearization_json(model, &l))
                        .expect("serialize linearization")
                );
            } else {
                for line in lines(model, &l) {
                    println!("{line}");
                }
            }
            ExitCode::SUCCESS
        }
        Err(diags) => {
            print_diagnostics(&diags, &texts, args.json);
            failure_code(&diags)
        }
    }
}

fn lines(model: &str, l: &Linearization) -> Vec<String> {
    let mut out = vec![format!(
        "{model}: t = {} で線形化（状態 {}・入力 {}・出力 {}）",
        l.time,
        l.states.len(),
        l.inputs.len(),
        l.outputs.len()
    )];
    let mut block = |title: &str, rows: &[String], cols: &[String], m: &[Vec<f64>]| {
        out.push(format!(
            "{title}（行: {} / 列: {}）",
            names(rows),
            names(cols)
        ));
        if m.is_empty() || cols.is_empty() {
            out.push("  （空）".to_string());
        }
        for row in m.iter().filter(|_| !cols.is_empty()) {
            out.push(format!(
                "  [{}]",
                row.iter()
                    .map(|v| format!("{v:>12.6}"))
                    .collect::<Vec<_>>()
                    .join(" ")
            ));
        }
    };
    block("A = df/dx", &l.states, &l.states, &l.a);
    block("B = df/du", &l.states, &l.inputs, &l.b);
    block("C = dy/dx", &l.outputs, &l.states, &l.c);
    block("D = dy/du", &l.outputs, &l.inputs, &l.d);
    // S286: A の固有値（極）と安定性の一言。状態が 0 個なら出さない
    let eigs = globe_mode::eigenvalues(&l.a);
    if !eigs.is_empty() {
        let list: Vec<String> = eigs
            .iter()
            .map(|&(re, im)| {
                if im == 0.0 {
                    format!("{re:.6}")
                } else {
                    format!("{re:.6}{im:+.6}i")
                }
            })
            .collect();
        out.push(format!("固有値（A）: {}", list.join(", ")));
        out.push(
            match globe_mode::stability(&eigs) {
                "stable" => "安定（すべて Re < 0）",
                "unstable" => "不安定（Re > 0 の固有値がある）",
                _ => "境界（Re ≈ 0 の固有値を含む）",
            }
            .to_string(),
        );
    }
    out
}

fn names(v: &[String]) -> String {
    if v.is_empty() {
        "-".to_string()
    } else {
        v.join(", ")
    }
}

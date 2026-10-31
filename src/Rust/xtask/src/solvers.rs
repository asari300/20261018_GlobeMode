//! `cargo xtask msl-solvers`: ソルバごとの `msl-sim --json` を集め、実行できた本数と管に収まった本数の表を書く（S050）。
//!
//! 入力は `msl-sim` が書いた JSON（`settings.solver`・`summary`・`total_seconds` だけを読む）。ソルバは JSON の
//! `settings.solver` から決め、`auto` の行（ふつうの `msl-sim` の結果）を先頭に、残りは名前順に並べる。

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::Result;

/// 1 ソルバの行（`msl-sim` の JSON の集計から取り出した数）。
#[derive(Debug, Clone, PartialEq)]
struct Row {
    solver: String,
    total: usize,
    simulated: usize,
    compared: usize,
    passed: usize,
    unsupported: usize,
    init: usize,
    integration: usize,
    event: usize,
    timeout: usize,
    hard_timeout: usize,
    other: usize,
    seconds: f64,
}

fn count(v: &Value, key: &str) -> usize {
    v.get(key).and_then(Value::as_u64).unwrap_or(0) as usize
}

fn row_of(root: &Value) -> Option<Row> {
    let solver = root.pointer("/settings/solver")?.as_str()?.to_string();
    let s = root.get("summary")?;
    let by_code: BTreeMap<&str, usize> = s
        .get("failed_by_code")
        .and_then(Value::as_object)
        .map(|m| {
            m.iter()
                .map(|(k, v)| (k.as_str(), v.as_u64().unwrap_or(0) as usize))
                .collect()
        })
        .unwrap_or_default();
    let code = |c: &str| by_code.get(c).copied().unwrap_or(0);
    let total = count(s, "total");
    let simulated = count(s, "stop_time") + count(s, "terminated");
    let hard_timeout = count(s, "hard_timeouts");
    let (unsupported, init, integration, event, timeout) = (
        code("GM-U0004"),
        code("GM-E11001"),
        code("GM-E11002"),
        code("GM-E11003"),
        code("GM-E11005"),
    );
    let named = simulated + unsupported + init + integration + event + timeout + hard_timeout;
    Some(Row {
        solver,
        total,
        simulated,
        compared: count(s, "compared"),
        passed: count(s, "ref_passed"),
        unsupported,
        init,
        integration,
        event,
        timeout,
        hard_timeout,
        other: total.saturating_sub(named),
        seconds: root
            .get("total_seconds")
            .and_then(Value::as_f64)
            .unwrap_or(0.0),
    })
}

fn render(rows: &[Row]) -> String {
    let mut out = String::new();
    out.push_str("# MSL 4.1.0 の例題のソルバ別の結果（globe-mode-exec）\n\n");
    out.push_str("`cargo xtask msl-solvers` が生成した。手で編集しない。ソルバごとに `cargo xtask msl-sim --solver <名前> --tolerance-scale 0.1` を回し、`msl-funnel` で OK の例題を実行して MAP-LIB の参照結果と矩形の管で比べた結果（GitHub Actions の `msl-sim.yml` のジョブ `solvers`）。\n\n");
    out.push_str("| ソルバ | 対象 | 実行できた | 管に収まった / 比べた | 割合 | GM-U0004 | 初期化 | 積分 | イベント | 時間切れ | 外から停止 | その他 | 秒 |\n");
    out.push_str("| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |\n");
    for r in rows {
        let rate = if r.compared == 0 {
            "-".to_string()
        } else {
            format!("{:.1}%", 100.0 * r.passed as f64 / r.compared as f64)
        };
        let _ = writeln!(
            out,
            "| `{}` | {} | {} | {} / {} | {} | {} | {} | {} | {} | {} | {} | {} | {:.0} |",
            r.solver,
            r.total,
            r.simulated,
            r.passed,
            r.compared,
            rate,
            r.unsupported,
            r.init,
            r.integration,
            r.event,
            r.timeout,
            r.hard_timeout,
            r.other,
            r.seconds
        );
    }
    out.push_str("\n止まった理由の列は、最初の Error のコードで分ける: 初期化は `GM-E11001`、積分は `GM-E11002`、イベントは `GM-E11003`、時間切れは `GM-E11005`（協調型。予算 30 s、回し直しは 180 s）、外から停止は親がワーカーを止めたもの。その他はコンパイル・S10・panic・`GM-U0005` など。`GM-U0004` はそのソルバが扱えない系（未対応）。`auto` の行は BDF が失敗したら dopri45 で回し直す。\n");
    out
}

/// 入力の JSON を行にする。同じソルバが 2 つあれば、あとのものを使う。
fn collect(inputs: &[PathBuf]) -> Result<Vec<Row>> {
    let mut rows: BTreeMap<String, Row> = BTreeMap::new();
    for p in inputs {
        let text = fs::read_to_string(p).map_err(|e| format!("{}: {e}", p.display()))?;
        let root: Value = serde_json::from_str(&text)
            .map_err(|e| format!("{} の JSON が壊れている: {e}", p.display()))?;
        let row = row_of(&root)
            .ok_or_else(|| format!("{} に settings.solver か summary が無い", p.display()))?;
        rows.insert(row.solver.clone(), row);
    }
    let mut rows: Vec<Row> = rows.into_values().collect();
    rows.sort_by_key(|r| (r.solver != "auto", r.solver.clone()));
    Ok(rows)
}

pub(crate) fn msl_solvers(
    inputs: &[PathBuf],
    report: Option<&Path>,
) -> Result<std::process::ExitCode> {
    let rows = collect(inputs)?;
    let text = render(&rows);
    match report {
        Some(path) => {
            if let Some(dir) = path.parent() {
                fs::create_dir_all(dir).map_err(|e| e.to_string())?;
            }
            fs::write(path, &text).map_err(|e| format!("{}: {e}", path.display()))?;
            println!("{} ソルバの表を {} に書いた", rows.len(), path.display());
        }
        None => print!("{text}"),
    }
    Ok(std::process::ExitCode::SUCCESS)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn json(solver: &str, simulated: u64, passed: u64, compared: u64) -> Value {
        serde_json::json!({
            "settings": { "solver": solver },
            "total_seconds": 12.4,
            "summary": {
                "total": 10, "stop_time": simulated, "terminated": 0,
                "hard_timeouts": 1, "compared": compared, "ref_passed": passed,
                "failed_by_code": { "GM-U0004": 2, "GM-E11002": 1, "GM-E11005": 1 }
            }
        })
    }

    #[test]
    fn rows_count_the_reasons_and_sort_auto_first() {
        let a = row_of(&json("tsit45", 5, 4, 5)).unwrap();
        assert_eq!(
            (a.simulated, a.unsupported, a.integration, a.timeout),
            (5, 2, 1, 1)
        );
        assert_eq!((a.hard_timeout, a.other), (1, 0));
        let dir = std::env::temp_dir().join(format!("gm-solvers-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let (p1, p2) = (dir.join("a.json"), dir.join("b.json"));
        fs::write(&p1, json("tsit45", 5, 4, 5).to_string()).unwrap();
        fs::write(&p2, json("auto", 6, 6, 6).to_string()).unwrap();
        let rows = collect(&[p1, p2]).unwrap();
        assert_eq!(rows[0].solver, "auto");
        let text = render(&rows);
        assert!(
            text.contains("| `auto` | 10 | 6 | 6 / 6 | 100.0% |"),
            "{text}"
        );
        assert!(
            text.contains("| `tsit45` | 10 | 5 | 4 / 5 | 80.0% | 2 | 0 | 1 | 0 | 1 | 1 | 0 | 12 |"),
            "{text}"
        );
        let _ = fs::remove_dir_all(dir);
    }
}

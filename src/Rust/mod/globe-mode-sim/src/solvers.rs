//! ソルバの名前の登録表（doc/architecture.md §3.3）。
//!
//! 名前は 3 種類ある。
//!
//! - 実装のあるもの（`bdf`、`dopri45`、`auto`）
//! - 別名（`dassl`、`lsodar` → `bdf`、`rk45`、`dopri5` → `dopri45`）。同等の手法に置き換えるが、
//!   刻み幅の制御などが違うので結果は一致しない
//! - 登録表にはあるが、まだ実装していないもの（`esdirk34`、`radau5` など）
//!
//! 表に無い名前は、編集距離の近い名前を 3 つまで挙げる。

use globe_mode_ir::SolverId;
use serde::Serialize;

/// 名前を引いた結果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SolverLookup {
    /// 実装のあるソルバ
    Available(SolverId),
    /// 別名（例: `dassl` → `bdf`。同等の手法で、結果は一致しない）
    Alias { requested: String, id: SolverId },
    /// 登録表にはあるが、まだ実装していない（例: `esdirk34`）
    Planned { name: String },
    /// 名前が無い。`suggestions` は近い名前
    Unknown {
        name: String,
        suggestions: Vec<&'static str>,
    },
}

/// 登録表の 1 行の状態。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", content = "solver", rename_all = "snake_case")]
pub enum SolverStatus {
    /// 実装がある
    Available(SolverId),
    /// 別名。中身のソルバに置き換える
    Alias(SolverId),
    /// 計画だけあって、まだ実装していない
    Planned,
}

/// 登録表の 1 行。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct SolverEntry {
    /// GlobeMode での名前（小文字）
    pub name: &'static str,
    pub status: SolverStatus,
    /// 短い説明（CLI の一覧に出す）
    pub description: &'static str,
    /// まだ実装していないものの、いま代わりに使える近い手法（無ければ None）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instead: Option<SolverId>,
}

const fn available(id: SolverId, description: &'static str) -> SolverEntry {
    SolverEntry {
        name: id.name(),
        status: SolverStatus::Available(id),
        description,
        instead: None,
    }
}

const fn alias(name: &'static str, id: SolverId, description: &'static str) -> SolverEntry {
    SolverEntry {
        name,
        status: SolverStatus::Alias(id),
        description,
        instead: None,
    }
}

const fn planned(
    name: &'static str,
    description: &'static str,
    instead: Option<SolverId>,
) -> SolverEntry {
    SolverEntry {
        name,
        status: SolverStatus::Planned,
        description,
        instead,
    }
}

const BDF: Option<SolverId> = Some(SolverId::Bdf);
const DOPRI45: Option<SolverId> = Some(SolverId::Dopri45);

/// ソルバの登録表（doc/architecture.md §3.3 の表の順。実装のあるもの、別名、計画の順に並べる）。
pub const SOLVERS: &[SolverEntry] = &[
    available(SolverId::Bdf, "可変次数の BDF（diffsol。既定）"),
    available(
        SolverId::Dopri45,
        "Dormand–Prince 5(4)（陽的、刻み幅の制御あり。Rumoca の RK45）",
    ),
    available(SolverId::Auto, "実行系が系を見て BDF か陽的 RK を選ぶ"),
    alias(
        "dassl",
        SolverId::Bdf,
        "OpenModelica・Dymola の DASSL の別名（同等の手法で、結果は一致しない）",
    ),
    alias(
        "lsodar",
        SolverId::Bdf,
        "Dymola の Lsodar の別名（同等の手法で、結果は一致しない）",
    ),
    alias(
        "rk45",
        SolverId::Dopri45,
        "Rumoca の RK45 の別名（Dormand–Prince 5(4)）",
    ),
    alias("dopri5", SolverId::Dopri45, "Dormand–Prince 5(4) の別名"),
    planned("ida", "SUNDIALS の IDA（feature `sundials`。未実装）", BDF),
    planned(
        "cvode",
        "SUNDIALS の CVODE（feature `sundials`。未実装）",
        BDF,
    ),
    planned("esdirk34", "ESDIRK 3(4)（diffsol の SDIRK。未実装）", BDF),
    planned("trbdf2", "TR-BDF2（diffsol の SDIRK。未実装）", BDF),
    planned("esdirk23", "ESDIRK 2(3)（自前の係数表。未実装）", BDF),
    planned("esdirk45", "ESDIRK 4(5)（自前の係数表。未実装）", BDF),
    planned(
        "sdirk34hw",
        "SDIRK 3(4)（Hairer–Wanner の係数。未実装）",
        BDF,
    ),
    planned("tsit45", "Tsitouras 5(4)（diffsol。未実装）", DOPRI45),
    planned("dopri853", "Dormand–Prince 8(5,3)（自前。未実装）", DOPRI45),
    planned("euler", "陽的 Euler 法（固定刻み。未実装）", DOPRI45),
    planned("heun", "Heun 法（固定刻み。未実装）", DOPRI45),
    planned(
        "rk4",
        "古典的な 4 段の Runge–Kutta 法（固定刻み。未実装）",
        DOPRI45,
    ),
    planned("impeuler", "陰的 Euler 法（固定刻み。未実装）", BDF),
    planned("trapezoid", "台形公式（固定刻み。未実装）", BDF),
    planned("radau5", "Radau IIA 5 次（自前。P8 の予定。未実装）", BDF),
    planned("cerk23", "連続の陽的 RK 2(3)（後回し）", DOPRI45),
    planned("cerk34", "連続の陽的 RK 3(4)（後回し）", DOPRI45),
    planned("cerk45", "連続の陽的 RK 4(5)（後回し）", DOPRI45),
    planned("qss", "量子化状態法 QSS（後回し）", None),
];

/// 近い名前として挙げる数の上限。
const MAX_SUGGESTIONS: usize = 3;

/// 名前を引く（前後の空白を除き、大文字と小文字は区別しない）。
pub fn lookup(name: &str) -> SolverLookup {
    let trimmed = name.trim();
    let key = trimmed.to_ascii_lowercase();
    match entry(&key) {
        Some(e) => match e.status {
            SolverStatus::Available(id) => SolverLookup::Available(id),
            SolverStatus::Alias(id) => SolverLookup::Alias {
                requested: trimmed.to_string(),
                id,
            },
            SolverStatus::Planned => SolverLookup::Planned {
                name: e.name.to_string(),
            },
        },
        None => SolverLookup::Unknown {
            name: trimmed.to_string(),
            suggestions: suggestions(&key),
        },
    }
}

/// 登録表の行を名前（小文字）で引く。
pub fn entry(name: &str) -> Option<&'static SolverEntry> {
    SOLVERS.iter().find(|e| e.name == name)
}

/// 実装のあるソルバの名前（表の順）。
pub fn available_names() -> impl Iterator<Item = &'static str> {
    SOLVERS
        .iter()
        .filter(|e| matches!(e.status, SolverStatus::Available(_)))
        .map(|e| e.name)
}

/// 編集距離の近い名前を [`MAX_SUGGESTIONS`] 個まで返す（距離の小さい順、同じなら表の順）。
/// 距離が名前の長さに比べて大きすぎるもの（`max(2, 長さ / 3)` を超えるもの）は挙げない。
fn suggestions(key: &str) -> Vec<&'static str> {
    if key.is_empty() {
        return Vec::new();
    }
    let limit = (key.chars().count() / 3).max(2);
    let mut scored: Vec<(usize, usize, &'static str)> = SOLVERS
        .iter()
        .enumerate()
        .map(|(i, e)| (levenshtein(key, e.name), i, e.name))
        .filter(|&(d, _, _)| d <= limit)
        .collect();
    scored.sort_unstable();
    scored
        .into_iter()
        .take(MAX_SUGGESTIONS)
        .map(|(_, _, name)| name)
        .collect()
}

/// 文字単位の Levenshtein 距離（挿入・削除・置換をどれも 1 とする）。
fn levenshtein(a: &str, b: &str) -> usize {
    let b: Vec<char> = b.chars().collect();
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    let mut cur = vec![0; b.len() + 1];
    for (i, ca) in a.chars().enumerate() {
        cur[0] = i + 1;
        for (j, &cb) in b.iter().enumerate() {
            let substitute = prev[j] + usize::from(ca != cb);
            cur[j + 1] = substitute.min(prev[j + 1] + 1).min(cur[j] + 1);
        }
        std::mem::swap(&mut prev, &mut cur);
    }
    prev[b.len()]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn available_names_are_found() {
        assert_eq!(lookup("bdf"), SolverLookup::Available(SolverId::Bdf));
        assert_eq!(
            lookup("dopri45"),
            SolverLookup::Available(SolverId::Dopri45)
        );
        assert_eq!(lookup("auto"), SolverLookup::Available(SolverId::Auto));
        assert_eq!(
            available_names().collect::<Vec<_>>(),
            ["bdf", "dopri45", "auto"]
        );
    }

    #[test]
    fn lookup_ignores_case_and_surrounding_space() {
        assert_eq!(lookup(" BDF "), SolverLookup::Available(SolverId::Bdf));
        assert_eq!(
            lookup("Dassl"),
            SolverLookup::Alias {
                requested: "Dassl".into(),
                id: SolverId::Bdf
            }
        );
    }

    #[test]
    fn aliases_map_to_implemented_solvers() {
        for (name, id) in [
            ("dassl", SolverId::Bdf),
            ("lsodar", SolverId::Bdf),
            ("rk45", SolverId::Dopri45),
            ("dopri5", SolverId::Dopri45),
        ] {
            assert_eq!(
                lookup(name),
                SolverLookup::Alias {
                    requested: name.into(),
                    id
                },
                "{name}"
            );
        }
    }

    #[test]
    fn planned_solvers_are_recognised() {
        for name in [
            "ida",
            "cvode",
            "esdirk34",
            "trbdf2",
            "esdirk23",
            "esdirk45",
            "sdirk34hw",
            "tsit45",
            "dopri853",
            "euler",
            "heun",
            "rk4",
            "impeuler",
            "trapezoid",
            "radau5",
            "cerk23",
            "cerk34",
            "cerk45",
            "qss",
        ] {
            assert_eq!(
                lookup(&name.to_uppercase()),
                SolverLookup::Planned { name: name.into() },
                "{name}"
            );
        }
    }

    #[test]
    fn table_names_are_unique_lowercase_and_aliases_point_to_available() {
        for (i, e) in SOLVERS.iter().enumerate() {
            assert_eq!(e.name, e.name.to_ascii_lowercase());
            assert!(
                SOLVERS[i + 1..].iter().all(|o| o.name != e.name),
                "{} が重複",
                e.name
            );
            assert!(!e.description.is_empty());
            if let SolverStatus::Alias(id) = e.status {
                assert!(available_names().any(|n| n == id.name()));
            }
            if !matches!(e.status, SolverStatus::Planned) {
                assert_eq!(e.instead, None);
            }
        }
    }

    #[test]
    fn unknown_names_get_close_suggestions() {
        assert_eq!(
            lookup("dasl"),
            SolverLookup::Unknown {
                name: "dasl".into(),
                suggestions: vec!["dassl"],
            }
        );
        // 距離 1 の bdf が先、同じ距離なら表の順
        match lookup("bdf2") {
            SolverLookup::Unknown { suggestions, .. } => {
                assert_eq!(suggestions[0], "bdf");
                assert!(suggestions.len() <= MAX_SUGGESTIONS);
            }
            other => panic!("{other:?}"),
        }
        match lookup("esdirk") {
            SolverLookup::Unknown { suggestions, .. } => {
                assert_eq!(suggestions, ["esdirk34", "esdirk23", "esdirk45"]);
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn far_names_get_no_suggestions() {
        assert_eq!(
            lookup("newton-krylov"),
            SolverLookup::Unknown {
                name: "newton-krylov".into(),
                suggestions: vec![],
            }
        );
        assert_eq!(
            lookup(""),
            SolverLookup::Unknown {
                name: String::new(),
                suggestions: vec![],
            }
        );
    }

    #[test]
    fn levenshtein_distance() {
        assert_eq!(levenshtein("", ""), 0);
        assert_eq!(levenshtein("abc", ""), 3);
        assert_eq!(levenshtein("kitten", "sitting"), 3);
        assert_eq!(levenshtein("dopri", "dopri45"), 2);
    }
}

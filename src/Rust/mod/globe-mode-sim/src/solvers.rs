//! ソルバの名前の登録表（doc/architecture.md §3.3）。
//!
//! 名前は 3 種類ある。
//!
//! - 実装のあるもの（`bdf`、`dopri45`、`auto`。S012 から `esdirk34`、`trbdf2`、`tsit45`、`euler`、`heun`、`rk4`。S026 から `impeuler`、`trapezoid`。S027 から `esdirk23`、`esdirk45`、`sdirk34hw`。S028 から `dopri853`。S029 から `cerk23`、`cerk34`、`cerk45`。S030 から `radau5`。S031 から `qss`。feature `sundials` のとき S032 から `cvode`、S033 から `ida`）
//! - 別名（`dassl`、`lsodar` → `bdf`、`rk45`、`dopri5` → `dopri45`）。同等の手法に置き換えるが、
//!   刻み幅の制御などが違うので結果は一致しない
//! - 登録表にはあるが、まだ実装していないもの（feature `sundials` を立てないときの `ida`・`cvode`。立てると無い）
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
    /// 登録表にはあるが、まだ実装していない（例: feature `sundials` を立てないときの `ida`）
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

// feature `sundials` を立てると計画だけのソルバは無い
#[cfg(not(feature = "sundials"))]
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

#[cfg(not(feature = "sundials"))]
const BDF: Option<SolverId> = Some(SolverId::Bdf);

/// ソルバの登録表（doc/architecture.md §3.3 の表の順。実装のあるもの、別名、計画の順に並べる）。
pub const SOLVERS: &[SolverEntry] = &[
    available(SolverId::Bdf, "可変次数の BDF（diffsol。既定）"),
    available(
        SolverId::Dopri45,
        "Dormand–Prince 5(4)（陽的、刻み幅の制御あり。Rumoca の RK45）",
    ),
    available(
        SolverId::Auto,
        "実行系が系を見て BDF か陽的 RK を選ぶ（BDF が積分で失敗したら dopri45 で回し直す）",
    ),
    available(
        SolverId::Esdirk34,
        "ESDIRK 3(4)（diffsol の SDIRK。陰的、刻み幅の制御あり）",
    ),
    available(
        SolverId::Trbdf2,
        "TR-BDF2（diffsol の SDIRK。2 次、L 安定。陰的、刻み幅の制御あり）",
    ),
    available(
        SolverId::Tsit45,
        "Tsitouras 5(4)（diffsol の陽的 RK、刻み幅の制御あり）",
    ),
    available(
        SolverId::Euler,
        "陽的 Euler 法（固定刻み、1 次。刻み幅は --step、既定は出力の間隔。刻みの間は 3 次の Hermite 補間）",
    ),
    available(
        SolverId::Heun,
        "Heun 法（固定刻み、2 次。刻み幅は --step、既定は出力の間隔。刻みの間は 3 次の Hermite 補間）",
    ),
    available(
        SolverId::Rk4,
        "古典的な 4 段の Runge–Kutta 法（固定刻み、4 次。刻み幅は --step、既定は出力の間隔。刻みの間は 3 次の Hermite 補間）",
    ),
    available(
        SolverId::Impeuler,
        "陰的 Euler 法（固定刻み、1 次、L 安定。刻み幅は --step、既定は出力の間隔。1 歩ごとに Newton 法。刻みの間は 3 次の Hermite 補間）",
    ),
    available(
        SolverId::Trapezoid,
        "台形公式（固定刻み、2 次、A 安定。刻み幅は --step、既定は出力の間隔。1 歩ごとに Newton 法。刻みの間は 3 次の Hermite 補間）",
    ),
    available(
        SolverId::Esdirk23,
        "ESDIRK 2(3)（3 段、2 次、L 安定。Jørgensen らの係数を diffsol の SDIRK に差す。陰的、刻み幅の制御あり）",
    ),
    available(
        SolverId::Esdirk45,
        "ESDIRK 5(4)（7 段、5 次、L 安定。Kennedy–Carpenter の ESDIRK5(4)7L[2]SA2 を diffsol の SDIRK に差す。陰的、刻み幅の制御あり）",
    ),
    available(
        SolverId::Sdirk34hw,
        "SDIRK 4(3)（5 段、4 次、L 安定。Hairer–Wanner の SDIRK4 を diffsol の SDIRK に差す。陰的、刻み幅の制御あり）",
    ),
    available(
        SolverId::Dopri853,
        "DOP853（13 段の FSAL、8 次。Hairer らの係数を diffsol の陽的 RK に差す。陽的、刻み幅の制御あり。刻み幅は 5 次の見積もりで決める）",
    ),
    available(
        SolverId::Cerk23,
        "CERK 3(2)（Owren–Zennaro。4 段の FSAL、3 次、刻みの間も 3 次の補間。係数を diffsol の陽的 RK に差す。陽的、刻み幅の制御あり）",
    ),
    available(
        SolverId::Cerk34,
        "CERK 4(3)（Owren–Zennaro。6 段の FSAL、4 次、刻みの間も 4 次の補間。係数を diffsol の陽的 RK に差す。陽的、刻み幅の制御あり）",
    ),
    available(
        SolverId::Cerk45,
        "CERK 5(4)（Owren–Zennaro。8 段の FSAL、5 次、刻みの間も 5 次の補間。係数を diffsol の陽的 RK に差す。陽的、刻み幅の制御あり）",
    ),
    available(
        SolverId::Radau5,
        "Radau IIA（3 段、5 次、L 安定。自前の簡略 Newton 法と HW の埋め込みの誤差の見積もり。陰的、刻み幅の制御あり。刻みの間は 3 次の Hermite 補間）",
    ),
    available(
        SolverId::Qss,
        "量子化状態法 QSS2（自前。状態と量子化した状態の差が量子に達した事象ごとに進む。陽的、2 次、誤差は量子に比例。硬い系は苦手）",
    ),
    #[cfg(feature = "sundials")]
    available(
        SolverId::Cvode,
        "SUNDIALS の CVODE（BDF、可変次数 1〜5、可変刻み。密な線形ソルバと差分商のヤコビ行列。陰的、刻み幅の制御あり。feature `sundials`）",
    ),
    #[cfg(feature = "sundials")]
    available(
        SolverId::Ida,
        "SUNDIALS の IDA（可変係数の BDF、可変次数 1〜5、可変刻み。残差 y' − f を解く。密な線形ソルバと差分商のヤコビ行列。陰的、刻み幅の制御あり。feature `sundials`）",
    ),
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
    #[cfg(not(feature = "sundials"))]
    planned(
        "ida",
        "SUNDIALS の IDA（feature `sundials` を立てたビルドで使える。このビルドでは未実装）",
        BDF,
    ),
    #[cfg(not(feature = "sundials"))]
    planned(
        "cvode",
        "SUNDIALS の CVODE（feature `sundials` を立てたビルドで使える。このビルドでは未実装）",
        BDF,
    ),
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
            lookup("esdirk34"),
            SolverLookup::Available(SolverId::Esdirk34)
        );
        assert_eq!(lookup("RK4"), SolverLookup::Available(SolverId::Rk4));
        assert_eq!(
            lookup("SDIRK34HW"),
            SolverLookup::Available(SolverId::Sdirk34hw)
        );
        assert_eq!(
            lookup("DOPRI853"),
            SolverLookup::Available(SolverId::Dopri853)
        );
        assert_eq!(lookup("CERK34"), SolverLookup::Available(SolverId::Cerk34));
        assert_eq!(lookup("Radau5"), SolverLookup::Available(SolverId::Radau5));
        assert_eq!(lookup("QSS"), SolverLookup::Available(SolverId::Qss));
        assert_eq!(
            available_names().collect::<Vec<_>>(),
            [
                "bdf",
                "dopri45",
                "auto",
                "esdirk34",
                "trbdf2",
                "tsit45",
                "euler",
                "heun",
                "rk4",
                "impeuler",
                "trapezoid",
                "esdirk23",
                "esdirk45",
                "sdirk34hw",
                "dopri853",
                "cerk23",
                "cerk34",
                "cerk45",
                "radau5",
                "qss",
                #[cfg(feature = "sundials")]
                "cvode",
                #[cfg(feature = "sundials")]
                "ida"
            ]
        );
    }

    #[cfg(feature = "sundials")]
    #[test]
    fn cvode_and_ida_are_available_with_sundials() {
        assert_eq!(lookup("CVODE"), SolverLookup::Available(SolverId::Cvode));
        assert_eq!(lookup("IDA"), SolverLookup::Available(SolverId::Ida));
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
        // cvode・ida は feature `sundials` を立てると使える（上のテスト）。立てると計画だけのものは無い
        let names: &[&str] = if cfg!(feature = "sundials") {
            &[]
        } else {
            &["ida", "cvode"]
        };
        for &name in names {
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

//! Rumoca の DAE から、方程式 × 未知数の構造（[`DaeStructure`]、S6 の入力）を取り出す。
//!
//! Rumoca の incidence（`rumoca-phase-structural` の `build_incidence`）は公開されていないので、
//! 同じ手順を公開 API でなぞる（Rumoca v0.10.2 の `incidence.rs` と `incidence/projection.rs`）。
//!
//! - 未知数: 変数の順に、状態はスカラーごとの `der(x)`、Algebraic / Output はスカラーそのもの。
//!   状態・パラメータ・入力・離散変数・`pre()`・`time` は既知として数えない。
//! - 方程式: `continuous_owners()` の順。Residual は 1 行、構造化された族（for や配列の方程式）は
//!   定義域の点ごと・本体の式ごとに 1 行。
//! - 参照: `rumoca_eval_dae::for_each_scalar_coordinate_cached` で、1 行の残差が依存する
//!   スカラーの座標を列挙し、`der(x)` と Algebraic の座標を `incidence` に残す（Rumoca と同じ der の形）。
//!   状態そのもの（`x`）の座標は、その状態の `der(x)` の添字に写して `state_refs` に残す
//!   （S6 は `x` と `der(x)` を 1 つの列にまとめたグラフで判定する。`DaeStructure::merged_incidence`）。
//!
//! DAE の ID は `inspect` の閉包に縛られるので、閉包を出る前に添字と文字列に写す。

use std::collections::HashMap;

use globe_mode_core::TextRange;
use globe_mode_ir::{DaeStructure, EquationOrigin, EquationRow, Unknown, UnknownKind};
use rumoca_compile::compile::{
    ContinuousOwnerView, CoordinateView, Dae, DaeProvenance, DaeView, VariableRole,
};
use rumoca_eval_dae::{ScalarCoordinateProjectionCache, for_each_scalar_coordinate_cached};
use rumoca_ir_dae::{DaeGeneration, DaeProvenanceOrigin};

/// 表示用のラベルの長さの上限（文字数）。
const LABEL_CHARS: usize = 60;

/// 未知数の鍵（変数の添字、スカラーの添字）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Key {
    Derivative(u32, u32),
    Algebraic(u32, u32),
}

/// DAE の構造を取り出す。射影できない式があれば Err（理由）。panic しても Err にする。
pub(crate) fn extract(dae: &Dae) -> Result<DaeStructure, String> {
    let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        dae.inspect(|view| extract_view(dae, view))
    }));
    match caught {
        Ok(result) => result,
        Err(payload) => Err(format!(
            "射影の途中で panic した: {}",
            payload
                .downcast_ref::<&str>()
                .map(|s| s.to_string())
                .or_else(|| payload.downcast_ref::<String>().cloned())
                .unwrap_or_default()
        )),
    }
}

fn extract_view<'dae>(dae: &Dae, view: DaeView<'dae>) -> Result<DaeStructure, String> {
    let locate = |provenance: DaeProvenance| location(dae, provenance);

    // 未知数（Rumoca の build_unknowns と同じ順）
    let mut keys: HashMap<Key, u32> = HashMap::new();
    let mut unknowns = Vec::new();
    for (id, variable) in view.variables() {
        let kind = match variable.role() {
            VariableRole::State => UnknownKind::Derivative,
            VariableRole::Algebraic => UnknownKind::Algebraic,
            VariableRole::Output => UnknownKind::Output,
            VariableRole::Input
            | VariableRole::Parameter
            | VariableRole::Constant
            | VariableRole::DiscreteReal
            | VariableRole::DiscreteValue => continue,
        };
        let count = variable.value_type().scalar_count().ok_or_else(|| {
            format!(
                "変数 `{}` の大きさがコンパイル時に決まっていない",
                variable.name()
            )
        })?;
        let (file, range) = locate(variable.declaration());
        for scalar in 0..count {
            let scalar_u32 =
                u32::try_from(scalar).map_err(|_| "スカラーの添字が u32 に収まらない")?;
            let key = match kind {
                UnknownKind::Derivative => Key::Derivative(id.index(), scalar_u32),
                UnknownKind::Algebraic | UnknownKind::Output => {
                    Key::Algebraic(id.index(), scalar_u32)
                }
            };
            let base = variable
                .scalar_name(scalar)
                .unwrap_or_else(|| variable.name().to_string());
            let name = match kind {
                UnknownKind::Derivative => format!("der({base})"),
                UnknownKind::Algebraic | UnknownKind::Output => base,
            };
            keys.insert(key, unknowns.len() as u32);
            unknowns.push(Unknown {
                name,
                kind,
                file: file.clone(),
                range,
            });
        }
    }

    // 方程式（Rumoca の visit_owner_rows と同じ順）
    let mut equations = Vec::new();
    let mut incidence = Vec::new();
    let mut state_refs = Vec::new();
    let mut cache = ScalarCoordinateProjectionCache::default();
    let mut push_row = |expression,
                        scalar: usize,
                        point: Option<(_, &[i64])>,
                        provenance: DaeProvenance,
                        suffix: Option<&[i64]>|
     -> Result<(), String> {
        let mut row: Vec<u32> = Vec::new();
        // 状態そのもの（x）を参照しているときの、その状態の der(x) の添字
        let mut states: Vec<u32> = Vec::new();
        for_each_scalar_coordinate_cached(
            view,
            expression,
            scalar,
            point,
            &mut cache,
            |coordinate, scalar| {
                let Ok(scalar) = u32::try_from(scalar) else {
                    return;
                };
                match coordinate {
                    CoordinateView::Derivative(state) => {
                        if let Some(&u) = keys.get(&Key::Derivative(state.index(), scalar)) {
                            row.push(u);
                        }
                    }
                    CoordinateView::Algebraic(variable) => {
                        if let Some(&u) = keys.get(&Key::Algebraic(variable.index(), scalar)) {
                            row.push(u);
                        }
                    }
                    CoordinateView::State(state) => {
                        if let Some(&u) = keys.get(&Key::Derivative(state.index(), scalar)) {
                            states.push(u);
                        }
                    }
                    // 既知の値（パラメータ・入力・離散・pre・time など）
                    _ => {}
                }
            },
        )
        .map_err(|e| e.to_string())?;
        row.sort_unstable();
        row.dedup();
        states.sort_unstable();
        states.dedup();
        let index = equations.len();
        let (file, range) = locate(provenance);
        equations.push(EquationRow {
            label: label(view, provenance, index, suffix),
            origin: origin(provenance),
            file,
            range,
        });
        incidence.push(row);
        state_refs.push(states);
        Ok(())
    };

    for owner in view.continuous_owners() {
        match owner {
            ContinuousOwnerView::Residual { equation, .. } => {
                push_row(equation.residual(), 0, None, equation.provenance(), None)?;
            }
            ContinuousOwnerView::Structured { family, .. } => {
                let domain = view
                    .domain(family.domain())
                    .ok_or("構造化された方程式の定義域が見つからない")?;
                for point in 0..domain.scalar_count() as usize {
                    let values = domain
                        .structured()
                        .index_tuple_at(point)
                        .map_err(|e| format!("定義域の点を作れない: {e:?}"))?
                        .ok_or("定義域の点が範囲の外")?;
                    let scalar = family
                        .scalar_view()
                        .body_scalar(point, domain.extents())
                        .ok_or("族の本体のスカラーを決められない")?;
                    for expression in family.bodies().iter() {
                        push_row(
                            expression,
                            scalar,
                            Some((family.domain(), &values)),
                            family.provenance(),
                            Some(&values),
                        )?;
                    }
                }
            }
        }
    }

    Ok(DaeStructure {
        unknowns,
        equations,
        incidence,
        state_refs,
    })
}

/// 位置（ファイル名とバイト範囲）。範囲がその本文の中に収まるときだけ返す。
pub(crate) fn location(
    dae: &Dae,
    provenance: DaeProvenance,
) -> (Option<String>, Option<TextRange>) {
    span_location(dae.source_map(), provenance.span())
}

/// Rumoca の Span を、ファイル名とバイト範囲に直す（`rumoca_compile::compile::source_span_location`
/// と同じ検査。行と列ではなくバイト位置のまま返す）。
pub(crate) fn span_location(
    source_map: &rumoca_core::SourceMap,
    span: rumoca_core::Span,
) -> (Option<String>, Option<TextRange>) {
    if span.is_dummy() {
        return (None, None);
    }
    let Some((file, source)) = source_map.get_source(span.source) else {
        return (None, None);
    };
    let (start, end) = (span.start.0, span.end.0);
    let range =
        (start < end && end <= source.len()).then(|| TextRange::new(start as u32, end as u32));
    (Some(file.to_string()), range)
}

/// 表示用の短いラベル: ソースの一部（1 行、60 文字まで）。取れなければ `f[i]`（1 始まり）。
/// 族の行には定義域の点を `[i,j]` で添える。
pub(crate) fn label(
    view: DaeView<'_>,
    provenance: DaeProvenance,
    index: usize,
    point: Option<&[i64]>,
) -> String {
    let excerpt = view
        .source_text(provenance)
        .map(|t| t.split_whitespace().collect::<Vec<_>>().join(" "))
        .filter(|t| !t.is_empty());
    let mut label = match excerpt {
        Some(t) if t.chars().count() > LABEL_CHARS => {
            let cut: String = t.chars().take(LABEL_CHARS - 1).collect();
            format!("{cut}…")
        }
        Some(t) => t,
        None => format!("f[{}]", index + 1),
    };
    if let Some(point) = point {
        let point: Vec<String> = point.iter().map(i64::to_string).collect();
        label = format!("{label} [{}]", point.join(","));
    }
    label
}

pub(crate) fn origin(provenance: DaeProvenance) -> EquationOrigin {
    match provenance.origin() {
        DaeProvenanceOrigin::Source
        // 書かれた配列やレコードの方程式を、スカラーに分けたもの
        | DaeProvenanceOrigin::Generated(
            DaeGeneration::ArrayEquationProjection | DaeGeneration::RecordEquationProjection,
        ) => EquationOrigin::Source,
        DaeProvenanceOrigin::Generated(DaeGeneration::ConnectionEquation) => {
            EquationOrigin::Connection
        }
        DaeProvenanceOrigin::Generated(DaeGeneration::FlowBalanceEquation) => {
            EquationOrigin::FlowBalance
        }
        DaeProvenanceOrigin::Generated(DaeGeneration::BindingEquation) => EquationOrigin::Binding,
        DaeProvenanceOrigin::Generated(_) => EquationOrigin::Generated,
    }
}

#[cfg(test)]
mod tests {
    use rumoca_compile::{Session, SessionConfig};

    use super::*;

    fn examples() -> String {
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../../Modelica/GlobeModeExamples/package.mo");
        std::fs::read_to_string(path).unwrap()
    }

    /// 最大マッチング（照合用の素朴な増加路法）。
    fn max_matching(n_var: usize, incidence: &[Vec<u32>]) -> usize {
        fn augment(
            e: usize,
            incidence: &[Vec<u32>],
            seen: &mut [bool],
            owner: &mut [Option<usize>],
        ) -> bool {
            for &u in &incidence[e] {
                let u = u as usize;
                if seen[u] {
                    continue;
                }
                seen[u] = true;
                if owner[u].is_none_or(|o| augment(o, incidence, seen, owner)) {
                    owner[u] = Some(e);
                    return true;
                }
            }
            false
        }
        let mut owner = vec![None; n_var];
        (0..incidence.len())
            .filter(|&e| augment(e, incidence, &mut vec![false; n_var], &mut owner))
            .count()
    }

    #[test]
    fn rc_structure_matches_rumoca_structural_analysis() {
        let mut session = Session::new(SessionConfig::default());
        session.add_document("package.mo", &examples()).unwrap();
        let compilation = session
            .compile_model_strict("GlobeModeExamples.RC")
            .expect("RC compiles");
        let dae = &compilation.result().dae;
        let s = extract(dae).expect("structure");
        let reference = dae.inspect(rumoca_compile::phase_structural::analyze);

        assert_eq!((s.unknowns.len(), s.equations.len()), (20, 20));
        assert_eq!(s.incidence.len(), s.equations.len());
        assert_eq!(
            (s.equations.len(), s.unknowns.len()),
            (reference.n_equations, reference.n_unknowns)
        );
        assert_eq!(
            max_matching(s.unknowns.len(), &s.incidence),
            reference.matching_size
        );
        assert_eq!(reference.matching_size, 20);
        for (e, row) in s.incidence.iter().enumerate() {
            assert!(
                !row.is_empty(),
                "{} が未知数を含まない",
                s.equations[e].label
            );
            assert!(row.windows(2).all(|w| w[0] < w[1]), "昇順・重複なし");
        }
        // 状態はコンデンサの電圧 1 つ。その微分が未知数に入る
        let ders: Vec<&str> = s
            .unknowns
            .iter()
            .filter(|u| u.kind == UnknownKind::Derivative)
            .map(|u| u.name.as_str())
            .collect();
        assert_eq!(ders, ["der(c.v)"]);
        // 位置はソースの中を指す（ファイル名は文書の名前）
        let text = examples();
        for e in &s.equations {
            assert_eq!(e.file.as_deref(), Some("package.mo"));
            let r = e.range.expect("range");
            assert!((r.end as usize) <= text.len());
        }
        assert!(
            s.equations
                .iter()
                .any(|e| e.origin == EquationOrigin::Connection)
        );
        assert!(
            s.equations
                .iter()
                .any(|e| e.origin == EquationOrigin::FlowBalance)
        );
        assert!(s.equations.iter().any(|e| e.label == "v = R * i"));
        // 状態そのもの（c.v）を参照する方程式（例: v = p.v - n.v）は、state_refs に der(c.v) を持つ
        let der = s
            .unknowns
            .iter()
            .position(|u| u.name == "der(c.v)")
            .unwrap() as u32;
        assert_eq!(s.state_refs.len(), s.equations.len());
        assert!(s.state_refs.iter().any(|r| r == &[der]));
        assert!(s.state_refs.iter().flatten().all(|&u| u == der));
    }

    #[test]
    fn array_equations_expand_per_scalar() {
        let text = "model Arr\n  Real x[3](each start = 1);\n  Real y[3];\nequation\n  for i in 1:3 loop\n    der(x[i]) = -y[i];\n  end for;\n  y = 2 * x;\nend Arr;\n";
        let mut session = Session::new(SessionConfig::default());
        session.add_document("Arr.mo", text).unwrap();
        let compilation = session.compile_model_strict("Arr").expect("Arr compiles");
        let dae = &compilation.result().dae;
        let s = extract(dae).expect("structure");
        let reference = dae.inspect(rumoca_compile::phase_structural::analyze);
        assert_eq!((s.unknowns.len(), s.equations.len()), (6, 6));
        assert_eq!(
            (
                reference.n_equations,
                reference.n_unknowns,
                reference.matching_size
            ),
            (6, 6, max_matching(6, &s.incidence))
        );
        assert!(s.unknowns.iter().any(|u| u.name == "der(x[2])"));
        assert!(s.incidence.iter().all(|r| !r.is_empty()));
    }
}

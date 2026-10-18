//! Rumoca の DAE から、初期化系の構造（[`InitStructure`]、S8 の入力）を取り出す（S006）。
//!
//! - 状態: `VariableRole::State` の変数を、変数の順にスカラーごとに 1 つ。`structure.rs` の
//!   `Derivative` の未知数（`der(x)`）と同じ順になる。`fixed` の既定は false（MLS §4.8）。
//! - パラメータ: `VariableRole::Parameter`（`Constant` は除く）。`fixed` の既定は true。束縛の有無も持つ。
//! - 離散変数: `DiscreteReal` / `DiscreteValue`。`fixed` の既定は false。
//! - 代数変数: `Algebraic` / `Output`。`fixed` の既定は false（`fixed = true` なら初期条件が 1 つ増える）。
//! - 初期方程式: `initialization_owners()` を `structure.rs` と同じ要領でスカラーの行に展開する
//!   （Residual は 1 行、構造化された族は定義域の点ごと・本体の式ごとに 1 行）。
//!   Rumoca が初期方程式（と初期アルゴリズム）から取り分ける定義も、対象のスカラーごとに 1 行として足す:
//!   - Real でない `fixed = false` のパラメータを決める式（`initial_parameter_values()`。Rumoca は初期化の
//!     方程式として扱う）
//!   - 離散変数の、初期化の時点の値を決める式や代入（`initial_discrete_values()`）
//!
//! Rumoca v0.10.2 の DAE への変換（`compile_model_strict`）について、テストで確かめたこと:
//!
//! - `fixed = true` の `start` を初期方程式には**変えない**（`initialization_owners()` に現れない。
//!   その変換はシミュレーションの準備（solve）の側にある）。だから S8 は `fixed = true` の状態を
//!   初期方程式とは別に数えればよく、2 重には数えない。
//! - `start()` は、何も書かなくても**ほぼ常に Some**。既定の start（0 など）を補うため:
//!   - 組み込みの型（`Real x;`）は、宣言そのもの（`Real x` の位置）を指す式で補う。
//!   - start の無い型（`type V = Real(unit = "V")`）は、`DaeGeneration::DefaultStart` の式で補う。
//!
//!   そこで「start が書いてある」は、start の式が宣言と同じ位置ではなく、`DefaultStart` でもないこと、とする
//!   （[`explicit_start`]）。型から受け継いだ start（`type V = Real(start = 1)`）、パラメータを使った式
//!   （`start = p`）、外からの修正（`Sub s(x(start = 2))`）は、どれも書いてあるとみなされる。
//! - 束縛のあるパラメータ（`parameter Real p = 2`）は、束縛が start にも写される（MLS §4.4.4）。
//! - 初期アルゴリズムで決める `fixed = false` のパラメータ（`p := 2`）は、束縛のあるパラメータになる。
//! - 初期方程式のうち離散変数を決めるもの（`initial_discrete_values()`）は、`initialization_owners()` に
//!   入らない（連続の変数を読む離散 Real の式は入る）。S8 は離散変数を数えるので、上のとおり足す。
//!
//! 取り出せないときは Err（理由）。panic しても Err にする。

use globe_mode_ir::{EquationRow, InitStructure, InitVariable};
use rumoca_compile::compile::{Dae, DaeProvenance, DaeView, VariableRole, VariableView};
use rumoca_ir_dae::{DaeGeneration, DaeProvenanceOrigin, InitializationOwnerView};

use crate::structure::{label, location, origin};

/// 初期化系の構造を取り出す。
pub(crate) fn extract(dae: &Dae) -> Result<InitStructure, String> {
    let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        dae.inspect(|view| extract_view(dae, view))
    }));
    match caught {
        Ok(result) => result,
        Err(payload) => Err(format!(
            "初期化系を取り出す途中で panic した: {}",
            payload
                .downcast_ref::<&str>()
                .map(|s| s.to_string())
                .or_else(|| payload.downcast_ref::<String>().cloned())
                .unwrap_or_default()
        )),
    }
}

fn extract_view<'dae>(dae: &Dae, view: DaeView<'dae>) -> Result<InitStructure, String> {
    let mut init = InitStructure::default();

    // 変数（変数の順、スカラーごと。structure.rs の未知数と同じ順）
    for (_, variable) in view.variables() {
        let role = variable.role();
        let (list, default_fixed) = match role {
            VariableRole::State => (&mut init.states, false),
            VariableRole::Parameter => (&mut init.parameters, true),
            VariableRole::DiscreteReal | VariableRole::DiscreteValue => {
                (&mut init.discretes, false)
            }
            VariableRole::Algebraic | VariableRole::Output => (&mut init.algebraics, false),
            VariableRole::Input | VariableRole::Constant => continue,
        };
        let count = variable.value_type().scalar_count().ok_or_else(|| {
            format!(
                "変数 `{}` の大きさがコンパイル時に決まっていない",
                variable.name()
            )
        })?;
        let (file, range) = location(dae, variable.declaration());
        let has_start = explicit_start(view, variable);
        // 束縛はパラメータだけが意味を持つ（状態・離散・代数は常に false）
        let has_binding = role == VariableRole::Parameter && variable.binding().is_some();
        for scalar in 0..count {
            list.push(InitVariable {
                name: variable
                    .scalar_name(scalar)
                    .unwrap_or_else(|| variable.name().to_string()),
                fixed: variable.fixed_scalar(scalar).unwrap_or(default_fixed),
                has_start,
                has_binding,
                file: file.clone(),
                range,
            });
        }
    }

    // 初期方程式（Rumoca の visit_initialization_rows と同じ順）
    let mut equations: Vec<EquationRow> = Vec::new();
    let mut push_row = |provenance: DaeProvenance, suffix: Option<&[i64]>| {
        let index = equations.len();
        let (file, range) = location(dae, provenance);
        equations.push(EquationRow {
            label: label(view, provenance, index, suffix),
            origin: origin(provenance),
            file,
            range,
        });
    };
    for owner in view.initialization_owners() {
        match owner {
            InitializationOwnerView::Residual { equation, .. } => {
                push_row(equation.provenance(), None);
            }
            InitializationOwnerView::Structured { family, .. } => {
                let domain = view
                    .domain(family.domain())
                    .ok_or("構造化された初期方程式の定義域が見つからない")?;
                for point in 0..domain.scalar_count() as usize {
                    let values = domain
                        .structured()
                        .index_tuple_at(point)
                        .map_err(|e| format!("定義域の点を作れない: {e:?}"))?
                        .ok_or("定義域の点が範囲の外")?;
                    for _ in family.bodies().iter() {
                        push_row(family.provenance(), Some(&values));
                    }
                }
            }
        }
    }
    // Rumoca が初期化の行から取り分けた定義: Real でない fixed = false のパラメータの値と、
    // 離散変数の初期化の時点の値。どちらも対象のスカラーごとに 1 行
    let definitions: Vec<_> = view
        .initial_parameter_values()
        .map(|d| (d.target(), d.provenance()))
        .chain(
            view.initial_discrete_values()
                .map(|d| (d.target(), d.provenance())),
        )
        .collect();
    for (target, provenance) in definitions {
        let target = view
            .variable(target)
            .ok_or("初期化で値を決める変数が見つからない")?;
        let count = target.value_type().scalar_count().ok_or_else(|| {
            format!(
                "変数 `{}` の大きさがコンパイル時に決まっていない",
                target.name()
            )
        })?;
        for scalar in 0..count {
            let point = [scalar as i64 + 1];
            push_row(provenance, (count > 1).then_some(&point[..]));
        }
    }
    init.initial_equations = equations;
    Ok(init)
}

/// `start` が書いてある（修飾子か、型の宣言から受け継いだもの）。
///
/// Rumoca は start を書かない変数にも既定の start を補う。補った式は、宣言と同じ位置を指すか
/// （組み込みの型）、`DaeGeneration::DefaultStart` の由来を持つ（start の無い型）ので、それを除く。
fn explicit_start<'dae>(view: DaeView<'dae>, variable: VariableView<'dae>) -> bool {
    let Some(start) = variable.start().and_then(|id| view.expression(id)) else {
        return false;
    };
    let provenance = start.provenance();
    let defaulted = matches!(
        provenance.origin(),
        DaeProvenanceOrigin::Generated(DaeGeneration::DefaultStart)
    ) || provenance.span() == variable.declaration().span();
    !defaulted
}

#[cfg(test)]
mod tests {
    use rumoca_compile::{Session, SessionConfig};

    use super::*;

    fn compile(name: &str, text: &str) -> InitStructure {
        let mut session = Session::new(SessionConfig::default());
        session.add_document("M.mo", text).unwrap();
        let compilation = session
            .compile_model_strict(name)
            .unwrap_or_else(|_| panic!("{name} compiles"));
        extract(&compilation.result().dae).expect("init")
    }

    /// (名前, fixed, has_start, has_binding)
    fn flags(vars: &[InitVariable]) -> Vec<(&str, bool, bool, bool)> {
        vars.iter()
            .map(|v| (v.name.as_str(), v.fixed, v.has_start, v.has_binding))
            .collect()
    }

    #[test]
    fn fixed_start_is_not_turned_into_an_initial_equation() {
        // Rumoca の DAE への変換は、fixed = true の start を初期方程式にしない（S8 で 2 重に数えない）
        let text =
            "model A\n  Real x(start = 1, fixed = true);\nequation\n  der(x) = -x;\nend A;\n";
        let init = compile("A", text);
        assert_eq!(flags(&init.states), [("x", true, true, false)]);
        assert!(init.initial_equations.is_empty());
        let r = init.states[0].range.expect("宣言の位置");
        assert_eq!(&text[r.start as usize..r.end as usize], "Real x");
    }

    #[test]
    fn default_start_is_not_counted_as_written() {
        // 何も書かなければ、Rumoca が補う既定の start は「書いてある」に数えない
        let text = "model B\n  type U = Real(unit = \"V\");\n  Real x;\n  U y;\n  Real z(start = 0);\nequation\n  der(x) = -x;\n  der(y) = -y;\n  der(z) = -z;\nend B;\n";
        let init = compile("B", text);
        assert_eq!(
            flags(&init.states),
            [
                ("x", false, false, false),
                ("y", false, false, false),
                ("z", false, true, false)
            ]
        );
    }

    #[test]
    fn inherited_and_expression_starts_count_as_written() {
        let text = "model C\n  type V = Real(start = 1);\n  type W = Real(fixed = true, start = 2);\n  model Sub\n    Real a;\n    V b;\n  equation\n    der(a) = -a;\n    der(b) = -b;\n  end Sub;\n  parameter Real p = 2;\n  V x;\n  W w;\n  Real y(start = p);\n  Sub s(a(start = 3));\n  Sub t;\nequation\n  der(x) = -x;\n  der(w) = -w;\n  der(y) = -y;\nend C;\n";
        let init = compile("C", text);
        assert_eq!(
            flags(&init.states),
            [
                ("x", false, true, false),
                ("w", true, true, false),
                ("y", false, true, false),
                ("s.a", false, true, false),
                ("s.b", false, true, false),
                ("t.a", false, false, false),
                ("t.b", false, true, false)
            ]
        );
        assert_eq!(flags(&init.parameters), [("p", true, true, true)]);
    }

    #[test]
    fn parameters_keep_fixed_start_and_binding() {
        let text = "model P\n  parameter Real a;\n  parameter Real b(start = 3);\n  parameter Real c = 1;\n  parameter Real d(fixed = false);\n  parameter Real e(fixed = false, start = 1);\n  constant Real k = 2;\n  Real x(start = 1, fixed = true);\ninitial equation\n  d = 2 * a;\ninitial algorithm\n  e := 2;\nequation\n  der(x) = -(a + b + c + d + e + k) * x;\nend P;\n";
        let init = compile("P", text);
        assert_eq!(
            flags(&init.parameters),
            [
                ("a", true, false, false),
                ("b", true, true, false),
                ("c", true, true, true),
                ("d", false, false, false),
                // 初期アルゴリズムで決めるパラメータは、束縛のあるパラメータになる
                ("e", false, true, true)
            ]
        );
        assert_eq!(init.initial_equations.len(), 1);
        assert_eq!(init.initial_equations[0].label, "d = 2 * a");
    }

    #[test]
    fn non_real_free_parameter_equation_is_an_initial_equation() {
        // Rumoca は Real でない fixed = false のパラメータを決める式を、初期化の行から取り分ける
        let text = "model H\n  parameter Integer n(fixed = false);\n  Real x(start = 1, fixed = true);\ninitial equation\n  n = 3;\nequation\n  der(x) = -n * x;\nend H;\n";
        let init = compile("H", text);
        assert_eq!(flags(&init.parameters), [("n", false, false, false)]);
        assert_eq!(init.initial_equations.len(), 1);
        assert_eq!(init.initial_equations[0].label, "n = 3");
        assert!(init.initial_equations[0].range.is_some());
    }

    #[test]
    fn discretes_and_algebraics_are_listed_apart() {
        let text = "model K\n  discrete Real d(start = 1, fixed = true);\n  Integer k;\n  Real x(start = 1, fixed = true);\n  Real y(start = 2, fixed = true);\n  output Real z;\nequation\n  der(x) = -x;\n  y = 2 * x;\n  z = y + 1;\n  when sample(0, 0.1) then\n    d = pre(d) + 1;\n    k = pre(k) + 1;\n  end when;\nend K;\n";
        let init = compile("K", text);
        assert_eq!(
            flags(&init.discretes),
            [("d", true, true, false), ("k", false, false, false)]
        );
        assert_eq!(
            flags(&init.algebraics),
            [("y", true, true, false), ("z", false, false, false)]
        );
        assert_eq!(flags(&init.states), [("x", true, true, false)]);
        assert!(init.initial_equations.is_empty());
    }

    #[test]
    fn initial_definitions_of_discretes_are_initial_equations() {
        // k = 0（定数で決まる離散値）は Rumoca が initial_discrete_values() に取り分け、
        // d = x（連続の変数を読む離散 Real）は初期化の行に残る。どちらも初期方程式として数える。
        // 初期アルゴリズムの代入（b := true）も、離散変数の初期化の時点の値として数える
        let text = "model D
  discrete Real d;
  Integer k;
  Boolean b;
  Real x(start = 1, fixed = true);
initial equation
  k = 0;
  d = x;
initial algorithm
  b := true;
equation
  der(x) = -x;
  when sample(0, 0.1) then
    d = x;
    k = pre(k) + 1;
    b = not pre(b);
  end when;
end D;
";
        let init = compile("D", text);
        assert_eq!(init.discretes.len(), 3);
        let mut labels: Vec<&str> = init
            .initial_equations
            .iter()
            .map(|e| e.label.as_str())
            .collect();
        labels.sort_unstable();
        // 初期アルゴリズムの代入の位置は、代入先（`b`）を指す
        assert_eq!(labels, ["b", "d = x", "k = 0"]);
        assert!(init.initial_equations.iter().all(|e| e.range.is_some()));
    }

    #[test]
    fn array_initial_equations_expand_per_scalar() {
        let text = "model Arr\n  Real x[3];\ninitial equation\n  for i in 1:3 loop\n    x[i] = i;\n  end for;\nequation\n  der(x) = -x;\nend Arr;\n";
        let init = compile("Arr", text);
        let names: Vec<&str> = init.states.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(names, ["x[1]", "x[2]", "x[3]"]);
        assert_eq!(init.initial_equations.len(), 3);
        assert!(init.initial_equations.iter().all(|e| e.range.is_some()));
    }

    #[test]
    fn array_equation_in_initial_section_counts_each_scalar() {
        let text = "model ArrEq\n  Real x[2];\ninitial equation\n  x = {1, 2};\nequation\n  der(x) = -x;\nend ArrEq;\n";
        let init = compile("ArrEq", text);
        assert_eq!(init.states.len(), 2);
        assert_eq!(init.initial_equations.len(), 2);
    }

    #[test]
    fn per_element_fixed_is_read_per_scalar() {
        let text = "model Each\n  Real x[2](start = {1, 2}, fixed = {true, false});\nequation\n  der(x) = -x;\nend Each;\n";
        let init = compile("Each", text);
        assert_eq!(
            flags(&init.states),
            [("x[1]", true, true, false), ("x[2]", false, true, false)]
        );
    }
}

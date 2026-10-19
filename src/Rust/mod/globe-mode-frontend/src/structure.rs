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
//! - 非線形（S010）: 残差の式木を歩き、未知数がその式について 1 次かを**族（変数）の単位で**判定して、
//!   行の `incidence` のうち非線形の族に属するものを `nonlinear` に残す（[`Linearity`]）。
//!   スカラーの添字は見ないので、同じ族の別の要素が非線形に現れれば、線形の要素も非線形にする（広め）。
//!
//! DAE の ID は `inspect` の閉包に縛られるので、閉包を出る前に添字と文字列に写す。

use std::collections::HashMap;

use globe_mode_core::TextRange;
use globe_mode_ir::{DaeStructure, EquationOrigin, EquationRow, Unknown, UnknownKind};
use rumoca_compile::compile::{
    ContinuousOwnerView, CoordinateView, Dae, DaeProvenance, DaeView, VariableRole,
};
use rumoca_eval_dae::{ScalarCoordinateProjectionCache, for_each_scalar_coordinate_cached};
use rumoca_ir_dae::{
    BinaryOperator, DaeGeneration, DaeLiteral, DaeProvenanceOrigin, ExprId, ExpressionOperands,
    ExpressionOperation, ExpressionVariability, PureBuiltin, SubscriptView, SubscriptsView,
    UnaryOperator,
};

/// 表示用のラベルの長さの上限（文字数）。
const LABEL_CHARS: usize = 60;

/// 未知数の鍵（変数の添字、スカラーの添字）。`init.rs` も同じ鍵で初期方程式の参照を写す。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum Key {
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

/// 未知数（Rumoca の build_unknowns と同じ順）と、その鍵から添字への対応。
///
/// `init.rs` も同じ対応を使うので、[`DaeStructure::unknowns`] の添字と初期方程式の参照が揃う。
pub(crate) fn unknowns_and_keys<'dae>(
    dae: &Dae,
    view: DaeView<'dae>,
) -> Result<(Vec<Unknown>, HashMap<Key, u32>), String> {
    let locate = |provenance: DaeProvenance| location(dae, provenance);
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
    Ok((unknowns, keys))
}

fn extract_view<'dae>(dae: &Dae, view: DaeView<'dae>) -> Result<DaeStructure, String> {
    let locate = |provenance: DaeProvenance| location(dae, provenance);
    let (unknowns, keys) = unknowns_and_keys(dae, view)?;
    // 未知数の添字から、その族へ
    let mut families = vec![Family::Derivative(0); unknowns.len()];
    for (key, &u) in &keys {
        families[u as usize] = match *key {
            Key::Derivative(v, _) => Family::Derivative(v),
            Key::Algebraic(v, _) => Family::Algebraic(v),
        };
    }
    let mut linearity = Linearity::new(view);

    // 方程式（Rumoca の visit_owner_rows と同じ順）
    let mut equations = Vec::new();
    let mut incidence = Vec::new();
    let mut state_refs = Vec::new();
    let mut nonlinear = Vec::new();
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
        // 族の単位の判定を、この行の参照（スカラー）に写す。式ごとに覚えるので、族の各点では歩き直さない
        let shape = linearity.root(expression);
        let row_nonlinear: Vec<u32> = row
            .iter()
            .copied()
            .filter(|&u| shape.opaque || shape.nonlinear.contains(&families[u as usize]))
            .collect();
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
        nonlinear.push(row_nonlinear);
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
        nonlinear,
    })
}

/// 未知数の族（変数の添字）。非線形の判定はこの単位で行う。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum Family {
    Derivative(u32),
    Algebraic(u32),
}

/// 1 つの部分式で、未知数の族がどう現れるか。
#[derive(Debug, Clone, Default)]
struct Shape {
    /// 現れる族（昇順、重複なし）
    all: Vec<Family>,
    /// そのうち非線形に現れる族（昇順、重複なし。`all` の部分集合）
    nonlinear: Vec<Family>,
    /// 中を歩けない式（関数の本体の値など）を含む。このときは行の参照すべてを非線形にする
    opaque: bool,
}

impl Shape {
    fn is_free(&self) -> bool {
        self.all.is_empty() && !self.opaque
    }

    /// 和（加減など）: 現れる族も、非線形の族も和集合。
    fn union(mut self, other: Shape) -> Shape {
        merge(&mut self.all, other.all);
        merge(&mut self.nonlinear, other.nonlinear);
        self.opaque |= other.opaque;
        self
    }

    /// 現れる族すべてを非線形にする。
    fn saturate(mut self) -> Shape {
        self.nonlinear.clone_from(&self.all);
        self
    }

    /// 真偽の値（関係演算など）。イベントで凍結されるなら、中の族はどれも非線形にしない。
    fn frozen(mut self) -> Shape {
        self.nonlinear.clear();
        self
    }
}

/// 昇順・重複なしの列の和集合を `into` に入れる。
fn merge(into: &mut Vec<Family>, other: Vec<Family>) {
    if other.is_empty() {
        return;
    }
    if into.is_empty() {
        *into = other;
        return;
    }
    into.extend(other);
    into.sort_unstable();
    into.dedup();
}

/// 残差の式木を歩き、未知数の族が 1 次に現れるか（線形か）を判定する（S010）。
///
/// 未知数は `der(x)` と Algebraic / Output（incidence の列になるもの）。状態そのもの・パラメータ・入力・
/// 離散変数・`pre()`・`time`・内包の添字は既知で、係数に入っても線形のまま。規則:
///
/// - 加減・単項の符号・配列とレコードの構築・添字・内包・`sum`・`transpose` などの線形の写像:
///   和集合（非線形の族も和集合）
/// - 積（`*`、`.*`、`cross`、`outerProduct`）: 両辺に未知数があれば両辺の族すべてが非線形。片方だけなら線形
/// - 商（`/`、`./`）: 分母に未知数があれば、分子・分母の族すべてが非線形
/// - 冪: 指数に未知数がある、または底に未知数があり指数がリテラルの 1 でなければ、すべてが非線形
/// - 組み込みの関数（`sin`・`exp`・`abs`・`min`・`max`・`sign`・`floor`・`product` など）と利用者の関数:
///   引数の族すべてが非線形（`abs`・`min`・`max` はイベントの間は区分的に線形だが、広めに見積もる）
/// - `noEvent(e)`・`smooth(n, e)`: `e` を素通しにする。ただし中の関係演算はイベントで凍結されないので、
///   その条件を持つ if 式は、条件と分岐の族すべてが非線形
/// - `homotopy(actual, simplified)`: 両方の和集合（λ は既知なので、両方が線形なら線形）
/// - if 式: 分岐は和集合。条件の関係演算はイベントで凍結される（イベントの間は定数）ので、
///   条件の族は非線形にしない。条件が凍結されない（`noEvent` の中、または真偽の値を関数で作る）ときは、
///   条件と分岐の族すべてが非線形
/// - 添字に未知数があるとき、判定できない式（関数の本体の値など）: すべてが非線形
///
/// 結果は式（と `noEvent` の中かどうか）ごとに覚える。族の各点で同じ本体を歩き直さず、
/// 共有された部分式も 1 回だけ歩く。
struct Linearity<'dae> {
    view: DaeView<'dae>,
    memo: HashMap<(u32, bool), Shape>,
}

impl<'dae> Linearity<'dae> {
    fn new(view: DaeView<'dae>) -> Self {
        Self {
            view,
            memo: HashMap::new(),
        }
    }

    /// 残差（1 行の根の式）の判定。
    fn root(&mut self, expression: ExprId<'dae>) -> Shape {
        self.shape(expression, false)
    }

    /// `no_event`: `noEvent` / `smooth` の中（関係演算がイベントで凍結されない）。
    fn shape(&mut self, expression: ExprId<'dae>, no_event: bool) -> Shape {
        let Some(node) = self.view.expression(expression) else {
            return Shape {
                opaque: true,
                ..Shape::default()
            };
        };
        // 連続でない式（定数・パラメータ・離散）は未知数を含まない
        if node.variability() != ExpressionVariability::Continuous {
            return Shape::default();
        }
        let key = (expression.index(), no_event);
        if let Some(shape) = self.memo.get(&key) {
            return shape.clone();
        }
        let shape = self.operation(node.operation(), no_event);
        self.memo.insert(key, shape.clone());
        shape
    }

    fn operation(&mut self, operation: ExpressionOperation<'dae>, no_event: bool) -> Shape {
        match operation {
            ExpressionOperation::Literal(_) | ExpressionOperation::Range(_) => Shape::default(),
            ExpressionOperation::Coordinate(coordinate) => match coordinate {
                CoordinateView::Derivative(state) => Shape {
                    all: vec![Family::Derivative(state.index())],
                    ..Shape::default()
                },
                CoordinateView::Algebraic(variable) => Shape {
                    all: vec![Family::Algebraic(variable.index())],
                    ..Shape::default()
                },
                // 関数の引数は関数の本体の中にしか現れない（本体は歩かない）
                CoordinateView::FunctionParameter(_) => Shape {
                    opaque: true,
                    ..Shape::default()
                },
                // 既知の値（状態そのもの・パラメータ・入力・離散・pre・time・内包の添字など）
                _ => Shape::default(),
            },
            ExpressionOperation::Unary { operator, operand } => {
                let shape = self.shape(operand, no_event);
                match operator {
                    UnaryOperator::Plus | UnaryOperator::Negate => shape,
                    UnaryOperator::Not => self.relation(shape, no_event),
                }
            }
            ExpressionOperation::Binary { operator, lhs, rhs } => {
                self.binary(operator, lhs, rhs, no_event)
            }
            ExpressionOperation::Conditional(operands) => self.conditional(operands, no_event),
            ExpressionOperation::Array(operands) | ExpressionOperation::Record(operands) => {
                self.operands(operands, no_event)
            }
            ExpressionOperation::Field { base, .. } => self.shape(base, no_event),
            ExpressionOperation::Comprehension { body, .. } => self.shape(body, no_event),
            ExpressionOperation::Index { base, subscripts } => {
                let base = self.shape(base, no_event);
                self.subscripted(base, subscripts, no_event)
            }
            ExpressionOperation::ArrayUpdate {
                base,
                value,
                subscripts,
            } => {
                let shape = self
                    .shape(base, no_event)
                    .union(self.shape(value, no_event));
                self.subscripted(shape, subscripts, no_event)
            }
            ExpressionOperation::Builtin { builtin, arguments } => {
                self.builtin(builtin, arguments, no_event)
            }
            ExpressionOperation::Call { arguments, .. } => {
                self.operands(arguments, no_event).saturate()
            }
            ExpressionOperation::StringConversion { value, .. } => {
                self.shape(value, no_event).saturate()
            }
            ExpressionOperation::ClockTransfer { source, .. } => self.shape(source, no_event),
            // 関数の本体の値（モデルの方程式には現れないはず）
            ExpressionOperation::FunctionValue { .. }
            | ExpressionOperation::FunctionFoldParameter { .. }
            | ExpressionOperation::FunctionFoldOutput { .. } => Shape {
                opaque: true,
                ..Shape::default()
            },
        }
    }

    /// 項の和集合。
    fn operands(&mut self, operands: ExpressionOperands<'dae>, no_event: bool) -> Shape {
        operands.iter().fold(Shape::default(), |acc, operand| {
            acc.union(self.shape(operand, no_event))
        })
    }

    /// 真偽の値。`noEvent` の中なら中の族すべてが非線形、そうでなければイベントで凍結される。
    fn relation(&self, shape: Shape, no_event: bool) -> Shape {
        if no_event {
            shape.saturate()
        } else {
            shape.frozen()
        }
    }

    /// 添字に未知数があれば、すべてを非線形にする。
    fn subscripted(
        &mut self,
        base: Shape,
        subscripts: SubscriptsView<'dae>,
        no_event: bool,
    ) -> Shape {
        let mut index = Shape::default();
        for subscript in subscripts.iter() {
            match subscript {
                SubscriptView::Index { expression, .. }
                | SubscriptView::Slice { expression, .. } => {
                    index = index.union(self.shape(expression, no_event));
                }
                SubscriptView::Whole { .. } => {}
            }
        }
        if index.is_free() {
            base
        } else {
            base.union(index).saturate()
        }
    }

    fn binary(
        &mut self,
        operator: BinaryOperator,
        lhs: ExprId<'dae>,
        rhs: ExprId<'dae>,
        no_event: bool,
    ) -> Shape {
        let left = self.shape(lhs, no_event);
        let right = self.shape(rhs, no_event);
        match operator {
            BinaryOperator::Add
            | BinaryOperator::Subtract
            | BinaryOperator::ElementwiseAdd
            | BinaryOperator::ElementwiseSubtract => left.union(right),
            BinaryOperator::Multiply | BinaryOperator::ElementwiseMultiply => product(left, right),
            BinaryOperator::Divide | BinaryOperator::ElementwiseDivide => {
                if right.is_free() {
                    left
                } else {
                    left.union(right).saturate()
                }
            }
            BinaryOperator::Power | BinaryOperator::ElementwisePower => {
                if right.is_free() && (left.is_free() || self.is_literal_one(rhs)) {
                    left
                } else {
                    left.union(right).saturate()
                }
            }
            BinaryOperator::Equal
            | BinaryOperator::NotEqual
            | BinaryOperator::Less
            | BinaryOperator::LessEqual
            | BinaryOperator::Greater
            | BinaryOperator::GreaterEqual
            | BinaryOperator::And
            | BinaryOperator::Or => self.relation(left.union(right), no_event),
        }
    }

    /// if 式（`[条件, 値, 条件, 値, …, それ以外]`）。
    fn conditional(&mut self, operands: ExpressionOperands<'dae>, no_event: bool) -> Shape {
        let mut branches = Shape::default();
        let mut conditions = Shape::default();
        let last = operands.len().saturating_sub(1);
        for (ordinal, operand) in operands.iter().enumerate() {
            let shape = self.shape(operand, no_event);
            if ordinal < last && ordinal % 2 == 0 {
                conditions = conditions.union(shape);
            } else {
                branches = branches.union(shape);
            }
        }
        // 凍結されない条件が未知数に依ると、分岐の切り替わりも反復の中で起こる
        if conditions.opaque || !conditions.nonlinear.is_empty() {
            branches.union(conditions).saturate()
        } else {
            branches.union(conditions.frozen())
        }
    }

    fn builtin(
        &mut self,
        builtin: PureBuiltin,
        arguments: ExpressionOperands<'dae>,
        no_event: bool,
    ) -> Shape {
        match builtin {
            PureBuiltin::NoEvent => match arguments.get(0) {
                Some(argument) => self.shape(argument, true),
                None => Shape::default(),
            },
            PureBuiltin::Smooth => match arguments.get(1) {
                Some(argument) => self.shape(argument, true),
                None => Shape::default(),
            },
            // 大きさだけに依る
            PureBuiltin::Size => Shape::default(),
            // 線形の写像（λ が既知の homotopy も含む）
            PureBuiltin::Homotopy
            | PureBuiltin::Sum
            | PureBuiltin::Zeros
            | PureBuiltin::Ones
            | PureBuiltin::Fill
            | PureBuiltin::Linspace
            | PureBuiltin::PromotedCat1
            | PureBuiltin::PromotedCat2
            | PureBuiltin::Identity
            | PureBuiltin::Vector
            | PureBuiltin::Transpose
            | PureBuiltin::Diagonal
            | PureBuiltin::Skew => self.operands(arguments, no_event),
            // 双線形
            PureBuiltin::Cross | PureBuiltin::OuterProduct => {
                let mut factors = arguments.iter();
                let left = factors
                    .next()
                    .map(|a| self.shape(a, no_event))
                    .unwrap_or_default();
                let right = factors.fold(Shape::default(), |acc, a| {
                    acc.union(self.shape(a, no_event))
                });
                product(left, right)
            }
            PureBuiltin::Abs
            | PureBuiltin::Sign
            | PureBuiltin::Sqrt
            | PureBuiltin::Div
            | PureBuiltin::Mod
            | PureBuiltin::Rem
            | PureBuiltin::Floor
            | PureBuiltin::Ceil
            | PureBuiltin::Integer
            | PureBuiltin::Sin
            | PureBuiltin::Cos
            | PureBuiltin::Tan
            | PureBuiltin::Asin
            | PureBuiltin::Acos
            | PureBuiltin::Atan
            | PureBuiltin::Atan2
            | PureBuiltin::Sinh
            | PureBuiltin::Cosh
            | PureBuiltin::Tanh
            | PureBuiltin::Exp
            | PureBuiltin::Log
            | PureBuiltin::Log10
            | PureBuiltin::Min
            | PureBuiltin::Max
            | PureBuiltin::Product
            | PureBuiltin::LinearSolve => self.operands(arguments, no_event).saturate(),
        }
    }

    /// 式がリテラルの 1（`x^1`）か。
    fn is_literal_one(&self, expression: ExprId<'dae>) -> bool {
        self.view.expression(expression).is_some_and(|node| {
            matches!(
                node.operation(),
                ExpressionOperation::Literal(DaeLiteral::Integer(1))
            ) || matches!(
                node.operation(),
                ExpressionOperation::Literal(DaeLiteral::Real(v)) if *v == 1.0
            )
        })
    }
}

/// 積: 両辺に未知数があれば両辺の族すべてが非線形、片方だけなら（係数が既知なので）そのまま。
fn product(left: Shape, right: Shape) -> Shape {
    if left.is_free() || right.is_free() {
        left.union(right)
    } else {
        left.union(right).saturate()
    }
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

    /// 1 つのモデルを文書から取り出す。
    fn structure_of(name: &str, text: &str) -> DaeStructure {
        let mut session = Session::new(SessionConfig::default());
        session.add_document(&format!("{name}.mo"), text).unwrap();
        let compilation = session
            .compile_model_strict(name)
            .unwrap_or_else(|e| panic!("{name} compiles: {e:?}"));
        extract(&compilation.result().dae).expect("structure")
    }

    /// ラベルが `label` の行の、非線形の未知数の名前
    fn nonlinear_names(s: &DaeStructure, label: &str) -> Vec<String> {
        let e = s
            .equations
            .iter()
            .position(|e| e.label == label)
            .unwrap_or_else(|| panic!("方程式 {label} が無い: {:?}", s.equations));
        s.nonlinear[e]
            .iter()
            .map(|&u| s.unknowns[u as usize].name.clone())
            .collect()
    }

    #[test]
    fn nonlinear_follows_the_rules_per_operation() {
        let text = "model Rules\n  parameter Real p = 2;\n  Real x(start = 1, fixed = true);\n  Real a, b, c, d, e, f, g, h, k, m, n, q, r;\nequation\n  der(x) = -p * x * a;\n  a = time;\n  b * a = 1;\n  c = a / p;\n  d = p / c;\n  e = a ^ 1;\n  f = sin(b);\n  g = if a > 0 then 2 * b else c;\n  h = noEvent(if a > 0 then b else c);\n  k = homotopy(a * b, a);\n  m = x * c;\n  n = c ^ 2 + d;\n  q = smooth(1, b * p);\n  r = abs(c);\nend Rules;\n";
        let s = structure_of("Rules", text);
        assert_eq!(s.nonlinear.len(), s.equations.len());
        for (row, nonlinear) in s.incidence.iter().zip(&s.nonlinear) {
            assert!(nonlinear.windows(2).all(|w| w[0] < w[1]));
            assert!(nonlinear.iter().all(|u| row.contains(u)));
        }
        let none: [&str; 0] = [];
        // 状態とパラメータは係数に入っても線形
        assert_eq!(nonlinear_names(&s, "der(x) = -p * x * a"), none);
        assert_eq!(nonlinear_names(&s, "a = time"), none);
        assert_eq!(nonlinear_names(&s, "b * a = 1"), ["a", "b"]);
        assert_eq!(nonlinear_names(&s, "c = a / p"), none);
        assert_eq!(nonlinear_names(&s, "d = p / c"), ["c"]);
        assert_eq!(nonlinear_names(&s, "e = a ^ 1"), none);
        assert_eq!(nonlinear_names(&s, "f = sin(b)"), ["b"]);
        // イベントで凍結される条件は線形のまま。noEvent の中の条件は、条件と分岐すべてを非線形に
        assert_eq!(nonlinear_names(&s, "g = if a > 0 then 2 * b else c"), none);
        assert_eq!(
            nonlinear_names(&s, "h = noEvent(if a > 0 then b else c)"),
            ["a", "b", "c"]
        );
        assert_eq!(nonlinear_names(&s, "k = homotopy(a * b, a)"), ["a", "b"]);
        assert_eq!(nonlinear_names(&s, "m = x * c"), none);
        assert_eq!(nonlinear_names(&s, "n = c ^ 2 + d"), ["c"]);
        assert_eq!(nonlinear_names(&s, "q = smooth(1, b * p)"), none);
        assert_eq!(nonlinear_names(&s, "r = abs(c)"), ["c"]);
    }

    #[test]
    fn nonlinear_array_rows_follow_the_family() {
        // 行列の積（係数がパラメータ）は線形、要素ごとの積は非線形。族の単位で判定する
        let text = "model Mat\n  parameter Real A[2, 2] = [1, 2; 3, 4];\n  Real x[2], y[2], z[2];\nequation\n  A * x = {time, 1};\n  y = x .* x;\n  for i in 1:2 loop\n    z[i] = y[i] * 2;\n  end for;\nend Mat;\n";
        let s = structure_of("Mat", text);
        assert_eq!(s.equations.len(), 6);
        let nonlinear: Vec<Vec<String>> = s
            .nonlinear
            .iter()
            .map(|r| {
                r.iter()
                    .map(|&u| s.unknowns[u as usize].name.clone())
                    .collect()
            })
            .collect();
        let rows: Vec<(&str, &Vec<String>)> = s
            .equations
            .iter()
            .map(|e| e.label.as_str())
            .zip(&nonlinear)
            .collect();
        for (label, names) in rows {
            if label.starts_with("y = x .* x") {
                assert!(
                    !names.is_empty() && names.iter().all(|n| n.starts_with("x[")),
                    "{label}: {names:?}"
                );
            } else {
                assert!(names.is_empty(), "{label}: {names:?}");
            }
        }
    }

    #[test]
    fn rumoca_reports_algebraic_loops_only_as_es002_warnings() {
        // Rumoca v0.10.2 の analyze は algebraic_loops を埋めず、ループを ES002 の警告として返す
        let text = "model L\n  Real x, y;\nequation\n  x * y = 1;\n  x + y = 3;\nend L;\n";
        let mut session = Session::new(SessionConfig::default());
        session.add_document("L.mo", text).unwrap();
        let compilation = session.compile_model_strict("L").expect("L compiles");
        let dae = &compilation.result().dae;
        let reference = dae.inspect(rumoca_compile::phase_structural::analyze);
        assert!(reference.algebraic_loops.is_empty());
        let loops: Vec<&str> = reference
            .diagnostics
            .iter()
            .filter(|d| d.code.as_deref() == Some("ES002"))
            .map(|d| d.message.as_str())
            .collect();
        assert_eq!(
            loops,
            ["algebraic loop detected: 2 equations must be solved simultaneously"]
        );
        let sorted = dae
            .inspect(|view| {
                rumoca_compile::phase_structural::sort(view).map(|sorted| {
                    sorted
                        .blocks
                        .iter()
                        .filter_map(|b| match b {
                            rumoca_compile::phase_structural::BltBlock::AlgebraicLoop {
                                equations,
                                ..
                            } => Some(equations.len()),
                            _ => None,
                        })
                        .collect::<Vec<_>>()
                })
            })
            .expect("sort");
        assert_eq!(sorted, [2]);
    }

    /// MSL の例題で、取り出しの時間と非線形の行の割合を見る（MSL が無ければ飛ばす）。
    /// `cargo test -p globe-mode-frontend --lib msl_nonlinear -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn msl_nonlinear_extraction_is_fast() {
        let msl = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../../data/msl");
        if !msl.is_dir() {
            eprintln!("MSL が無いので飛ばす: {}", msl.display());
            return;
        }
        let mut session = Session::new(SessionConfig::default());
        let load = session.load_source_root_tolerant(
            "library:msl",
            rumoca_compile::compile::SourceRootKind::DurableExternal,
            &msl,
            None,
        );
        assert!(load.parsed_file_count > 0, "MSL loads");
        for model in [
            "Modelica.Electrical.Analog.Examples.ChuaCircuit",
            "Modelica.Mechanics.MultiBody.Examples.Elementary.DoublePendulum",
            "Modelica.Fluid.Examples.HeatingSystem",
        ] {
            let compilation = session.compile_model_strict(model).expect("compiles");
            let dae = &compilation.result().dae;
            let start = std::time::Instant::now();
            let s = extract(dae).expect("structure");
            let elapsed = start.elapsed();
            let rows = s.nonlinear.iter().filter(|r| !r.is_empty()).count();
            let refs: usize = s.incidence.iter().map(Vec::len).sum();
            let nonlinear: usize = s.nonlinear.iter().map(Vec::len).sum();
            eprintln!(
                "{model}: 方程式 {}、非線形の行 {rows}、参照 {refs} のうち非線形 {nonlinear}、取り出し {elapsed:?}",
                s.equations.len()
            );
            assert_eq!(s.nonlinear.len(), s.equations.len());
        }
    }
}

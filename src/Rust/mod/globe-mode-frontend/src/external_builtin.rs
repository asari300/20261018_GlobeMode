//! `external "builtin"` で組み込みの数学関数を呼ぶだけの関数を、組み込み演算に置き換える（S068、ADR-0029）。
//!
//! MSL 4.0.0 の `Modelica.Math.sin` は `external "builtin" y = sin(u);` と書く（4.1.0 は `algorithm y := .sin(u);`）。
//! Rumoca の実行系はこれを「`calls 'sin', which this runtime cannot execute`」で断る（EL005 / EX002）。
//! Rumoca の `Dae` は読み取り専用で、公開の構築 API は借用の印（ブランド）付きの閉包だけなので、書き換えは
//! 直列化の形（`Dae` の serde、読み戻すときに Rumoca が検査し直す）の上で行う。
//!
//! 置き換えの条件（関数定義の external 節から判定し、名前の固定表は組み込み関数の名前と引数の数だけ）:
//! - 言語が `builtin`、記号が [`ARITY`] の名前、引数が関数の入力を順に渡すだけ、出力が 1 つで本体が空
//! - 呼び出しの引数の数が合う
//!
//! 条件に合わない external（`"C"` や別の記号）は触らない（今まで通り GM-U0004）。置き換えた Dae を Rumoca が
//! 受け付けなければ元の Dae を返す。

use std::sync::Arc;

use rumoca_ir_dae::{Dae, ExternalLanguage};
use serde_json::{Value, json};

/// 組み込みの数学関数（`PureBuiltin` の名前）と引数の数。
const ARITY: &[(&str, usize)] = &[
    ("sin", 1),
    ("cos", 1),
    ("tan", 1),
    ("asin", 1),
    ("acos", 1),
    ("atan", 1),
    ("atan2", 2),
    ("sinh", 1),
    ("cosh", 1),
    ("tanh", 1),
    ("exp", 1),
    ("log", 1),
    ("log10", 1),
    ("sqrt", 1),
    ("abs", 1),
];

/// `external "builtin"` の呼び出しを組み込み演算にした Dae。置き換えるものが無い・置き換えられなければ `dae` のまま。
pub(crate) fn lower_builtin_externals(dae: &Arc<Dae>) -> Arc<Dae> {
    if !has_builtin_external(dae) {
        return Arc::clone(dae);
    }
    rewrite(dae).map_or_else(|| Arc::clone(dae), Arc::new)
}

fn has_builtin_external(dae: &Dae) -> bool {
    dae.inspect(|view| {
        (0..)
            .map_while(|i| view.function_id(i))
            .filter_map(|id| view.function(id)?.external())
            .any(|e| e.language() == ExternalLanguage::Builtin)
    })
}

fn rewrite(dae: &Dae) -> Option<Dae> {
    let mut wire = serde_json::to_value(dae).ok()?;
    // 表は読み戻しの形（deny_unknown_fields）に無いので外し、読み戻した Dae に付け直す
    wire.as_object_mut()?.remove("external_tables");
    let storage = wire.get_mut("storage")?;
    let table = builtin_functions(storage);
    if table.iter().all(Option::is_none) {
        return None;
    }
    let mut changed = false;
    for node in storage
        .get_mut("expressions")?
        .get_mut("nodes")?
        .as_array_mut()?
    {
        let Some(call) = node.get("call") else {
            continue;
        };
        let function = usize::try_from(call.get("function")?.as_u64()?).ok()?;
        let (Some(Some((name, arity))), Some(count)) = (
            table.get(function),
            call.get("operand_count").and_then(Value::as_u64),
        ) else {
            continue;
        };
        if count as usize == *arity && call.get("output").and_then(Value::as_u64) == Some(0) {
            *node = json!({ "builtin": { "builtin": name, "operand_count": arity } });
            changed = true;
        }
    }
    changed
        .then(|| serde_json::from_value::<Dae>(wire).ok())
        .flatten()
        .map(|rewritten| rewritten.with_external_tables(dae.external_tables().to_vec()))
}

/// 関数の番号ごとの、置き換え先（組み込み演算の名前と引数の数）。
fn builtin_functions(storage: &Value) -> Vec<Option<(&'static str, usize)>> {
    let Some(functions) = storage["functions"].as_array() else {
        return Vec::new();
    };
    functions
        .iter()
        .map(|function| {
            let external = function.get("external").filter(|e| !e.is_null())?;
            if external["language"] != "builtin" || !function["statements"].as_array()?.is_empty() {
                return None;
            }
            let symbol = external["symbol"].as_str()?;
            let &(name, arity) = ARITY.iter().find(|(n, _)| *n == symbol)?;
            let arguments = external["arguments"].as_array()?;
            // shortcut: 引数が「入力そのもの」かは式の番号で確かめられない（直列化の式の番号は Rumoca の内部の番号と
            // ずれる）。入力の数・出力の数・引数がすべて入力であること・番号が増える順であること（atan2(u2, u1) を弾く）
            // で代える。Rumoca が公開の読み取りで引数を引けるようになったら式まで確かめる。
            let ids: Vec<u64> = arguments
                .iter()
                .map(|a| a["input"].as_u64())
                .collect::<Option<_>>()?;
            let simple = function["parameters"].as_array()?.len() == arity
                && function["outputs"].as_array()?.len() == 1
                && external["result"].as_u64() == Some(0)
                && ids.len() == arity
                && ids.windows(2).all(|w| w[0] < w[1]);
            simple.then_some((name, arity))
        })
        .collect()
}

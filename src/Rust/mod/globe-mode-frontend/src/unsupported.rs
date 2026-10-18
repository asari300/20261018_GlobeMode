//! 「コンパイラが未対応」（GM-U）の振り分け（ADR-0007）。
//!
//! 規則は上から順に当てる。
//!
//! 1. コードで決まるもの（モデルに依らない）: Rumoca のエラーコードが「未対応の言語機能・実装の制限」
//!    （例 ED019 → `GM-U0001`）か「コンパイラの内部エラー」（例 ED008 → `GM-U0002`）のとき。
//! 2. 既知の制約の一覧: `src/Modelica/msl-known-unsupported.txt`（モデルの完全修飾名と Rumoca のコードの組）
//!    に当たったとき → `GM-U0003`。ED001（数が合わない）のように、モデルの誤りでも出るコードはこちらで決める。
//!
//! ED001（数が合わない）・EF002（接続の型が合わない）・ER003（基底クラスが無い）などは、
//! モデルの誤りでも出るので、**コードだけでは決めない**（一覧に載っているときだけ GM-U にする）。
//!
//! コードの表は Rumoca v0.10.2（rev 276e82bd）の各 phase の `errors.rs` と
//! `spec/SPEC_0008_PHASE_ERRORS.md` を読んで作った（2026-10-18 に確認）。Rumoca の rev を上げたら見直す。

use std::sync::OnceLock;

use globe_mode_core::codes;

/// GM-U の判定結果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unsupported {
    /// `GM-U0001`（未対応）/ `GM-U0002`（内部エラー）/ `GM-U0003`（既知の制約の一覧）
    pub code: &'static str,
    /// 利用者に見せる理由
    pub reason: String,
}

/// 規則 1: 「未対応の言語機能・実装の制限」と読める Rumoca のコード（→ `GM-U0001`）。
///
/// 出典は Rumoca v0.10.2 の次のファイル（`~/.cargo/git/checkouts/rumoca-*/276e82b/crates/…`）。
const UNSUPPORTED_CODES: &[(&str, &str)] = &[
    // rumoca-phase-dae/src/errors.rs
    // UnresolvedClockSchedule: 静的な周期に解決できない Clock（SPEC_0022 §4.16.1）
    ("ED009", "静的な周期に解決できないクロック"),
    // UnsupportedAlgorithm: 「DAE を作る前に、対応している宣言的な形に落とす必要がある」
    ("ED013", "DAE に落とせないアルゴリズム節"),
    // UnsupportedRuntimeOperator: 「検査済みの DAE で表せない実行時の意味は拒む」
    ("ED018", "対応していない実行時の演算子"),
    // UnsupportedFlatSemantics: 「検査済みの DAE の表現ができるまで、シミュレーションの前に拒む」
    ("ED019", "DAE で表せない構文（flat の意味の持ち主）"),
    // Construction: 検査付きの DAE の構築が拒んだ式（下げ方の文法の外、形が合わないなど）。
    // ここまで来たモデルは型検査を通っているので、モデルの誤りではなく Rumoca の制限として扱う
    ("ED020", "DAE の構築が受け付けない式の形"),
    // rumoca-phase-flatten/src/errors.rs
    // UnsupportedEquationForm
    ("EF004", "対応していない方程式の形"),
    // UnsupportedExpandableConnectorAugmentation: MLS §9.1.3 の expandable connector の拡張
    (
        "EF020",
        "expandable connector のメンバーの拡張（MLS §9.1.3）",
    ),
    // UnhonoredFunctionRedeclare: 「この redeclare の形は正しいが、まだ扱えない」
    ("EF031", "replaceable な関数の redeclare"),
    // NonUniformParameterFixed: 「パラメータの配列の要素ごとの fixed は未対応」
    ("EF033", "パラメータの配列の要素ごとに違う fixed"),
    // rumoca-phase-instantiate/src/errors.rs（ファイル冒頭の表で「implementation limit」）
    // InstantiationDepthLimit
    ("EI030", "instantiate の深さの上限（実装の制限）"),
    // UnsupportedFixedAttribute: 表せない fixed 属性（MLS §4.8）
    ("EI035", "表せない fixed 属性"),
    // rumoca-phase-resolve/src/semantic_checks/builtin_calls.rs
    // ER073_STATE_MACHINE_UNSUPPORTED: 「状態機械の下げ方はまだ実装していない（MLS §17）」
    ("ER073", "状態機械（MLS §17）"),
];

/// 規則 1: 「コンパイラの内部の不変条件が崩れた」と読める Rumoca のコード（→ `GM-U0002`）。
/// 前の phase が正しく働いていれば起きないもの。モデルの誤りではなく Rumoca の不具合として扱う。
const COMPILER_BUG_CODES: &[(&str, &str)] = &[
    // rumoca-phase-dae/src/errors.rs
    ("ED003", "DAE への変換の内部エラー"),
    // MissingProvenance: 「すべての宣言・式・添字は、ソースの位置を持っていなければならない」
    ("ED007", "flat IR がソースの位置を失った"),
    // UnresolvedReference: 「flat IR は、宣言済みで解決済みの座標を持っていなければならない」
    ("ED008", "flat IR の参照が解決されていない"),
    // MissingSemanticIdentity: 「flat IR は、定義済みの識別子を phase の境界をまたいで保たなければならない」
    ("ED021", "flat IR が意味の識別子を失った"),
    // rumoca-phase-flatten/src/errors.rs
    // Internal
    ("EF005", "flatten の内部エラー"),
    // MissingSourceScope: 「instantiate は flatten の名前の探索に使う字句のスコープを保たなければならない」
    ("EF008", "instantiate がソースのスコープを失った"),
    // FunctionWithoutBody: 「flatten は replaceable な関数を実行できる具体的な関数に解決しなければならない」
    ("EF009", "実行できない関数が flat IR に残った"),
    // UnresolvedVariableType: 「instantiate / typecheck は型の識別子を保たなければならない」
    ("EF014", "変数の型が flatten までに解決されていない"),
    // MissingResolvedClassMetadata: 「名前解決は DefId を付けて保たなければならない」
    ("EF015", "クラスの DefId が失われた"),
    // MissingSourceContext: 「前の phase はソースの対応表を保たなければならない」
    ("EF017", "ソースの対応表が失われた"),
    // InconsistentFunctionReference: 「名前解決は関数の識別子を 1 つに保たなければならない」
    ("EF019", "関数の参照の表示名と構造が食い違う"),
    // UnresolvedFlatReference: 「resolve と instantiate は参照の構造を保たなければならない」
    ("EF023", "flat IR の参照が解決されていない"),
    // MissingFlatVariableIdentity: 「instantiate はすべての宣言に解決済みのパスを付けなければならない」
    ("EF024", "flat IR の変数が識別子を失った"),
    // MissingFunctionSelectionIdentity: 「resolve と instantiate は関数の選択を保たなければならない」
    ("EF025", "関数の選択の識別子が失われた"),
    // InconsistentFunctionCallKind: 「前の phase は通常の関数の呼び出しを constructor と印付けてはならない」
    ("EF029", "関数の呼び出しの種類が食い違う"),
    // rumoca-phase-instantiate/src/errors.rs（ファイル冒頭の表で「compiler … invariant」）
    // MissingResolvedIdentity: 「compiler phase-order invariant」
    ("EI033", "解決済みの宣言の識別子が失われた"),
    // MissingSourceContext: 「compiler provenance invariant」
    ("EI098", "instantiate でソースの対応表が失われた"),
    // rumoca-compile/src/session/strict_compile_diagnostics.rs: 「internal error: missing source-map entry …」
    ("EI000", "Session の内部エラー（ソースの対応表）"),
    // rumoca-phase-resolve/src/lib.rs: ER098_MISSING_SOURCE_CONTEXT
    ("ER098", "名前解決でソースの対応表が失われた"),
    // rumoca-phase-structural/src/diagnostic_codes.rs: ES014_CONTRACT_VIOLATION
    // 「構造解析に要る DAE IR のメタデータが無いか、食い違う」
    ("ES014", "構造解析の前提（DAE IR のメタデータ）が崩れた"),
];

/// 既知の制約の一覧（ADR-0007）。`globe-mode-frontend` にコンパイル時に埋め込む。
const KNOWN_LIST: &str = include_str!("../../../../Modelica/msl-known-unsupported.txt");

/// 既知の制約の一覧の 1 行。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KnownEntry {
    /// モデルの完全修飾名
    pub model: String,
    /// Rumoca のエラーコード（`ED001` の形）
    pub code: String,
    /// `#` から後ろ（無ければ None）
    pub reason: Option<String>,
}

/// 一覧を読んだ結果。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct KnownList {
    pub entries: Vec<KnownEntry>,
    /// 形が誤っていて読み飛ばした行（1 始まりの行番号と本文）
    pub malformed: Vec<(usize, String)>,
}

impl KnownList {
    /// 一覧の本文を読む。1 行 = `<モデルの完全修飾名> <Rumoca のコード> # 理由`。
    /// `#` で始まる行と空行は飛ばす。形の誤った行は `malformed` に入れて飛ばす（panic しない）。
    pub fn parse(text: &str) -> Self {
        let mut list = KnownList::default();
        for (i, line) in text.lines().enumerate() {
            let (body, reason) = match line.split_once('#') {
                Some((b, r)) => (b, Some(r.trim())),
                None => (line, None),
            };
            let fields: Vec<&str> = body.split_whitespace().collect();
            match fields.as_slice() {
                [] => {}
                [model, code] if is_model_name(model) && is_rumoca_code(code) => {
                    list.entries.push(KnownEntry {
                        model: (*model).to_string(),
                        code: (*code).to_string(),
                        reason: reason.filter(|r| !r.is_empty()).map(str::to_string),
                    });
                }
                _ => list.malformed.push((i + 1, line.to_string())),
            }
        }
        list
    }

    fn find(&self, model: &str, code: &str) -> Option<&KnownEntry> {
        self.entries
            .iter()
            .find(|e| e.model == model && e.code == code)
    }
}

/// 埋め込んだ一覧（初めて使うときに一度だけ読む）。
pub fn known_list() -> &'static KnownList {
    static LIST: OnceLock<KnownList> = OnceLock::new();
    LIST.get_or_init(|| KnownList::parse(KNOWN_LIST))
}

/// `model`（完全修飾名）の検査が Rumoca のコード `rumoca_code` で失敗したとき、GM-U に当たるかを返す。
pub fn classify(model: &str, rumoca_code: &str) -> Option<Unsupported> {
    classify_in(known_list(), model, rumoca_code)
}

/// [`classify`] と同じ判定を、一覧の本文 `list` を渡して行う（試験用）。
pub fn classify_with(list: &str, model: &str, rumoca_code: &str) -> Option<Unsupported> {
    classify_in(&KnownList::parse(list), model, rumoca_code)
}

/// 規則 1（コードだけで決まるもの）だけを当てる。一覧は見ない。
pub fn classify_code(rumoca_code: &str) -> Option<Unsupported> {
    let code = bare_code(rumoca_code);
    if let Some((_, what)) = UNSUPPORTED_CODES.iter().find(|(c, _)| *c == code) {
        return Some(Unsupported {
            code: codes::UNSUPPORTED,
            reason: format!("Rumoca v0.10.2 が未対応の言語機能（{code}: {what}）"),
        });
    }
    if let Some((_, what)) = COMPILER_BUG_CODES.iter().find(|(c, _)| *c == code) {
        return Some(Unsupported {
            code: codes::COMPILER_BUG,
            reason: format!("Rumoca v0.10.2 の内部エラー（{code}: {what}）。モデルの誤りではない"),
        });
    }
    None
}

fn classify_in(list: &KnownList, model: &str, rumoca_code: &str) -> Option<Unsupported> {
    if let Some(u) = classify_code(rumoca_code) {
        return Some(u);
    }
    let code = bare_code(rumoca_code);
    list.find(model, code).map(|e| Unsupported {
        code: codes::KNOWN_LIMITATION,
        reason: match &e.reason {
            Some(r) => format!("既知の制約（{code}）: {r}"),
            None => format!(
                "既知の制約の一覧（src/Modelica/msl-known-unsupported.txt）に載っている（{code}）"
            ),
        },
    })
}

/// `rumoca::todae::ED001` のような表記から、末尾の `ED001` を取り出す（SPEC_0008: 末尾で比べる）。
fn bare_code(code: &str) -> &str {
    code.rsplit("::").next().unwrap_or(code).trim()
}

/// `ED001` の形（英大文字 2〜3 字 + 数字 3 字）。
fn is_rumoca_code(s: &str) -> bool {
    let letters = s.chars().take_while(char::is_ascii_uppercase).count();
    let digits = &s[letters..];
    (2..=3).contains(&letters) && digits.len() == 3 && digits.chars().all(|c| c.is_ascii_digit())
}

/// `A.B.C` の形（`.` で区切った識別子。引用識別子は一覧では使わない）。
fn is_model_name(s: &str) -> bool {
    !s.is_empty()
        && s.split('.').all(|p| {
            p.chars()
                .next()
                .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
                && p.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    const LIST: &str = "\
# 見出しのコメント

A.B.Model ED001 # 周期の配列の釣り合いを数え違える
A.B.NoReason EF002
  A.C.Indented   ER003   #   前後の空白は無視する
";

    #[test]
    fn unsupported_feature_by_code() {
        let u = classify_with("", "Any.Model", "ED019").unwrap();
        assert_eq!(u.code, codes::UNSUPPORTED);
        assert!(u.reason.contains("ED019"), "{}", u.reason);
        // Rumoca の miette の表記（末尾がコード）でも同じ
        assert_eq!(
            classify_with("", "Any.Model", "rumoca::todae::ED019")
                .unwrap()
                .code,
            codes::UNSUPPORTED
        );
        assert_eq!(classify_code("ER073").unwrap().code, codes::UNSUPPORTED);
    }

    #[test]
    fn compiler_bug_by_code() {
        let u = classify_with("", "Any.Model", "ED008").unwrap();
        assert_eq!(u.code, codes::COMPILER_BUG);
        assert!(u.reason.contains("内部エラー"), "{}", u.reason);
        assert_eq!(classify_code("ES014").unwrap().code, codes::COMPILER_BUG);
    }

    #[test]
    fn ambiguous_codes_are_not_classified_by_code_alone() {
        for code in ["ED001", "EF002", "ER003", "EI008", ""] {
            assert_eq!(classify_with("", "Any.Model", code), None, "{code}");
            assert_eq!(classify_code(code), None, "{code}");
        }
    }

    #[test]
    fn list_entry_gives_known_limitation() {
        let u = classify_with(LIST, "A.B.Model", "ED001").unwrap();
        assert_eq!(u.code, codes::KNOWN_LIMITATION);
        assert!(u.reason.contains("周期の配列"), "{}", u.reason);

        let u = classify_with(LIST, "A.B.NoReason", "EF002").unwrap();
        assert_eq!(u.code, codes::KNOWN_LIMITATION);
        assert!(
            u.reason.contains("msl-known-unsupported.txt"),
            "{}",
            u.reason
        );

        assert_eq!(
            classify_with(LIST, "A.C.Indented", "ER003").unwrap().code,
            codes::KNOWN_LIMITATION
        );
        // モデルとコードの両方が合うときだけ
        assert_eq!(classify_with(LIST, "A.B.Model", "EF002"), None);
        assert_eq!(classify_with(LIST, "A.B.Other", "ED001"), None);
    }

    #[test]
    fn code_rule_wins_over_list() {
        let list = "A.B.Model ED019 # 一覧にもある\n";
        assert_eq!(
            classify_with(list, "A.B.Model", "ED019").unwrap().code,
            codes::UNSUPPORTED
        );
    }

    #[test]
    fn comments_and_blank_lines_are_ignored() {
        let list = KnownList::parse(LIST);
        assert_eq!(list.entries.len(), 3);
        assert!(list.malformed.is_empty(), "{:?}", list.malformed);
        assert_eq!(
            list.entries[2].reason.as_deref(),
            Some("前後の空白は無視する")
        );
        assert_eq!(list.entries[1].reason, None);
    }

    #[test]
    fn malformed_lines_are_skipped_without_panic() {
        let text = "\
OnlyModel
A.B ED001 extra
A.B ed001
A..B ED001
A.B E1
A.B ED0011
1A.B ED001
ü.B ED001
Good.Model ED001 #
";
        let list = KnownList::parse(text);
        assert_eq!(list.entries.len(), 1, "{:?}", list.entries);
        assert_eq!(list.entries[0].model, "Good.Model");
        assert_eq!(list.entries[0].reason, None);
        assert_eq!(list.malformed.len(), 8);
        assert_eq!(list.malformed[0].0, 1);
    }

    #[test]
    fn embedded_list_is_well_formed() {
        // 中身は問わない（MSL の結果で変わる）。形の誤った行が無いことだけを確かめる。
        assert!(
            known_list().malformed.is_empty(),
            "src/Modelica/msl-known-unsupported.txt に形の誤った行がある: {:?}",
            known_list().malformed
        );
    }
}

# ADR（設計判断の記録）

| 番号 | 題名 | 状態 |
| --- | --- | --- |
| [0001](0001-license-policy.md) | ライセンス方針 | Accepted |
| [0002](0002-rumoca-adapter.md) | Rumoca をアダプタ経由の git 依存で使う | Accepted |
| [0003](0003-toolchain.md) | ツールチェーン（mise・uv・cargo） | Accepted |
| [0004](0004-ui-tauri-svelte.md) | UI は Tauri 2 + Svelte 5 | Accepted |
| [0005](0005-lazy-query-db.md) | salsa による遅延評価とシミュレーション前のアサーション | Accepted |
| [0006](0006-incremental-ci.md) | CI は差分だけを検査し、大きな変更のときだけ全体を検査する | Accepted |
| [0007](0007-unsupported-classification.md) | 「コンパイラが未対応」（GM-U）を、コードの規則と既知の制約の一覧で振り分ける | Accepted |
| [0008](0008-local-llm-reviewer.md) | ローカル LLM は副レビュアーとし、スキルから OpenAI 互換 API で呼ぶ | Accepted |
| [0009](0009-structural-index-and-init.md) | S7 は Pantelides 法で構造インデックスを見積もり、S8 は初期条件の数と属性で判定する | Accepted |
| [0010](0010-omc-comparison.md) | omc との照合は別プロセスで行い、checkModel の数と translateModel の文言で突き合わせる | Accepted |
| [0011](0011-init-system-matching.md) | S8 は初期化系（DAE とその微分 + 初期条件）をマッチングして、冗長な初期条件と初期条件の無い未知数を名指しする | Accepted |
| [0012](0012-loops-and-preflight.md) | S9 は BLT と貪欲な tearing で非線形の代数ループの反復変数を見る。S10 は静的な部分（パラメータと start 値の評価、experiment annotation）から始める | Accepted |
| [0013](0013-exec-backend.md) | P3 の実行系は、Rumoca の実行系（rumoca-sim）を globe-mode-exec に閉じ込めて使う。結果は中立の型で返し、MAP-LIB の参照結果と矩形の管で比べる | Accepted |
| [0014](0014-session-route-wasm.md) | wasm32 と自前の積分器は、Rumoca の FMI ME 風の host に数値積分のプラグインを差す session の道で回す。ネイティブの bdf・dopri45・auto は型付きの道のまま（wasm-bindgen、auto の切り替え、パラメータの上書き、MAT v4） | Accepted |

書式: `NNNN-<題名>.md`。章は「状態 / 背景 / 決定 / 結果」の4つ。決定を覆すときは古い ADR を消さず、状態を Superseded にして新しい番号で書く。

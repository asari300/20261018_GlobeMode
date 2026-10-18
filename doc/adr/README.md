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
| [0008](0008-structural-index-and-init.md) | S7 は Pantelides 法で構造インデックスを見積もり、S8 は初期条件の数と属性で判定する | Accepted |

書式: `NNNN-<題名>.md`。章は「状態 / 背景 / 決定 / 結果」の4つ。決定を覆すときは古い ADR を消さず、状態を Superseded にして新しい番号で書く。

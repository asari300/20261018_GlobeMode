# ADR（設計判断の記録）

| 番号 | 題名 | 状態 |
| --- | --- | --- |
| [0001](0001-license-policy.md) | ライセンス方針 | Accepted |
| [0002](0002-rumoca-adapter.md) | Rumoca をアダプタ経由の git 依存で使う | Accepted |
| [0003](0003-toolchain.md) | ツールチェーン（mise・uv・cargo） | Accepted |
| [0004](0004-ui-tauri-svelte.md) | UI は Tauri 2 + Svelte 5 | Accepted |
| [0005](0005-lazy-query-db.md) | salsa による遅延評価とシミュレーション前のアサーション | Accepted |

書式: `NNNN-<題名>.md`。章は「状態 / 背景 / 決定 / 結果」の4つ。決定を覆すときは古い ADR を消さず、状態を Superseded にして新しい番号で書く。

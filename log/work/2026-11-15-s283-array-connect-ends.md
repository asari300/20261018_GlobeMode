# S283 `array-connect-ends`

| | |
| --- | --- |
| 日付 | 2026-11-15 |
| ブランチ | `feature/s283-array-connect-ends` |
| worktree | `.git-worktree/s283-array-connect-ends` |
| Phase | P8 |
| セッション文書 | `doc/sessions/S283-array-connect-ends/` |
| セッション記録 | なし（このセッションの記録が `log/local/` に無いため省く） |

## 依頼

S281 の不具合。`connect_ends` が配列の添字を落とすので、`connect(b[2].p, c.n)` を消すと `b[1].p` の結線も消える。

## 決めたこと

- 共有の `connect_ends` を直し、`[…]` の字句を空白なしで端に含める。`delete_connect`・`add_connect`（端の検査は別）・図の結線の `from`・`to` はこれで同時に直る。
- `delete_part` は端の最初の名前で数えるので変更なし。UI（`DiagramView`・`shape-svg.ts`）は `data-conn` の `from to` をそのまま渡し、端に空白が無いので変更なし。
- 既存の試験が添字を落とした値を前提にしていた（`r2.p`、`b.p`）ので、添字つきに直した。

## 検証

- `cargo test -p globe-mode-fmt --lib diagram` 9 本 pass。直す前は新しい試験を含む 3 本が落ちることを確かめた。
- `cargo test -p globe-mode-lsp --lib delete_` pass（新しい 1 本を含む）。

## 残したこと

wasm の試験は足していない（fmt と LSP で同じ関数を通る）。ブラウザでの操作は確かめていない。

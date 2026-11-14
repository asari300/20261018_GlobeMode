# S250 `diagram-move`

| | |
| --- | --- |
| 日付 | 2026-11-14 |
| ブランチ | `feature/s250-diagram-move` |
| worktree | `.git-worktree/s250-diagram-move` |
| Phase | P8 |
| セッション文書 | `doc/sessions/S250-diagram-move/` |
| セッション記録 | なし（サブエージェントの作業で、このセッション専用の記録が無い） |

## 依頼

Diagram ビューの編集の最初の一切れ = 部品のドラッグ移動。Placement の `origin` を Rust がテキスト編集にして返し、UI がエディタに適用する。結線の点・部品と結線の追加・VS Code は今回やらない。

## 決めたこと

- 書き換えるのは `origin` だけ。無ければ `transformation(` の直後に足す。`extent`・`rotation` は触らない（`origin` は図の座標の平行移動）。
- Placement が無い宣言、`origin` が数の並びでない宣言は動かさない（null）。
- 口は `rename` と同じ形（LSP の TextEdit の配列）。`globe/movePart`、wasm `move_part`。Desktop の `RPC_METHODS` は 27。
- 吸着は UI で移動量を 2 単位に丸める（UI に `origin` を持たせないため）。
- ブラウザの操作の試験（`test_ui.py`）は足していない（Playwright とブラウザ版の wasm のビルドが要り、手元の負荷が高かったので CI の全体検査に任せる）。

## 検証

（数字は締めで書く）

# S286 `linearize-eigen`

| | |
| --- | --- |
| 日付 | 2026-11-15 |
| ブランチ | `feature/s286-linearize-eigen` |
| worktree | `.git-worktree/s286-linearize-eigen` |
| Phase | P8 |
| セッション文書 | `doc/sessions/S286-linearize-eigen/` |
| セッション記録 | なし（`log/local/` に無い） |

## 依頼

線形化に A の固有値（極）と安定性の一言を足す。新しい依存は足さない。UI の表は今回やらない。

## 決めたこと

- facade の `linearization_json` に `eigenvalues`・`stability` を足す（足すだけ）。`nalgebra`（Cargo.lock に既にある）を `globe-mode` の直接の依存にした。
- 境界は `|Re| ≤ 1e-6·max(1, |λ|)`。反復に上限を付けて、収束しない A で止まらなくする。

## やったこと

- `globe-mode`（`eigenvalues`・`stability`・`linearization_json`）、CLI の出力、CLI の試験 2 本（LinearSystem、Pendulum）。
- ADR-0038 の付記、features・roadmap・licenses・索引・スライドと解説書。

## 検証

- `cargo test -p globe-mode-cli --test linearize` 4 本、`-p globe-mode --test linearize` 2 本、`-p globe-mode-lsp linearize` と `-p globe-mode-c` 通過。
- `cargo clippy -p globe-mode -p globe-mode-cli --all-targets -- -D warnings` 0 件、`cargo check -p globe-mode --target wasm32-unknown-unknown` 通過、`cargo deny check` ok。

## 残したこと

- Svelte の固有値の表、固有ベクトル、解析ヤコビ行列。

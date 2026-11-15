# S307 `native-closure`

| | |
| --- | --- |
| 日付 | 2026-11-15 |
| ブランチ | `feature/s307-native-closure`（`feature/s305-check-native` から切った） |
| worktree | `.git-worktree/s307-native-closure` |
| Phase | Track B |
| セッション文書 | `doc/sessions/S307-native-closure/` |
| セッション記録 | なし（専用のものが log/local/ に無い） |

## 依頼

S305 の持ち越し 2 つ。(1) native の検査がモデルの閉包だけを読む。(2) `GM-E6001` に位置を付ける（flat が範囲を持つ場合だけ）。

## 決めたこと

- 閉包は Rumoca の道の `SplitLibrary`（字句の索引）を native でも作って使う。`globe-mode-resolve` の遅延化より小さく、MSL の 566 本で確かめた走査をそのまま使える。
- (2) は flat の `Equation`・`Var` が範囲を持たないため、`flatten.rs` に触れず次回に回した（別の担当が触っている）。

## やったこと

- `globe-mode-flat/src/lib.rs`: `Project::read_dirs`（本文を読むだけ）。`from_dirs_with` はこれを使う。`flatten.rs` は触っていない。
- `globe-mode-frontend`: `split_library.rs` を `native` でも作る、`closure_sources`（pub）、`NativeFrontend::check` が閉包だけを `Project::from_sources` に渡す、`tests/native_closure.rs`。
- 文書: ADR-0039・ADR-0025 の付記、roadmap、セッションの索引・スライド・解説書。

## 検証

- 前後（debug の `globe check -L data/msl --frontend native`）: `PID_Controller` 15.8 s・227 MB → 2.9 s・67 MB、`First` 14.4 s → 2.3 s、`ChuaCircuit` 後 2.2 s・67 MB。
- release（ライブラリの段、`#[ignore]` の試験）: 全部読み 1.15 s、閉包 0.24〜0.56 s、最大 RSS 82 MB（閉包の道のみ）/ 208 MB（全部読みを足して）。
- MSL の 10 モデルで閉包と全部読みの `flatten` が完全一致（成功 5・断る 5）。
- `check_native` 3 本: pass。
- `cargo xtask diff-frontend`（release、83 s・VmHWM 1.1 GiB。前回の JSON が手元に無いので差は出ない代わりに、数字を roadmap の値と照合）: MSL は parse 566/566・resolve 467/566・flat 192・DAE 192・balance 144/146・counts 144/146・states 141/146・regular 132/146、validation は 75 本で parse 75/75・flat 72・balance 71/71。roadmap の数字と全部一致（回帰 0）。
- `cargo clippy -D warnings`（frontend・flat・cli、native だけのビルドも）と `cargo fmt --check`: 警告 0。
- `check_docs.py`: スライド OK、リンク 4920 件中の切れ 0 件。`render_slides.py`: 問題 0 件。
- `cargo test -p globe-mode-flat --test flat` の `declined_forms` が失敗する（S305 のブランチでも同じ。`model A replaceable Real x;` が断られない。今回の変更とは無関係。別の担当の領域）。

## 残したこと

`GM-E6001` の位置（flat に範囲を持たせる）、MSL 全例題での閉包と全部読みの突き合わせ、release の `globe` での前後、S7 以降、`redeclare`。

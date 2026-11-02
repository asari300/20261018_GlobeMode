# S087 `gleam-external`

| | |
| --- | --- |
| 日付 | 2026-11-02 |
| ブランチ | `feature/s087-gleam-external` |
| worktree | `.git-worktree/s087-gleam-external` |
| Phase | P7 |
| セッション文書 | `doc/sessions/S087-gleam-external/` |
| セッション記録 | （親が写す） |

## 依頼

S082 の持ち越しの Gleam の `@external` の例（features 84 行「Gleam は `@external`」）。`src/Gleam/globe_mode`（Erlang target）が NIF の関数を `@external(erlang, "globe_mode_nif", …)` で宣言し、gleeunit で Decay を check → simulate（JSON は解析せず文字列で探す）。`mise run gleam:test` と ci.yml の `lang`。Rust のコードは変えない。main へのマージ・push は親。

## 決めたこと

- `.so` の読み込みは Gleam のプロジェクトの `src/globe_mode_nif.erl`（`-on_load` で `erlang:load_nif(os:getenv("GLOBE_MODE_NIF", …), 0)`）。`rustler::init!("globe_mode_nif")` なので、このモジュール名でないと読めない。Elixir 側の `:globe_mode_nif` とはプロジェクトが別なので重ならない。
- Gleam 側は `@external` の宣言だけで、包む関数を書かない。ワークスペースは中身の無い `pub type Workspace`、結果は `Result(String, String)`。
- `set_file` は `Result(#(), String)`。Rustler は `Ok(())` を `{ok, {}}` で返す（erl で確かめた）。`Nil` だと atom `nil` になり型が嘘になる。
- 試験は `"passed":true`・`"ok":true`・CSV の最後の行の時刻 1（JSON の中の `\n1,` か `\n1.0,`）を `string.contains` で探す。索引に無いモデルの `graph` は `Error`。
- `gleam:test` は `MISE_DISABLE_TOOLS` に `erlang` か `elixir` があれば何もせず 0 で終わる（RPi、ADR-0019）。
- CI の gleam は `erlef/setup-beam` の `gleam-version: "1.19.2"`（手元の mise の版と同じ）。
- Hex の依存は gleam_stdlib 1.0.5・gleeunit 1.11.0（`manifest.toml` をコミット）。どちらも `wisp_fmt` と同じ版で licenses の既存の行に含まれるが、1 行足した。

## やったこと

- `src/Gleam/globe_mode/`（`gleam.toml`、`manifest.toml`、`src/globe_mode.gleam`、`src/globe_mode_nif.erl`、`test/globe_mode_test.gleam`）。`build/` は `.gitignore` 済み。
- `mise.toml` に `gleam:test`。`ci.yml` の `lang` に `gleam-version` と `mise run gleam:test`。`test_ci_plan.py` は変えていない（`src/Gleam/*` はすでに `lang` の対象）。
- 文書: features 84 行、architecture の言語 API の表、roadmap（S087 の節、S082 の持ち越しに取り消し線）、licenses、`src/Gleam/README.md`、CLAUDE.md、セッションの索引、スライドと解説書。

## 検証

- `mise run gleam:test`（手元、WSL、gleam 1.19.2、OTP 27）: 1 本、失敗 0（`set_file` の型を `#()` に直す前の版。直した後の手元の回し直しは、共有の target を別のセッションの release ビルドが握っていたので止めた。CI で通っている）。`MISE_DISABLE_TOOLS=elixir` では飛ばして 0。`gleam format --check`: ok。
- `check_docs.py`: デッキ全部 OK / 相対リンク 1764 件中の切れ 0 件
- CI（`gh workflow run ci.yml --ref feature/s087-gleam-external -f full=true`）: run 41400365586 で全ジョブ success（plan・rust-linux・rust-windows・rust-sundials・deny・python・msl・wasm-node・ui・docs・lang）。`lang` は setup-beam の gleam 1.19.2 で `mise run gleam:test` が「1 passed, no failures」、`elixir:test` も 1 本・失敗 0。`render_slides.py` は手元で playwright が `.venv` に入る前だったので CI のジョブ `docs` に任せた（success）

## 残したこと

- Gleam の JS target から `globe-mode-wasm` を呼ぶ。
- rustler_precompiled での配布（Hex）。
- Windows・macOS では回していない（`lang` は ubuntu だけ）。

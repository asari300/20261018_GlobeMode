# S173 `ui-unsupported-force-errors`

| | |
| --- | --- |
| 日付 | 2026-11-05 |
| ブランチ | `feature/s173-ui-unsupported-force-errors` |
| worktree | `.git-worktree/s173-ui-unsupported-force-errors` |
| Phase | P4 |
| セッション文書 | `doc/sessions/S173-ui-unsupported-force-errors/` |
| セッション記録 | [2026-11-02-a2580a13-3889-4662-9db4-a45dc1ca6772.md](../sessions/2026-11-02-a2580a13-3889-4662-9db4-a45dc1ca6772.md) |

## 依頼

S164 の監査の穴 3（未対応の見せ方と force、GM-I7001）、穴 6（FMU はソースだけ）、N9（エラー文面）、S169 の持ち越し（最後の時刻の値の内部名）を Svelte 版 UI で埋める。ポニーテール（最小の完全な変更）で。S170 が main に入ってから。あとから親の依頼で、`release.yml` の dry run（run 42029009320）で落ちた Windows の pytest の 1 行を別コミットで同梱。

## 決めたこと

- 強行が意味を持つ診断は、**実例で確かめた `GM-E8001` だけ**。GM-U（Rumoca の 15 種のコードの例題 1 本ずつ）、S1〜S5、`GM-E6001`、`GM-E10003`、`GM-E10004` は `--force` でも実行できない。親の仮説（S7〜S10 の preflight の Error は強行で進む）は S10 では成り立たなかった（実行の設定が `GM-E10004` を断る、start が 1/0 だと実行系が止まる）。
- 分類は Rust（`globe-mode-protocol` の `unsupported_summary`・`force_helps`）。応答に `unsupported`（要約。無いときは出ない）と `can_force`（true のときだけ）を足した。UI は `unsupported` があれば Error の行を「未対応」とする（`GM-U` の規則は Svelte に無い）。`Backend.simulate*` の force は `SimulateRequest` に既にあり、wasm・LSP・Desktop の 3 面とも口の変更は不要だった。
- `refused` の文面は強行が効かないとき「強行しても実行できない」（CLI も同じ。snapshot `simulate-unbalanced.json` を更新）。
- 数でない終了時刻は SimSettings が欄の近くに出す文面（`aria-invalid`）だけにし、「実行」では失敗の行を出さず欄へフォーカスを移す。S170 の試験の 1 行（理由の位置）をそれに合わせて直した。
- 色は新しい `--unsup`（明 `#5b3fa0`、暗 `#b9a2ff`。地に対して 7.9 / 8.7）。
- GM-I7001 に宣言が並ぶ件は Rust の出力の誤り（`globe-mode-frontend/src/structure.rs` の `label()` が来歴の span のソース片を使う）。直さず報告。

## やったこと

- Rust: `globe-mode-protocol/src/lib.rs`（フィールドと 2 関数）、`globe-mode/src/lib.rs`（応答に載せる。`simulate_observed` を薄い包みにした）、試験は wasm の `tests/decay.rs` に 3 本（強行できる・未対応の要約・検査の応答）と既存 1 本の期待、`globe-mode-lsp` に RPC の 1 本、CLI の snapshot 1 件。
- Svelte: `App.svelte`、`SimSettings.svelte`、`protocol.ts`、`theme.css`。`test_ui.py` に 2 本相当と既存への確認（FMU の文面、Friction、内部名）。
- 文書: 解説書・スライド・索引・roadmap・features。
- 同梱: `src/Python/tests/test_api.py:30` の `.read_text()` を `.read_text(encoding="utf-8")` に（原因: Windows の既定 cp1252 で日本語を読めず UnicodeDecodeError。S111 の行。release.yml の dry run 42029009320）。`src/Python/mod`・marimo・streamlit・fastapi_fmt に同じ形は無い。

## 検証

- wasm の出所: main `6af26bd`（S170 の merge）の上のこの worktree（Rust の変更は commit `5a1a8d5`、Svelte は未コミットで release ビルド。`CARGO_PROFILE_RELEASE_*` は `wasm:build` と同じ）。出力先は共有の `wasm-pkg` でなく scratch に作り、`pnpm build` / `wasm:node` に渡した。
- `test_ui.py`: ok（MSL.json あり。新しい確認を含む）。`pnpm check`: 0 errors / 0 warnings。
- `cargo test -p globe-mode-wasm --test decay`: 18 本中 17 ok・1 ignored（MSL の比較）。`cargo test -p globe-mode-lsp force_runs`: ok。
- MSL の実例: Friction（GM-U0003）は UI で要約・`tr.unsup` 1 行・原文が詳細に残る。GM-U 15 種のコードの例題は `--force` でも「DAE まで変換できないので実行できない」。
- 同梱の修正: main の `.venv` の pytest で `test_api.py` 6 本 ok（Linux なので cp1252 の再現はしない）。
- GM-SMOKE の行: 項目は足していない（`settings=ok` を含め全部 ok）。
- `mise run check`: 通った（fmt・clippy・test 672 本 ok / 0 failed・deny（advisories・bans・licenses・sources ok）・`uv sync --locked`。1 回目は CLI の snapshot（`refused` の文面）で落ち `UPDATE_SNAPSHOTS=1` で更新、共有 target の古いビルドで E0432 が出て変えた crate を touch）
- `wasm:node` 相当（`smoke.mjs --pkg <scratch> --msl data/msl`）: ok。`wasm:compare` は実行系を変えていないので回していない。
- `check_docs.py`: S173 のデッキ OK / リンク 3621 件中の切れ 0 件。`render_slides.py`（S173 のデッキだけ）: 問題 0 件（8 枚、ライト・ダーク・スマートフォン。表の余白を目で確認）
- セッション記録の `--check`: 親が写す
- ステージした差分の漏洩検査: 0 件（メールアドレス・ホームのパス・トークンの grep）

## 残したこと

- 強行が通るほかの Error（`GM-E7001` など。例を作れていない）。
- GM-I7001 / `GM-E6001` の方程式名に宣言が出る件（Rust、別 WP）。
- 未対応の理由の日本語化、診断の折りたたみ、診断の行のキーボード操作、UI から `--build`。
- Desktop の画面は手元で起動できず、sidecar 側は LSP の単体（RPC）で確かめた。

# S154 `ui-recent-scope-fmi3`

| | |
| --- | --- |
| 日付 | 2026-11-05 |
| ブランチ | `feature/s154-ui-recent-scope-fmi3` |
| worktree | `.git-worktree/s154-ui-recent-scope-fmi3` |
| Phase | P4/P5 |
| セッション文書 | `doc/sessions/S154-ui-recent-scope-fmi3/` |
| セッション記録 | [2026-11-02-a2580a13-3889-4662-9db4-a45dc1ca6772.md](../sessions/2026-11-02-a2580a13-3889-4662-9db4-a45dc1ca6772.md) |

## 依頼

S151 の持ち越し（Desktop の最近使ったファイルを、fs の許可範囲の外でも開けるようにする）と、S144 の持ち越し（FMU の書き出しで FMI の版を UI から選ぶ）。新しい依存は足さない。

## 決めたこと

- 読む道は (a) Tauri の command `read_recent { path }`。(b) fs の scope を起動時に一覧で広げる道は、UI の `localStorage` の中身を信用して許可を広げることになる。(c) `persisted-scope` は新しい依存。
- 検証は拡張子 `.mo` と `metadata().is_file()` の 2 つ。ディレクトリ・無いパス・ほかの拡張子は Err。サイズの上限とシンボリックリンクの扱いは入れていない（shortcut として ADR-0023 に書いた）。
- 「開く」の dialog は従来どおり fs の `readTextFile` のまま（`openFile` が `openPath` を呼ぶのをやめた）。dialog は拡張子を絞らない選択もあり、`.mo` の縛りを持ち込まないため。
- FMI の版の select は「FMU を書き出す」の右に置き、既定 2.0（`$state(2)`）。
- smoke の `recent` は Desktop だけ（`tauri` が無ければ項目を出さない）。作業ディレクトリの相対パス `Recent.mo` を使うので、UI がパスを知らなくてよい。Windows の測定にも同じファイルを置いた（置かないと `recent=ng` になるため）。

## やったこと

- `src/Rust/apps/desktop/src/main.rs`: `read_mo`・command `read_recent`・`generate_handler!` に追加・単体試験 1 本。
- `src/Svelte/src/lib/tauri-backend.ts`: `openPath` が `read_recent` を呼ぶ。`openFile` は fs のまま。
- `src/Svelte/src/App.svelte`: FMI の版の select（`fmiVersion`）、`smoke()` に `recent` の項目。
- `src/Svelte/tests/test_ui.py`: 既定は `fmiVersion="2.0"`、3.0 を選ぶと `fmiVersion="3.0"` の FMU。
- `.github/workflows/desktop.yml`: Linux・Windows の測定で `Recent.mo` を置き、合格条件に `recent=ok`。
- 文書: ADR-0023 の S154、`doc/features.md`（Text ビューと FMI 2.0 Export の行）、`doc/roadmap.md`（P4・P5 の行と S154 の欄）、`doc/sessions/` の索引・スライド・解説書。

## 検証

- 手元の Rust: `rustfmt --check`（edition 2024）のみ。clippy と単体試験は desktop.yml（手元に webkit2gtk が無い）。
- `pnpm check`: 0 エラー 0 警告。`pnpm build`: 通った。
- `test_ui.py`（main の `.venv/bin/python -I`、load average 0.2）: `ok`。「FMU: FMI の版に 3.0 を選ぶと modelDescription.xml が fmiVersion=3.0」を含み、既存の項目と `GM-SMOKE`（全項目 ok。Playwright は Web なので `recent` は出ない）も通った。
- `desktop.yml`: run 41975800269（`--ref feature/s154-ui-recent-scope-fmi3`、headSha 45a33a0）success。ui・Linux・Windows の 3 ジョブとも通った。desktop の workspace の clippy -D warnings と試験（`read_mo_reads_only_existing_mo_files` ok、9 passed）、Linux の測定は 3 回とも `… def=ok recent=ok`（合格条件の `grep -c … = 3`）、Windows は 3 回とも `GM-SMOKE check=ok … def=ok recent=ok ready_ms=188〜231`（cfg=none は設定ファイルを置いていないため従来どおり）
- `check_docs.py`: デッキ OK / リンク 3092 件中の切れ 0 件
- `render_slides.py`（S154 のデッキだけ）: デッキ 1 件 / 問題 0 件。
- セッション記録の `--check`: 親が写すので対象外。
- ステージした差分の漏洩検査: メールアドレス・ホームのパス・トークンは 0 件（`mermaid` の語はコードフェンスのみ）

## 残したこと

- 再読み込みをまたぐ未保存の印、タブの並べ替え、`--build` の UI、FMU import。
- `read_mo` はサイズの上限が無い。

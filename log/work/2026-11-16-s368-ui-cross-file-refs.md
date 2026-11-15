# S368 `ui-cross-file-refs`

| | |
| --- | --- |
| 日付 | 2026-11-16 |
| ブランチ | `feature/s368-ui-cross-file-refs` |
| worktree | `.git-worktree/s368-ui-cross-file-refs` |
| Phase | P4 |
| セッション文書 | `doc/sessions/S368-ui-cross-file-refs/` |
| セッション記録 | なし（このセッションの専用の記録が `log/local/` に無いため省く） |

## 依頼

S366 の確認（LSP の `textDocument/references`・`rename` は S230・S235 で開いていないファイル・ライブラリ・継承した部品まで届くが、UI は開いている 1 文書の中だけ）を受けて、UI の参照・リネームを開いている全部のタブに広げる最小版。LSP の `globe/references`・`globe/rename` と wasm の `references`・`rename` が、ワークスペースに入っている全部の文書を見て、ファイルごとの位置・`changes` を返す。規則は S230・S235 の `textDocument/*` を再利用し、新しい規則は作らない。ライブラリのファイルは書き換えない（参照の一覧には出してよい）。UI は一覧の各位置をタブで開けるようにし、リネームは開いている各タブの本文に当てる（Svelte に業務ロジックを置かない）。並行で S367（`globe-mode-flat`）が動くので flat は触らない。

## 決めたこと

- **応答の形を LSP に揃えた（互換を切る）**: `globe/references`・wasm `references` は `[{uri, range}]`、`globe/rename`・wasm `rename` は `{changes: {名前: [TextEdit]}}`。配列の応答が複数ファイルを表せないことが S230 の見送りの理由の 1 つだった。UI の `Backend` 以外に利用者はいないので、旧形も別名の口も残さない。
- **規則は `definition::references_in`・`rename_in`**（`globe-mode-lsp`）に 1 つ。`textDocument/*` と同じ `component_name_at`・`component_hits`・`check_component_name`・`class_name_at`・`class_hits`・`rename` を文書の組に回すだけ。wasm と sidecar の `globe/*` が共有する（`Doc {key, path, text}` の組。wasm は `inner.files()` と facade に足した `Workspace::text`、sidecar は `didOpen` 済みの `texts`）。`textDocument/*` の中身は変えていない（`globe/*` の分岐を別の口に切り出しただけ。`textDocument/rename` の計算は同じ）。
- **ディスクは見ない**: sidecar の根（Desktop は作業フォルダ）の下の開いていない `.mo` は、UI の未保存の本文と食い違い、適用先のタブも無いので `globe/*` は読まない（S230 の「UI はメモリ上の文書」を保つ）。**ライブラリのクラスの参照だけ一覧に足す**（`library: true`。LSP は `disk_class_hits(None, libs)`、wasm は読み込んだ木の本文を走る `library_references`）。リネームは足さない。
- **ライブラリで宣言した部品のリネームは断る**（`textDocument/rename` の `component_rename_blocked` と同じ理由の文。組の外のファイルは書き換えない）。クラスのリネームは、これまでどおりそのクラスを定義した文書の中から（使う側からは「別のファイルで定義されたクラス」で断る。S166 のまま）。
- **離れたタブの本文は呼ぶ間だけ Workspace に入れる**（`src/Svelte/src/lib/cross-file.ts` の `acrossDocs`）。S141 以来、UI はタブを離れるとそのファイルを Workspace から外している（同じクラスの二重定義を避ける）。終わったら（断られても）空に戻す。
- **リネームの適用**: 今のタブは今までどおり 1 回の dispatch（undo 1 回。モデル名の追従は S186 のまま）、ほかの自分のタブは `state.update` でそのタブの本文に当てる（そのタブの Ctrl+Z で戻る）。ライブラリのタブ・開いていないファイルの編集は当てず、箇所の数を知らせる。全タブを 1 回の Ctrl+Z で戻すには UI の履歴を束ねる仕組みが要るので入れていない。
- **参照の一覧**: 複数ファイルのときだけファイル名の列を足し、今のタブ → タブの順 → ライブラリの順に並べる。別のタブの行を押すとそのタブへ移り、一覧は残す（`activate` は一覧を捨てるので押す前に取っておく）。ライブラリの行は押せない（UI にタブが無い）。
- 実行・検査の最中（`busy`）は参照・リネームに入らない（Workspace を書き換えるため）。

## やったこと

- `src/Rust/mod/globe-mode-lsp/src/definition.rs`: `Doc`・`references_in`・`rename_in`・`library_references`・`offset_of`。`references_at`・`rename_at` は文書の組を取って `[{uri, range}]`・`{changes}` を返す。
- `src/Rust/mod/globe-mode-lsp/src/lib.rs`: `globe/references`・`globe/rename` を専用の口にして `texts` の全部で回す。`textDocument/rename`・`prepareRename` の口から `globe/*` の分岐を除いた（計算は同じ）。LSP の表を直した。
- `src/Rust/mod/globe-mode/src/lib.rs`: `Workspace::text`（3 行。wasm が文書の組を作る）。
- `src/Rust/mod/globe-mode-wasm/src/lib.rs`・`web/client.js`・`web/worker.js`: `references`・`rename` が組を作る（ライブラリの参照つき）。コメントを直した。
- `src/Svelte/src/App.svelte`・`lib/backend.ts`・`lib/tauri-backend.ts`・`lib/cross-file.ts`: 上の UI。
- 試験: `definition.rs`（2 文書にまたがるクラス・継承した部品・拒否）、`lib.rs`（`globe/*` が `textDocument/*` と同じ `changes`・開いていない文書・ライブラリの参照）、`globe-mode-wasm/tests/decay.rs`・`node/smoke.mjs`（wasm32 の上で 2 ファイル）、`test_tauri.mjs`（`acrossDocs` と `globe/*` の口）、`test_ui.py`（2 タブの `cross_file_refs`、ChuaCircuit の Resistor の Shift+F12 にライブラリの行）。
- 文書: features・README（3 か所）・roadmap・architecture・CHANGELOG・ADR-0023 の付記（S368）、セッション文書、索引。

## 検証

- `cargo fmt --check`: 差分 0。`cargo clippy -p globe-mode -p globe-mode-c -p globe-mode-cli -p globe-mode-lsp -p globe-mode-nif -p globe-mode-py -p globe-mode-wasm --all-targets -- -D warnings`（`ci_plan.py` の `rust_packages`）: 警告 0。
- `cargo test -p globe-mode-lsp`: 78 本通過・1 本 ignored（MSL が要るもの。新規 2 本 `references_and_rename_reach_every_open_document`・`globe_references_list_library_hits_but_rename_leaves_them`、`class_references_rename_and_prepare_over_rpc` ほか 3 本を書き直し）。`cargo test -p globe-mode-wasm`: 31 本通過・1 本 ignored（新規 2 本）。`cargo test -p globe-mode --test check_with`・`-p globe-mode-cli --test lsp --test check_native`: 通過。`cargo test --no-fail-fast -p globe-mode -p globe-mode-cli` の全体（`msl_examples_match` 389 s を含む）は、共有 target の古いビルドが混ざった回（下）で 0 件落ちた。
- **共有 target の古いビルド（S367 と同時）**: `globe_mode_lsp` の試験のバイナリが S367 の worktree のビルドで入れ替わり、自分の新しい試験 2 本が走らず、native の `expandable` の試験（`check_with`・`check_selects_the_frontend` など）が落ちた。`touch -c` で flat・frontend・lsp ほかを触って直後に回し直して通した。
- `mise run wasm:build`（release の wasm）→ `mise run wasm:node`: `smoke.mjs`（新しい「参照・リネーム」の項を含む。MSL 込み）・`broken-chunks.mjs`・`split.mjs --limit 5`（5 本が全部読みと一致）が通った。`mise run wasm:compare`: **480 件中 466 件が一致、既知の差 13 件、一致しない 1 件**（`Pendulum` / `dopri853`、相対差 2.0e-9）。S347・S322 の記録と同じ数字で、実行系は触っていない。
- `pnpm check`（svelte-check）: 0 errors・0 warnings。`node src/Svelte/tests/test_tauri.mjs`: ok。`uv run python src/Svelte/tests/test_ui.py`（`data/msl` あり。release の wasm と `pnpm build` の dist）: 通過（最後の `ok`）。新しい `cross_file_refs` は A.mo の `Base` の Shift+F12 で 4 行（A.mo 2・B.mo 2、ファイル名の列つき）、B.mo の行で B.mo のタブへ移る、F2 で Core に変えると A.mo・B.mo の両方が変わり、B.mo の Ctrl+Z で戻る。ChuaCircuit の Resistor の Shift+F12 は 49 行（ライブラリの押せない行 46）で 0.5 s。
- `check_docs.py`: スライド全部 OK・リンク 文書 1332 件 / 相対リンク 5508 件 / 切れ 0 件。`render_slides.py`（自分のデッキだけ）: デッキ 1 件 / 問題 0 件（1 回目は `Page.screenshot` が 30 s で時間切れ、2 回目で通った）。`markdownlint-cli2` は入っておらず `pnpm dlx` での取得は許可されなかったので回していない（見出し・リスト・表・コードブロックの前後の空行は手で確かめた）。
- 1 回目の `test_ui.py` は `data/msl` を外して回し、`?smoke=1` の `msl=ng failure=MSL.json`（MSL.json が無いと smoke の MSL の項が落ちる。この WP と無関係）で止まった。それまでの試験（新しい 2 タブの試験を含む）は通っていた。`data/msl` を張って回し直して全部通った。

## 残したこと

- 開いていないファイル（ディスクの根の下）の参照・リネーム。UI の本文が正でディスクと食い違うので、UI に「開いていないファイルを開く」操作を足すか、Rust が突き合わせる設計が要る。
- 全タブの変更を 1 回の Ctrl+Z で戻すこと（UI の履歴を束ねる）。
- 同じクラスを 2 つのタブが定義しているとき（`Resolver` が片方に寄せる）。
- 部品の参照でライブラリの継承の鎖の先は一覧に出さない（LSP の `textDocument/references` は出す）。
- 実機の Desktop（sidecar の `globe/*` を Tauri 越しに）は回していない。sidecar 側は LSP の試験（`globe/*` の `file`・`line`・`character`）と、Desktop の口の形を偽の `invoke` で確かめた `test_tauri.mjs` まで。
- main へのマージ・push・worktree の削除は親が行う（索引 `doc/sessions/README.md` は S367 と衝突しうる）。

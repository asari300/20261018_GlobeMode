# S248 `parameter-dialog`

| | |
| --- | --- |
| 日付 | 2026-11-09 |
| ブランチ | `feature/s248-parameter-dialog` |
| worktree | `.git-worktree/s248-parameter-dialog` |
| Phase | P8 |
| セッション文書 | `doc/sessions/S248-parameter-dialog/` |
| セッション記録 | [2026-11-02-a2580a13-3889-4662-9db4-a45dc1ca6772.md](../sessions/2026-11-02-a2580a13-3889-4662-9db4-a45dc1ca6772.md) |

## 依頼

roadmap の P8「パラメータダイアログ」の読み取りと実行時の上書き。facade `parameters(model)`（部品ごとの入れ子の名前・型・既定値・単位）、wasm / LSP / `Backend` の口、UI の `ParameterDialog.svelte`。ソースの書き換え・`redeclare`・配列の要素ごとの編集・VS Code・Leptos/Dioxus はやらない。

## 決めたこと

- 情報源は flat モデルの Parameter 変数を `compile_strict` で `CompiledModel::parameters()` にしておく（Rumoca の型は `rumoca.rs` の外へ出さず、`globe-mode-ir::ModelParameter`）。式は評価せず字面、Real / Integer の式だけ facade が `tunable_parameters` の値で `value` を補う。
- 既存の実行の設定の一覧（`tunableParameters`、Real の tunable だけ）は残し、`picked` を共有する。そのため `picked` から「一覧に無い名前を消す」処理を外した（Boolean / Enumeration が消えるため。古い名前は実行の GM-E10006）。
- 部品の `<g>` に透明な `path` を足した（線だけのアイコンの中が押せず、ダブルクリックが届かなかった）。
- GM-E10006 は診断の文中の名前（識別子の一部には当てない）で欄に結ぶ。
- 図の `data-part` の名前を絞り込みの先頭（`c.`）にする。図のクラスと検査したモデルが違うときは絞り込みに該当が無いだけ（絞り込み欄は編集できる）。

## やったこと

`globe-mode-ir`（`ModelParameter`）、`globe-mode-frontend`（`parameters()`、試験 `tests/parameters.rs`）、`globe-mode-protocol`（`ModelParametersResponse`）、`globe-mode`（`Workspace::parameters`）、`globe-mode-lsp`（`globe/parameters`）、`globe-mode-wasm`（`parameters`、`web/client.js`・`worker.js`、`tests/decay.rs`、`node/smoke.mjs`）、Desktop の `RPC_METHODS` 25 → 26、Svelte（`ParameterDialog.svelte`、`SimSettings`・`DiagramView`・`DocsPanel`・`App` の配線、`backend`・`protocol`・`tauri-backend`・`worker-backend`、`shape-svg.ts`、`test_ui.py` の `parameter_dialog`）、文書（features、roadmap、Svelte README、CHANGELOG、セッション文書、索引）。

## 検証

- `cargo test -p globe-mode-frontend --test parameters`（Real / Integer / Boolean / Enumeration、入れ子の部品、修飾で変えた既定値 `c.k = -5`・`c.speed = 'x y'`、`unit`・`min`・`max`、式の既定値は字面、配列と String）pass。`cargo test -p globe-mode-wasm --test decay decay_lists`（2 本）pass。
- `wasm:node`（`smoke.mjs`: 自分のモデルの `c.k` = 5・`c.on` = 0・`c.n` = 3、ChuaCircuit の `C1.C`）ok。
- `pnpm check` 0 エラー 0 警告。`test_ui.py` の `parameter_dialog`（自分のモデル: ダブルクリック、`c.` の絞り込み、Boolean・Enumeration・Integer の部品、`c.k = 2` で x(1) = e⁻²、「既定に戻す」で消えて e⁻¹、小数の Integer の GM-E10006 が欄の近くに出る。MSL: ChuaCircuit の C1 → C1.C）pass。全体の `test_ui.py` は ci.yml に任せた。
- 時間（wasm、Node）: ChuaCircuit 16 本 33〜94 ms、Fluid.PumpingSystem 187 本 約 1.1 s、Fluid.HeatingSystem 364 本 約 4.7 s（`tunable_parameters` とほぼ同じ。lowering が支配）。
- `ci.yml` の全体検査（feature ブランチ、run 42567818478、コミット 98117c7）は 18 ジョブすべて success。手元の `mise run check` は結果の記録が残っていない（サブエージェントが報告の前に止まった）。回し直さず、同じコミットの全体検査（fmt・clippy・test・deny を含む）で代える。

## 残したこと

修飾の編集・`redeclare`、配列の要素ごとの編集、VS Code への配線。Fluid の大きいモデルでは開くまで数秒（lowering による補いを遅延にする余地あり）。

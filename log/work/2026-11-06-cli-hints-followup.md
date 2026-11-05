# S187 `cli-hints-followup`

| | |
| --- | --- |
| 日付 | 2026-11-06 |
| ブランチ | `feature/s187-cli-hints-followup` |
| worktree | `.git-worktree/s187-cli-hints-followup` |
| Phase | P4 |
| セッション文書 | `doc/sessions/S187-cli-hints-followup/` |
| セッション記録 | （親が写す） |

## 依頼

S185 の再点検（`doc/research/app-flow-2026-11-06.md`）が見つけた、S180・S182 の手当ての効いていない所・食い違いを直す。R5・E4・E10b・R6（開いている文書を先に）・穴 6・12・R1・R10・Python の文書。記述どおりに再現してから直す。整形（`src/Modelica/`）、終了コードの体系、`fmt --check` の標準入力、`msl fetch`、ヘルプの言語、FMU は範囲外。

## 決めたこと

- **E5 の原因は S185 の見立てと違った。** 補足の条件は正しかったが、CLI は `Workspace::with_libraries`（分割読み込み）で読むので、Checker の持つライブラリの名前が `library:<パス>` で、そこから `package.mo` を探していた。S180 の試験は `Checker::new` を直に呼んでいたので通っていた。名前をパスに戻して探す（`library_root_hint` は 1 つのまま）。
- **E4 は半分は直っていた。** FILES を 1 つも渡さないとき（`check -m Modelica.Blocks.…`）は S180 の案内が出る（実出力で確認）。出なかったのは、FILES を渡して `-L` を忘れたとき（「候補: A」だけ）。frontend に置いたら db の既存の試験が落ちた（ワークスペースにあるパッケージ名にも出た）ので、ワークスペースの索引を持つ db の `add_workspace_candidates` に移した。clap の拾い方を `<INPUTS>` にも広げ、`deps`・`graph` にも FILES の案内が出る。
- **E10b**: パッケージ名には `GM-E1002` を使う（新しいコードは足さない）。文面は「`X` はパッケージで、モデルではないので検査も実行もできない」、補足は直下のモデル 3 つ（ほか n 件）。モデルが無ければ直下のパッケージ。
- **R6**: 距離で候補にするのは 4 文字以上で、距離 × 2 < 長さ（半分より少ない違い）。前方一致は 3 文字以上。開いている文書のモデルはライブラリより先に、3 つ以下なら遠くても候補にする。
- **穴 12**: `deps` は CLI だけで `-L` を受け、ライブラリのファイルも入力と同じにワークスペースへ読む（facade は変えない。Python の `ws.deps`・wasm の挙動は同じ）。`outline` は 1 ファイルの字句の構造なので `-L` を足さず `<FILE>` の説明だけ。
- **Python の docstring（WP 案 A）は不要だった。** `Workspace` にもモジュールにも docstring の無い公開メソッド・関数は 0 件（`inspect` で確かめた。S181 で足し済み）。コードは変えていない。
- **R10**: README の例を `globe fmt --check model.mo`（自分のファイルを指す）に。`src/Modelica/` は整形していない（119 ファイルの内訳は解説書 3.2）。

## やったこと

- `globe-mode-frontend/src/rumoca.rs`: `library_root_hint`（組の名前をパスに戻す。相対の `-L` は親も相対）、`library_model_not_found`（パッケージ名の診断、閾値、開いている文書のモデルを先に）。
- `globe-mode-db/src/lib.rs`: `add_workspace_candidates` に「`Modelica` を読み込んでいない」の 1 行。
- `globe-mode-cli/src/main.rs`: `deps` の `-L`、`outline` の `<FILE>` の説明、FILES の案内を `<INPUTS>` にも。
- 試験: `frontend/tests/library_hints.rs`（小さなライブラリ 5 本 + MSL `#[ignore]` 2 本）、`db/tests/deps.rs` 1 本、`cli/tests/cli_flow.rs` 4 本（MSL `#[ignore]` 1 本）、`json_snapshots.rs` + `snapshots/check-library-short-name.json`、`src/Python/tests/test_read.py` 1 本。
- 文書: README（既知の限界の 2 行、秒数、`fmt --check` の例）、CHANGELOG、`doc/features.md`・`doc/roadmap.md`、索引、解説書・スライド。

## 実出力（MSL 4.1.0、release、`head` で切っていない）

E5（`-L` が MSL のサブフォルダ）。前は最後の `note:` が無かった:

```text
data/msl/Modelica 4.1.0/Constants.mo: error[GM-E1001] S1 名前解決: unresolved component reference: 'ModelicaServices.Machine.eps'
    note: rumoca: ER002
    note: 失敗したクラス: <resolve>
    note: ほかに 3 件の失敗がある
    note: ライブラリ data/msl/Modelica 4.1.0 は 1 つのパッケージで、同じ親のフォルダに ModelicaServices などが並んでいる。親のフォルダ（data/msl）を -L に渡す（自動では足さない）
Modelica.Blocks.Examples.PID_Controller: NG（S1 名前解決 で停止）
終了コード 1
```

そのほか（`globe3` = このブランチの release）:

```text
$ globe check a.mo -m Modelica.Blocks.Examples.PID_Controller
error[GM-E1002] S1 名前解決: モデル `Modelica.Blocks.Examples.PID_Controller` が見つからない
    note: 候補: A
    note: `Modelica` というパッケージは読み込んでいない。ライブラリのモデルなら、ライブラリのルートも渡す（CLI: -L data/msl、Python: Workspace("data/msl")、UI: 「MSL を読む」）
Modelica.Blocks.Examples.PID_Controller: NG（S1 名前解決 で停止）
$ globe check -L data/msl -m Modelica.Blocks
error[GM-E1002] S1 名前解決: `Modelica.Blocks` はパッケージで、モデルではないので検査も実行もできない
    note: 直下のパッケージ: Modelica.Blocks.Continuous, Modelica.Blocks.Discrete, Modelica.Blocks.Examples（ほか 13 件）。中のモデルを完全修飾名で渡す
Modelica.Blocks: NG（S1 名前解決 で停止）
$ globe check -L data/msl -m Modelica.Blocks.Examples
error[GM-E1002] S1 名前解決: `Modelica.Blocks.Examples` はパッケージで、モデルではないので検査も実行もできない
    note: 直下のモデル: Modelica.Blocks.Examples.BooleanNetwork1, Modelica.Blocks.Examples.BusUsage, Modelica.Blocks.Examples.CompareSincExpSine（ほか 19 件）。中のモデルを完全修飾名で渡す
Modelica.Blocks.Examples: NG（S1 名前解決 で停止）
$ globe check -L data/msl -m Nope
error[GM-E1002] S1 名前解決: モデル `Nope` が見つからない
    note: ライブラリのモデルは、パッケージ名から始まる完全修飾名で渡す（例: Modelica.Blocks.Examples.PID_Controller）
Nope: NG（S1 名前解決 で停止）
$ globe check -L data/msl -m T
error[GM-E1002] S1 名前解決: モデル `T` が見つからない
    note: ライブラリのモデルは、パッケージ名から始まる完全修飾名で渡す（例: Modelica.Blocks.Examples.PID_Controller）
T: NG（S1 名前解決 で停止）
$ globe deps -m X
error: the following required arguments were not provided:
  <INPUTS>...

Usage: globe3 deps --model <MODEL> <INPUTS>...

For more information, try '--help'.

ライブラリのモデルなら FILES は要らない。-L にライブラリのルートを、-m に完全修飾名を渡す:
  globe check -L data/msl -m Modelica.Blocks.Examples.PID_Controller
```

前（S185 の `2ec6105` 相当、`globe0`）: `Nope` は `Logical.Nor, Logical.Not, MathBoolean.Nor, MathBoolean.Not, ComplexMath.Bode`、`T` は `Continuous.PI, Interfaces.MO, Interfaces.SO, Logical.Not, Logical.Or`、`Modelica.Blocks` は `GM-E1001 Modelica.Blocks could not be compiled because resolve/parse failed`。

## 検証

- `mise run check`（2 回目。1 回目は db の `switching.rs` 2 本が落ちて直した）: 通った（695 passed、0 failed。fmt・clippy・deny・`uv sync --locked`）。deny の `unmatched wrapper` は既存の警告
- `mise run msl:compare`: 20 本中 20 本が一致（両方とも成功 17 本）。判定（OK・GM-U・NG の本数）は変えない変更なので `msl:funnel`・`msl:sim` は回していない
- `mise run wasm:node`: 通った（`lib.rs:539` の panic の行番号で、共有 target の古いビルドでなく自分のソースだと確かめた。最初の 1 回は S186 のビルドの出力で、行番号が 542 だった）
- Python（`uv sync --reinstall-package globe-mode` のあと `pytest src/Python/tests`）: 21 passed、1 skipped。共有 target の `lib_native.so` は別の worktree に上書きされるので、`touch` してから作り直した（最初の 3 回は古い `.so` で新しい試験が落ちた）
- `check_docs.py`: デッキ OK、リンク切れ 0 件（文書 755 件・相対リンク 3807 件）。`render_slides.py`（S187 のデッキだけ）: 問題 0 件
- セッション記録の `--check`: 取りこぼし 0 件（親が写してから）
- ステージした差分の漏洩検査: 0 件
- 全体検査（`gh workflow run ci.yml --ref feature/s187-cli-hints-followup -f full=true`）: run 42116936490 が 11 ジョブ success（rust-linux・rust-windows・rust-sundials・wasm-node・python・lang・msl・ui・docs・deny・plan）。1 回目で通った

## 残したこと

- `src/Modelica/` の整形（ユーザーの判断。119 ファイルの内訳は解説書 3.2）、終了コードの体系・`fmt --check` の標準入力・`msl fetch`・ヘルプの言語。
- UI のモデル名の追従（R2）・入れ子のクラスの位置（R3）・FMU（R4）は別の WP。
- パスに `,` を含む `-L` では、組の名前から戻せず親のフォルダの案内が出ない（`shortcut:` に書いた）。
- 短い名前を完全修飾名に補う提案。

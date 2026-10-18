# S004 lazy-queries

| | |
| --- | --- |
| 日付 | 2026-10-18 |
| ブランチ | `feature/s004-lazy-queries` |
| worktree | `.git-worktree/s004-lazy-queries` |
| Phase | P2 遅延クエリと検査（1 回目） |
| セッション文書 | [doc/sessions/S004-lazy-queries/](../../doc/sessions/S004-lazy-queries/解説書.md) |
| セッション記録 | [log/sessions/2026-10-18-cf689fe6-f0d2-41f8-a25d-1a34aca756c7.md](../sessions/2026-10-18-cf689fe6-f0d2-41f8-a25d-1a34aca756c7.md)（S001 と同じセッション） |

## 依頼

「S004 を進めて」。

- connect のテストでは、型（Real、Boolean、Medium など）を試すテスト用のモデルを `src/Modelica` に作る。
- そのディレクトリを Modelica のパッケージになるよう整える。
- 例: `Modelica/任意のパッケージ名Validation/` のようにパッケージ化し、ほかの Modelica の IDE でも閲覧できるようにする。

## 決めたこと

- **パッケージ名**: `GlobeModeValidation`（`src/Modelica/GlobeModeValidation/`）。
  - 1 ファイルのサブパッケージ（Connectors、Components）とディレクトリのサブパッケージ（Connect、Invalid）を混ぜ、読み込み側の両方を試す。
  - 1 モデル 1 ファイル、`package.order` あり。
- **`extends Modelica.Icons.*` は書かなかった**（最初は書いた）。
  - パッケージにアイコンの extends があると、名前解決のために、Real や Boolean だけのモデルの検査にも MSL が要る。
  - `uses(Modelica(version="4.1.0"))` の注記は残し、ほかの IDE では MSL が読み込まれるようにした。
- **`src/Modelica/SyntaxError.mo` を `globe-mode-frontend/tests/data/` に移した**。`src/Modelica` を MODELICAPATH として開けるライブラリに保つため（壊れたファイルがあるとほかの IDE が開けない）。
- **Medium の流体ポート**は、MSL の Fluid を使わずに自作した。
  - 中身は flow の `m_flow`、`p`、stream の `h_outflow`、`inStream`。
  - 物性は `Modelica.Media.Water.ConstantPropertyLiquidWater`。
  - 方程式数は手で数えて 12 = 12 になるよう設計し、実際に一致した。
- **防火壁は `outline_shape`**（位置を落とした写し）にした。
  - `Outline` は位置（`TextRange`）を含むので、空白の編集では変わってしまう。
  - 位置を 0 にした写しを別のクエリにし、Eq で比べられるようにした。
- **connect グラフの辺の種類**は、コネクタのクラスから決める（flow / stream / 組み込みの型を継承した信号）。
  - そのためにアウトラインへ `flow` / `stream` と `input` / `output` を足した。
  - Rumoca の `Causality` は公開されていないので、Debug の表記で見分けた。
- **名前解決**は字句的なスコープ（内側から外側）だけにした。import と、継承したクラスの中の入れ子のクラスは後回し。
- **作業中のパッケージは文書として読む**ことにした（`globe check <DIR>`）。
  - Rumoca v0.10.2 は、ライブラリとして読んだクラスから別のライブラリを extends できない。最小のモデル 4 通りで確かめた。
- **GM-U は検討だけ**にした。案 A / B / C を比べ、B を既定、C を任意とする案を S005 で決める。
- **時間の計測は無視指定（`#[ignore]`）のテスト**にした。CI では時間が揺れるため。

## やったこと

- **新しい crate**:
  - `globe-mode-db`: salsa 0.28.5。クエリ 4 つ、実行の記録、`load_dir`。
  - `globe-mode-graph`: connect グラフ、辺の種類、depth、graphology の JSON。
- **IR**:
  - `ConnectPrefix`、`Causality` を足した。
  - `Outline::without_ranges`、`ClassIndex`（resolve / interface / connects / builtin_base）を足した。
- **frontend**: アウトラインに prefix と causality を入れた。
- **CLI**:
  - `globe graph` を足した。
  - `check` と `graph` で、ディレクトリを入力にできるようにした。
- **Modelica**: `GlobeModeValidation`（12 ファイル）、`src/Modelica/README.md`。
- **テスト**:
  - db 13 件（型ごとのグラフ、depth、防火壁 3 通り、構文の誤り）。
  - frontend 3 件（検証用モデルの検査、型の誤りで S4、MSL が無いと Medium は S1）。
  - ir 2 件。
- **CI**:
  - 判定に graph と db を加えた（wasm も）。
  - サンプルの検査に `globe graph` と BooleanSignals を足した。
  - msl ジョブに MediumFlow を足した。
  - 判定のテストは 16 件。
- **文書**:
  - ADR-0002 と ADR-0005 に結果を書いた。
  - architecture、roadmap、licenses、features、research/rumoca、CLAUDE.md を更新した。
  - S004 のスライドと解説書、索引。

## 検証

- `mise run check`: すべて通過。
  - fmt の差分 0、clippy の警告 0
  - テスト 32 件（無視指定 1）
  - cargo-deny 4 項目 ok
  - `uv sync --locked`
- 防火壁: 空白とコメントだけの編集で、`class_index` と `connect_graph` は 0 回だった。connect を足すと 1 回ずつ実行された。
- 再計算の時間: 中央値 0.20 ms、最大 0.30 ms（`cargo test --release -p globe-mode-db -- --ignored --nocapture`）。
- wasm32: `cargo check -p globe-mode-graph -p globe-mode-db` が通過（2 分 41 秒）。
- Rumoca の CLI で検証用モデルを確かめた: RealCircuit・RealSignals・BooleanSignals・Subsystem・Hierarchical は Balance 0。SignalTypeMismatch は EF002。
- `globe check`:
  - MediumFlow は MSL ありで 12 = 12。
  - SignalTypeMismatch は S4 / GM-E4001。
- MSL での突き合わせ（`cargo xtask msl-compare`、アダプタを変えたので回し直した）: 20 本中 20 本が一致（両方とも成功 17 本）、終了コード 0。
- `test_ci_plan.py` は 16 件すべて通過。actionlint の指摘は 0 件。
- Playwright（S004 のデッキ）: 問題 0 件。目で見て、S05 と S07 の文字のはみ出しを直した。

## 追記: S004 の CI の失敗（run 39100593298）と修正

- **何が落ちたか**: rust-windows の `globe-mode-db` のテスト 3 件だけ。ほかの 6 ジョブ（msl を含む）は成功した。
  - `whitespace_and_comment_edits_stop_at_the_firewall`
  - `a_new_connect_reaches_the_graph`
  - `syntax_error_keeps_other_files_usable`
- **原因 1（改行）**: Windows のランナーは `.mo` を CRLF で checkout する。テストの `replace("equation\n", …)` が当たらず、編集そのものが起きていなかった。
- **原因 2（パス）**: `load_dir` は `\` 区切りのパスで登録するが、テストは `Connect/RealSignals.mo`（`/` 区切り）で差し替えていた。差し替えではなく、別のファイルとして追加されていた。
- **直したこと**（`feature/ops-windows-paths`）:
  - `.gitattributes` で `*.mo` と `package.order` を LF に固定した。
  - `GlobeDb` がファイルのキーの区切りを `/` にそろえるようにした（単体テスト 1 件を足した）。
  - テストは OS の区切りでパスを組み、本文の CRLF を LF に直してから使う。
- `.github/` を変えていないので、差分だけの CI では Windows のジョブが動かない。マージのコミットに `[full-ci]` を入れて、全体を検査した。
- 失敗したので、手順どおり worktree の片付けは見送った。修正の CI が通ってから行う。

## 残したこと

- `flat` と検査（S0〜S5）をクエリに載せる（S005）。
- GM-U の振り分けを決める（S005）。
- 名前解決の import と、継承した入れ子のクラス。for や if の中の connect。
- Rumoca の制約（ライブラリ間の extends）の上流への報告。

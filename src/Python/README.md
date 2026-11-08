# src/Python

- `mod/globe_mode/`: Python パッケージ `globe_mode`（Rust の facade を PyO3 で包み、maturin で wheel にする）。型は `py.typed` と `_native.pyi`。`uv run globe-mode` は版とソルバの数を出す
- `marimo/`: marimo のノートブックと anywidget（wasm を読む）
- `streamlit/`: 結果ダッシュボードとパラメータスイープ

使い方（MSL は `cargo xtask fetch-msl` で `data/msl` に取る。引数・戻り値・誤りは `help(globe_mode)`、`help(Workspace.simulate)`）。

```python
from globe_mode import Workspace, table

ws = Workspace("data/msl")  # ライブラリが 1 つなら文字列か Path でよい。複数ならリスト
model = "Modelica.Electrical.Analog.Examples.ChuaCircuit"
print(ws.check(model)["passed"])  # モデルの誤りは例外でなく passed: False と diagnostics
res = ws.simulate_columns(model, stop_time=2)  # 設定のキーが違えば TypeError（stoptime など）
print(res["ok"], table(res).shape)
```

- 読み取り（CLI の `globe outline` / `deps` / `flat --json` と同じ。`set_file` したファイルとモデルについて）: `ws.outline("A.mo")`（クラスの一覧。無いファイルと構文の誤りは `ValueError`）、`ws.deps("A")`（`check` が読むファイルのリスト）、`ws.flat("A")`（変数・方程式の数。通らなければ `None`）。
- 結果の 1 行: `ws.check(m)["summary"]`（`RC: OK（S0〜S10 を通過）`、止まれば `NG（S5 … で停止）`）。`simulate` の `summary` は実行しなかった・失敗したときだけ（理由と最初の error）。CLI の最後の行と同じ文。
- 誤りの型（S200）: ライブラリが出す例外は `GlobeModeError` の子で、`.diagnostics`（診断の dict のリスト。無ければ空）と最初の診断のコード `.code` を持つ。これまでの組み込みの型も継ぐので `except ValueError` などでも受けられる。**モデルの誤り・未対応は例外にせず**、結果の `passed` / `ok` / `diagnostics` で返す（`check` は未知のモデルも `passed: False`）。

  | 例外 | 継ぐ型 | 出る場合 |
  | --- | --- | --- |
  | `LibraryError` | `RuntimeError` | `Workspace(libraries)`・`load_dir` がライブラリを読めない（本文は最初の error の `コード: 文`、元の診断は `.diagnostics`） |
  | `SettingsError` | `ValueError` | `simulate` / `simulate_columns` の設定の値や型の誤り（NaN・inf を含む。どの設定かを言う） |
  | `ModelNotFoundError` | `ValueError` | `graph` のモデルが索引に無い |
  | `SourceError` | `ValueError` | `outline` のファイルが無い・構文の誤り（`.diagnostics`）、`format` の構文の誤り |
  | `ExportError` | `ValueError` | `export_fmu` が書き出せない（版が 2・3 でない、DAE にできない、書き込めない、`runtime="p5b"` の共有ライブラリを作れない・無い） |
  | `ResultError` | `ValueError` | `table` に時系列の無い結果（`ok: False`）を渡した |
  | `FmuError` | `RuntimeError` | `fmu_info` が FMU を読めない、`run_fmu` が回せない（FMU が落ちた・時間切れ・止めた・`globe` が見つからない） |

  呼び方の誤りは Python の習慣どおり素の `TypeError`（未知の設定のキー、`on_progress` が呼べない、`libraries` が bytes）で、`GlobeModeError` ではない。`on_progress` のコールバックが出した例外はそのまま通り、Rust の panic は `pyo3_runtime.PanicException`（`BaseException` の子）のまま。
- `simulate` / `simulate_columns` は実行の間 GIL を離す（`on_progress` が無くても。ほかの Python のスレッドは動き、Ctrl-C は進捗の区切りで効く）。`on_progress` は**キーワード専用**（`ws.simulate(m, {"stop_time": 2})` は `TypeError`。設定は `stop_time=2` で渡す）。コールバックの中から同じ `Workspace` は呼べない（`RuntimeError: Already borrowed`）。
- `on_progress` は既定のソルバ（`bdf`・`auto`・`dopri45`）では積分の中に届かない（段の変わり目だけ。積分を止めたいなら `max_wall_seconds`）。`esdirk34` などなら積分の時刻が付き、`True` を返して止められる。
- FMU の読み込みと実行（S229、FMI Import。[ADR-0026](../../doc/adr/0026-fmi-import-placement.md) の (p)）: `fmu_info("Dahlquist.fmu")`（`globe fmu info` と同じ情報の dict。ネイティブコードは実行しない）、`run_fmu("Dahlquist.fmu", kind="cs", stop_time=1, outputs=["x"], parameters={"k": 2}, timeout=60)`（`read_results` と同じ形の dict。`table(res)` で表になり、ME だけ `res["me"]` に事象の要約）。**FMU のネイティブコードを別プロセス（ワーカー）で実行する**ので、FMU が落ちても Python は落ちず `FmuError` になる（ディスク・ネットワークは隔離しない。信頼できる FMU だけ）。ワーカーは CLI の `globe`: 環境変数 `GLOBE_BIN`、無ければ PATH（wheel の中には実行ファイルが無い。見つからなければ `FmuError` が「`globe` を PATH に置くか GLOBE_BIN」と言う）。`on_progress(t)` が真を返すか `timeout` を超えるとワーカーを kill する。

経路の説明は [doc/architecture.md](../../doc/architecture.md#5-言語-api)。

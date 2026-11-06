# S200 `python-exceptions-review-fixes` のレビュー（Opus）

| | |
| --- | --- |
| 日付 | 2026-11-06 |
| ブランチ | `feature/s200-python-exceptions-review-fixes` |
| 対象 | Python の束縛（`globe-mode-py`・`globe_mode/`）。main cc846ba、PyO3 0.30.1 |
| レビュアー | Opus（medium）。読むだけ |
| 依頼の文面 | Python の束縛のレビュー。指摘は読んで論証しただけなので、修正する側が 1 件ずつ再現してから直す |
| 作業記録 | `log/work/2026-11-06-python-exceptions-review-fixes.md` |

## 指摘（原文のまま）

| # | 指摘（原文） | 採否 | 理由・再現 |
| --- | --- | --- | --- |
| 1 | [正しさ・確認済み] src/Python/mod/globe_mode/__init__.py:222 export_fmu の targets に文字列を渡すと list(targets) で 1 文字ずつ（S181 で libraries について直したのと同じ誤り）。直し方: `[targets] if isinstance(targets, str) else list(targets)`。 | 採った | 再現（`_ws` を差し替えて options の JSON が `["x","8",…]`）。`test_1_…` |
| 2 | [負荷+簡素化・確認済み] globe-mode-py/src/lib.rs:215-217,234-237,170-203,261 GIL を離すのは on_progress を渡したときだけ。simulate の間ほかの Python スレッド（marimo・streamlit のサーバ）が止まり、Ctrl-C も効かない。直し方: simulate / simulate_columns は on_progress の有無にかかわらず simulate_observed を通す 1 本の道に（コールバックが無ければ notify は check_signals だけ）。simulate_json と Rust 側の simulate_columns 関数を消せる。check・flat・deps・export_fmu は py.detach で包む。 | 採った | 再現（別スレッド 4 回、Ctrl-C は約 1.1 s の実行が終わるまで効かない）。PyO3 の規則を確認（解説書 2.3） |
| 3 | [正しさ・確認済み] __init__.py:59-75 np.int64・np.float32 を「数値のはず」と弾く。NaN・inf は JSON の誤りに化けてどの設定か言わない。直し方: `isinstance(value, numbers.Real) and not isinstance(value, bool)`、float() に直す、有限でなければ ValueError(設定名)。 | 採った | 再現（`Fraction`、`numbers.Real` 登録の偽物。numpy は入れていない）。有限でなければ `SettingsError`（`ValueError` でもある） |
| 4 | [正しさ・確認済み] __init__.py:150-155,184-189 on_progress が位置引数の 2 番目。ws.simulate("RC", {"stop_time": 2}) で dict が on_progress に入り、検査の後に 'dict' object is not callable。直し方: `def simulate(self, model, *, on_progress=None, **settings)`、呼べないものは入口で TypeError。 | 採った | 再現（signature、`on_progress=5`）。呼び方が変わる点として CHANGELOG に明記 |
| 5 | [負荷・写す回数は確認済み] lib.rs:62-67,243、__init__.py:201 列の道で全データを 3 回写しピークが 2 倍。直し方: Simulated を返し GIL を取り直してから PyBytes::new_with で直接詰める。polars の memoryview の扱いは未確認。 | 採った（一部） | 中間の `Vec<u8>` を無くした。測定でピーク 33〜36% 減（「2 倍」ではなく基準込みで 145 から 121 MiB）、時間は同じ。polars 側の写しは減らしていない |
| 6 | [正しさ・確認済み] __init__.py:92-96 Workspace(b"data/msl") が bytes を整数の列として読む。直し方: str \| bytes \| os.PathLike を 1 つとして os.fspath、または bytes は TypeError。Workspace("") は未確認。 | 採った | 再現（`ライブラリ 120`）。bytes は `TypeError`。`Workspace("")` は `GM-E1003` で落ちるので直さない |
| 7 | [正しさ・確認済み（論証）] __init__.py:72 parameters=None が Python の検査を抜け Rust で「settings が読めない: invalid type: null, expected a map」。直し方: 送る前に None の項目を落とす。 | 採った | 再現。`None` の項目は送らない |
| 8 | [試験] lib.rs:97-130 コールバックの枝に試験が無い: (a) KeyboardInterrupt 以外の例外の投げ直し、(b) コールバックの中で同じ ws を呼ぶ（借用の誤り）、(c) Ctrl-C の後に同じ ws を使える。docstring に「コールバックから同じ Workspace は呼べない」。 | 採った | (a)(b)(c) とも今のコードで通った。試験を足し、docstring に明記 |
| 9 | [型] _native.pyi:8,20 export_fmu の戻り値 -> str だが Rust は PathBuf（pathlib.Path になるはず）。__init__(libraries: list[str]) は PathLike の列も受ける。 | 採った | `.pyi` を直した（mypy 0 件） |
| 10 | [簡素化] __init__.py:96,106,222-223 パスを渡す前の str(...) は不要（os.fspath でない）。そのまま渡す。fmi_headers だけ os.fspath。 | 採った | 外した |
| 他 | table(ws.check(m)) は KeyError（__init__.py:249）。 | 採った | `ResultError` |

## 結論

- 採用 11 件（5 は一部）・保留 0 件・不採用 0 件。再現しなかった指摘は無い（8 は元から通ったので試験だけ）。
- レビュアーの誤り: 5 の「ピークが 2 倍」は誇張（基準の約 78 MiB を含めると 145 から 121 MiB）。写しの数（3 回から 2 回）は正しかった。

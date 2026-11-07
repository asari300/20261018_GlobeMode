"""GlobeMode が出す例外（S200）。Rust の束ね（`_native`）もここから名前で引いて出す。

`GlobeModeError` の子はすべて、これまで出していた組み込みの型も継ぐ（`except ValueError` などは壊れない）。
呼び方の誤り（未知のキーワード、引数の型）は Python の習慣どおり素の `TypeError` で、ここには入らない。
"""

from typing import Any


class GlobeModeError(Exception):
    """GlobeMode のライブラリが出す誤りの基底。`.diagnostics`（診断の dict のリスト。無ければ空）と `.code`（最初の診断のコード）。"""

    def __init__(self, message: str | None = None, diagnostics: list[dict[str, Any]] | None = None):
        self.diagnostics: list[dict[str, Any]] = list(diagnostics or [])
        super().__init__(_summary(self.diagnostics) if message is None else message)

    @property
    def code(self) -> str | None:
        return self.diagnostics[0]["code"] if self.diagnostics else None


class LibraryError(GlobeModeError, RuntimeError):
    """ライブラリ（ディレクトリ）を読めない。`Workspace(libraries)`・`load_dir`。本文は最初の error の `コード: 文`。"""


class SettingsError(GlobeModeError, ValueError):
    """`simulate` / `simulate_columns` の設定の値や型の誤り（どの設定かを本文が言う）。未知のキーは `TypeError`。"""


class ModelNotFoundError(GlobeModeError, ValueError):
    """モデルが索引に無い（`graph`）。`check` は例外にせず `passed: False` と診断で返す。"""


class SourceError(GlobeModeError, ValueError):
    """Modelica の本文の誤り: 構文の誤り、`set_file` していないファイル（`outline`・`format`）。"""


class ExportError(GlobeModeError, ValueError):
    """FMU に書き出せない（`export_fmu`。版が 2・3 でない、モデルを DAE にできない、書き込めない）。"""


class ResultError(GlobeModeError, ValueError):
    """結果に時系列が無い（`table`。実行できなかった結果を表にしようとした）か、結果のファイルを読めない（`read_results`）。"""


def _summary(ds: list[dict[str, Any]]) -> str:
    """診断のリストを、最初の error の `コード: 文（ファイル・バイト位置・補足）` と `ほか n 件` にする。"""
    if not ds:
        return ""
    d = next((d for d in ds if d["severity"] == "error"), ds[0])
    where = [d["file"]] if d.get("file") else []
    if r := d.get("range"):
        where.append(f"バイト {r['start']}〜{r['end']}")
    text = f"{d['code']}: {d['message']}" + (f"（{'、'.join(where + d.get('notes', []))}）" if where or d.get("notes") else "")
    return text + (f" ほか {len(ds) - 1} 件" if len(ds) > 1 else "")


for _c in (GlobeModeError, LibraryError, SettingsError, ModelNotFoundError, SourceError, ExportError, ResultError):
    _c.__module__ = "globe_mode"

# GlobeMode の整形・lint（marimo）。起動: uv run marimo run src/Python/marimo/fmt_app.py
# 計算は globe CLI（GLOBE_BIN、既定は PATH の globe）に任せる。globe_cli.py を同じフォルダから読む。

import marimo

__generated_with = "0.25.1"
app = marimo.App(width="medium", app_title="GlobeMode 整形・lint")


@app.cell
def _():
    import marimo as mo

    from globe_cli import format_text, lint_text

    return format_text, lint_text, mo


@app.cell
def _(mo):
    # 入力の本文は state に持つ。ファイルを選んだときと「入力へ戻す」で書き換える（UI 要素の値は直接書けないため）
    get_source, set_source = mo.state("")
    return get_source, set_source


@app.cell
def _(mo, set_source):
    upload = mo.ui.file(
        filetypes=[".mo"],
        label=".mo ファイルを選ぶ",
        on_change=lambda files: set_source(files[0].contents.decode("utf-8")) if files else None,
    )
    upload
    return (upload,)


@app.cell
def _(get_source, mo):
    source = mo.ui.text_area(
        value=get_source(), label="Modelica の入力", placeholder="model A ... end A;", rows=14, full_width=True
    )
    source
    return (source,)


@app.cell
def _(mo):
    indent = mo.ui.dropdown(options=["2", "4"], value="2", label="インデント")
    origin = mo.ui.checkbox(value=True, label="origin = {0, 0} を足す")
    mo.hstack([indent, origin], justify="start")
    return indent, origin


@app.cell
def _(format_text, indent, lint_text, mo, origin, source):
    def fmt_click(_):
        try:
            return format_text(source.value, indent=int(indent.value), insert_origin=origin.value)
        except RuntimeError as e:
            return {"ok": False, "text": None, "error": str(e)}

    def lint_click(_):
        try:
            return {"ok": True, "messages": lint_text(source.value)}
        except RuntimeError as e:
            return {"ok": False, "error": str(e)}

    fmt_btn = mo.ui.button(label="整形", kind="success", on_click=fmt_click)
    lint_btn = mo.ui.button(label="lint", on_click=lint_click)
    mo.hstack([fmt_btn, lint_btn], justify="start")
    return fmt_btn, lint_btn


@app.cell
def _(fmt_btn, lint_btn, mo, set_source):
    parts = []
    fmt = fmt_btn.value
    if fmt is not None and fmt["ok"]:
        back = mo.ui.button(label="入力へ戻す", on_click=lambda _: set_source(fmt["text"]))
        parts += [mo.md("### 整形結果"), back, mo.plain_text(fmt["text"])]
    elif fmt is not None:
        parts.append(mo.callout(mo.plain_text(f"整形できない: {fmt['error']}"), kind="danger"))

    lint = lint_btn.value
    if lint is not None and not lint["ok"]:
        parts.append(mo.callout(mo.plain_text(f"lint できない: {lint['error']}"), kind="danger"))
    elif lint is not None and lint["messages"]:
        parts += [mo.md("### lint"), mo.ui.table(lint["messages"], selection=None)]
    elif lint is not None:
        parts.append(mo.callout(mo.md("指摘はありません"), kind="success"))

    mo.vstack(parts)
    return


if __name__ == "__main__":
    app.run()

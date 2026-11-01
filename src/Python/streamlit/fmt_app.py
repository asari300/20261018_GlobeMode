"""GlobeMode の整形・lint の Streamlit 画面。起動は README.md を参照。"""

import streamlit as st

from globe_cli import GlobeError, fmt, lint

st.set_page_config(page_title="GlobeMode 整形・lint")
st.title("GlobeMode 整形・lint")

if "source" not in st.session_state:
    st.session_state["source"] = ""

# アップロードしたファイルは、新しいファイルを選んだときだけ入力欄に読み込む（編集を消さないため）
uploaded = st.file_uploader(".mo ファイルを選ぶ（入力欄に読み込む）", type=["mo"])
if uploaded is not None and st.session_state.get("loaded_id") != uploaded.file_id:
    st.session_state["source"] = uploaded.getvalue().decode("utf-8")
    st.session_state["loaded_id"] = uploaded.file_id

st.text_area("入力（Modelica）", key="source", height=300)

col_indent, col_origin = st.columns(2)
indent = col_indent.selectbox("インデント", [2, 4], index=0)
insert_origin = col_origin.checkbox("origin = {0, 0} を足す", value=True)

col_fmt, col_lint = st.columns(2)
do_fmt = col_fmt.button("整形")
do_lint = col_lint.button("lint")

source = st.session_state["source"]
try:
    if do_fmt:
        st.session_state["result"] = ("fmt", fmt(source, indent, insert_origin))
    if do_lint:
        st.session_state["result"] = ("lint", lint(source))
except GlobeError as e:
    st.session_state["result"] = ("error", str(e))


def _restore_input() -> None:
    # on_click は描画より前に動くので、入力欄の値を変えてよい
    st.session_state["source"] = st.session_state["result"][1]["text"]


result = st.session_state.get("result")
if result is not None:
    kind, body = result
    if kind == "error":
        st.error(body)
    elif kind == "fmt" and not body["ok"]:
        st.error(f"整形できません: {body['error']}")
    elif kind == "fmt":
        st.text_area("整形後（読み取り専用）", value=body["text"], height=300, disabled=True)
        st.download_button("整形結果をダウンロード", body["text"], file_name="formatted.mo")
        st.button("入力へ戻す", on_click=_restore_input)
    elif kind == "lint" and not body:
        st.success("lint の指摘はありません")
    elif kind == "lint":
        st.table(body)

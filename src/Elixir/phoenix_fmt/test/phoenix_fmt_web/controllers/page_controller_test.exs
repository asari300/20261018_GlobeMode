defmodule PhoenixFmtWeb.PageControllerTest do
  use PhoenixFmtWeb.ConnCase, async: true

  test "GET /healthz", %{conn: conn} do
    conn = get(conn, "/healthz")

    assert json_response(conn, 200) == %{"ok" => true}
  end

  test "GET / は日本語の画面で、外部 CDN を使わない", %{conn: conn} do
    conn = get(conn, "/")
    body = html_response(conn, 200)

    assert body =~ ~s(name="source")
    assert body =~ "整形"
    assert body =~ "lint"
    refute body =~ "https://"
    refute body =~ "http://"
  end
end

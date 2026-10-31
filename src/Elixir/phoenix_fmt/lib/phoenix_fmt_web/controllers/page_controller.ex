defmodule PhoenixFmtWeb.PageController do
  use PhoenixFmtWeb, :controller

  def index(conn, _params) do
    conn
    |> put_resp_content_type("text/html", "utf-8")
    |> send_file(200, Application.app_dir(:phoenix_fmt, "priv/index.html"))
  end

  def healthz(conn, _params), do: json(conn, %{ok: true})
end

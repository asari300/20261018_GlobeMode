defmodule PhoenixFmtWeb.ApiController do
  use PhoenixFmtWeb, :controller

  alias PhoenixFmt.Globe

  def fmt(conn, %{"text" => text} = params) when is_binary(text) do
    case fmt_args(params) do
      {:ok, args} ->
        case Globe.fmt(text, args) do
          {:ok, result} -> json(conn, result)
          {:error, msg} -> conn |> put_status(500) |> json(%{ok: false, text: nil, error: msg})
        end

      :error ->
        bad_request(conn, "indent は 0〜16、width は 1〜1000 の整数、insert_origin は真偽値")
    end
  end

  def fmt(conn, _params), do: bad_request(conn, "text（文字列）が必要")

  def lint(conn, %{"text" => text} = params) when is_binary(text) do
    disable = Map.get(params, "disable", [])

    if is_list(disable) and Enum.all?(disable, &(is_binary(&1) and &1 =~ ~r/\A[a-z][a-z-]*\z/)) do
      case Globe.lint(text, disable) do
        {:ok, result} -> json(conn, result)
        {:error, msg} -> conn |> put_status(500) |> json(%{error: msg})
      end
    else
      bad_request(conn, "disable は文字列の配列")
    end
  end

  def lint(conn, _params), do: bad_request(conn, "text（文字列）が必要")

  defp fmt_args(params) do
    indent = Map.get(params, "indent", 2)
    width = Map.get(params, "width", 80)
    insert_origin = Map.get(params, "insert_origin", true)

    if int_in?(indent, 0..16) and int_in?(width, 1..1000) and is_boolean(insert_origin) do
      origin = if insert_origin, do: [], else: ["--no-insert-origin"]
      {:ok, ["--indent", to_string(indent), "--width", to_string(width)] ++ origin}
    else
      :error
    end
  end

  defp int_in?(n, range), do: is_integer(n) and n in range

  defp bad_request(conn, msg), do: conn |> put_status(400) |> json(%{error: msg})
end

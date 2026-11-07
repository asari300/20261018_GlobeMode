defmodule PhoenixFmt.Globe do
  @moduledoc """
  globe CLI（環境変数 `GLOBE_BIN`、無ければ PATH の `globe`）を子プロセスで呼ぶ。
  計算は Rust 側だけで、ここは JSON を受け渡すだけ（doc/web-apps.md §1）。

  終了コード 1（文法エラー、lint の指摘あり）は正常な結果なので、JSON をそのまま返す。
  JSON が読めないときだけ `{:error, reason}`。
  """

  @doc "`globe fmt --json -` の標準出力の JSON（`%{\"ok\" => _, \"text\" => _, \"error\" => _}`）を返す。"
  def fmt(text, args), do: run(text, ["fmt", "--json", "-" | args])

  @doc "`globe lint --json -` の JSON（`%{\"messages\" => [...]}`）を返す。"
  def lint(text, disable) do
    run(text, ["lint", "--json", "-" | Enum.flat_map(disable, &["--disable", &1])])
  end

  defp run(text, args) do
    # shortcut: System.cmd は標準入力を渡せず、Port は入力だけ閉じられないため、一時ファイルを sh で標準入力へつなぐ。Windows では動かない、Port で直接パイプするなら書き直す
    tmp = Path.join(System.tmp_dir!(), "globe-web-#{System.unique_integer([:positive])}.mo")
    File.write!(tmp, text, [:exclusive])

    try do
      bin = System.get_env("GLOBE_BIN", "globe")
      script = ~s(f=$1; shift; exec "$@" < "$f")
      {out, _status} = System.cmd("sh", ["-c", script, "sh", tmp, bin | args])

      case Jason.decode(out) do
        {:ok, json} when is_map(json) -> {:ok, json}
        _ -> {:error, "globe の出力を読めない: #{String.slice(out, 0, 200)}"}
      end
    after
      File.rm(tmp)
    end
  end
end

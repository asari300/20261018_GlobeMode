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
    # #73: connect-type-mismatch は 1 MiB で 2.6 GB になるので常に止める
    run(text, ["lint", "--json", "-", "--disable", "connect-type-mismatch" | Enum.flat_map(disable, &["--disable", &1])])
  end

  defp run(text, args) do
    # shortcut: System.cmd は標準入力を渡せず、Port は入力だけ閉じられないため、一時ファイルを sh で標準入力へつなぐ。Windows では動かない、Port で直接パイプするなら書き直す
    tmp = Path.join(System.tmp_dir!(), "globe-web-#{System.unique_integer([:positive])}.mo")
    File.write!(tmp, text, [:exclusive])

    try do
      bin = System.get_env("GLOBE_BIN", "globe")
      script = ~s(f=$1; shift; exec "$@" < "$f")

      port =
        Port.open({:spawn_executable, System.find_executable("sh")}, [
          :binary,
          :exit_status,
          args: ["-c", script, "sh", tmp, bin | args]
        ])

      timeout = String.to_integer(System.get_env("GLOBE_TIMEOUT_MS", "10000"))

      case collect(port, "", System.monotonic_time(:millisecond) + timeout) do
        :timeout ->
          {:error, "globe が時間内に終わらなかった"}

        {:ok, out} ->
          case Jason.decode(out) do
            {:ok, json} when is_map(json) -> {:ok, json}
            _ -> {:error, "globe の出力を読めない: #{String.slice(out, 0, 200)}"}
          end
      end
    after
      File.rm(tmp)
    end
  end

  # 出力の塊ごとではなく全体で時間を数える。切るときは globe（sh が exec で置き換わっている）を kill する
  defp collect(port, acc, deadline) do
    receive do
      {^port, {:data, data}} -> collect(port, acc <> data, deadline)
      {^port, {:exit_status, _}} -> {:ok, acc}
    after
      max(deadline - System.monotonic_time(:millisecond), 0) ->
        with {:os_pid, pid} <- Port.info(port, :os_pid),
             do: System.cmd("kill", ["-KILL", to_string(pid)])

        Port.close(port)
        :timeout
    end
  end
end

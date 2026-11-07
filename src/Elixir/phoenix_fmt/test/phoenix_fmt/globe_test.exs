defmodule PhoenixFmt.GlobeTest do
  # 環境変数を書き換えるので async: false
  use ExUnit.Case, async: false

  @tag :tmp_dir
  test "遅い globe は時間切れで kill される", %{tmp_dir: dir} do
    slow = Path.join(dir, "slow.sh")
    File.write!(slow, "#!/bin/sh\nexec sleep 30\n")
    File.chmod!(slow, 0o755)
    System.put_env("GLOBE_BIN", slow)
    System.put_env("GLOBE_TIMEOUT_MS", "200")

    try do
      assert {:error, "globe が時間内に終わらなかった"} = PhoenixFmt.Globe.fmt("model A end A;", [])
    after
      System.delete_env("GLOBE_BIN")
      System.delete_env("GLOBE_TIMEOUT_MS")
    end
  end
end

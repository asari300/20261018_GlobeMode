defmodule GlobeModeSchedulerTest do
  # S202: 長い実行が ws の Mutex を持っている間に別のプロセスが set_file / format / lint を呼んでも、
  # BEAM の通常のスケジューラ（1 本に絞る）が塞がらない。塞がると、ただのタイマーのループが止まる。
  use ExUnit.Case

  @wiggle "model Wiggle Real x(start = 0, fixed = true); equation der(x) = sin(100 * time); end Wiggle;"

  # 10 ms ごとに起きるプロセス。起きる間隔の最大を覚える。
  defp ticker(parent) do
    spawn_link(fn -> tick(parent, System.monotonic_time(:millisecond), 0) end)
  end

  defp tick(parent, last, max_gap) do
    receive do
      {:stop, from} -> send(from, {:max_gap, max(max_gap, System.monotonic_time(:millisecond) - last)})
    after
      10 ->
        now = System.monotonic_time(:millisecond)
        tick(parent, now, max(max_gap, now - last))
    end
  end

  test "別プロセスの実行が Mutex を持つ間の set_file が通常のスケジューラを止めない" do
    online = :erlang.system_info(:schedulers_online)
    :erlang.system_flag(:schedulers_online, 1)

    try do
      {:ok, ws} = GlobeMode.new()
      GlobeMode.set_file(ws, "Wiggle.mo", @wiggle)
      token = GlobeMode.cancel_token()
      me = self()
      settings = %{solver: "esdirk34", stop_time: 5000, interval: 1}
      run = Task.async(fn -> GlobeMode.simulate_observed(ws, "Wiggle", settings, token, me) end)
      # 実行が積分に入った（ws の Mutex を持っている）
      assert_receive {:globe_progress, %{stage: "integration", time: t}} when is_float(t), 30_000

      tk = ticker(me)
      writer = Task.async(fn -> GlobeMode.set_file(ws, "Other.mo", "model Other end Other;") end)
      fmt = Task.async(fn -> GlobeMode.format(String.duplicate("model A Real x; end A;\n", 2000)) end)
      # 1 秒、ticker が回るかを見る（set_file はまだ Mutex を待っているはず）
      Process.sleep(1000)
      send(tk, {:stop, me})
      assert_receive {:max_gap, gap}, 30_000
      assert gap < 500, "通常のスケジューラが #{gap} ms 止まった"
      assert Task.yield(writer, 0) == nil, "set_file が実行の終わりを待っていない（試験が成り立たない）"

      GlobeMode.cancel(token)
      assert {:ok, _} = Task.await(run, 30_000)
      assert {:ok, _} = Task.await(writer, 30_000)
      Task.await(fmt, 30_000)
    after
      :erlang.system_flag(:schedulers_online, online)
    end
  end

  # 本文の長さに比例する NIF（lint は 1 MB で約 2 秒）も、通常のスケジューラで回さない
  test "format と lint が通常のスケジューラを止めない" do
    online = :erlang.system_info(:schedulers_online)
    :erlang.system_flag(:schedulers_online, 1)

    try do
      text = String.duplicate("model A Real x(start=1); equation der(x) = -x; end A;\n", 20_000)
      tk = ticker(self())
      lint = Task.async(fn -> GlobeMode.lint(text) end)
      fmt = Task.async(fn -> GlobeMode.format(text) end)
      assert is_list(Task.await(lint, 60_000))
      assert {:ok, _} = Task.await(fmt, 60_000)
      send(tk, {:stop, self()})
      assert_receive {:max_gap, gap}, 5_000
      assert gap < 500, "通常のスケジューラが #{gap} ms 止まった"
    after
      :erlang.system_flag(:schedulers_online, online)
    end
  end

  # S214: Mutex を待つ NIF（simulate・check など）は dirty IO で回す。dirty CPU を 1 本に絞っても、実行中に check が
  # 待っていて、無関係の format（dirty CPU）が通る
  test "実行が Mutex を持つ間、check の待ちが dirty CPU を塞がない" do
    online = :erlang.system_info(:dirty_cpu_schedulers_online)
    :erlang.system_flag(:dirty_cpu_schedulers_online, 1)

    try do
      {:ok, ws} = GlobeMode.new()
      GlobeMode.set_file(ws, "Wiggle.mo", @wiggle)
      token = GlobeMode.cancel_token()
      me = self()
      settings = %{solver: "esdirk34", stop_time: 5000, interval: 1}
      run = Task.async(fn -> GlobeMode.simulate_observed(ws, "Wiggle", settings, token, me) end)
      assert_receive {:globe_progress, %{stage: "integration", time: t}} when is_float(t), 30_000

      waiting = Task.async(fn -> GlobeMode.check(ws, "Wiggle") end)
      Process.sleep(200)
      fmt = Task.async(fn -> GlobeMode.format("model A Real x; end A;\n") end)
      assert {:ok, _} = Task.await(fmt, 5_000)
      assert Task.yield(waiting, 0) == nil, "check が実行の終わりを待っていない（試験が成り立たない）"

      GlobeMode.cancel(token)
      assert {:ok, _} = Task.await(run, 30_000)
      assert {:ok, _} = Task.await(waiting, 30_000)
    after
      :erlang.system_flag(:dirty_cpu_schedulers_online, online)
    end
  end
end

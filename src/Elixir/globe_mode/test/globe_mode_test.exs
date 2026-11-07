defmodule GlobeModeTest do
  use ExUnit.Case

  @lib Path.expand("../../../Modelica/GlobeModeValidation", __DIR__)
  @model "GlobeModeValidation.Simulation.Decay"

  test "Decay を check → simulate して最後の時刻が 1" do
    {:ok, ws} = GlobeMode.new([@lib])
    assert {:ok, %{"passed" => true, "summary" => "GlobeModeValidation.Simulation.Decay: OK" <> _}} =
             GlobeMode.check(ws, @model)  # S184
    {:ok, res} = GlobeMode.simulate(ws, @model)
    assert res["ok"], inspect(res["diagnostics"])
    [last | _] = res["csv"] |> String.split("\n", trim: true) |> Enum.reverse()
    {t, _} = last |> String.split(",") |> hd() |> Float.parse()
    assert_in_delta t, 1.0, 1.0e-9
    # S111: 参照シナリオの x(1) と format
    assert_in_delta res["final"]["x"], :math.exp(-1), 1.0e-4
    {:ok, formatted} = GlobeMode.format(File.read!(Path.join(@lib, "Simulation/Decay.mo")))
    assert formatted =~ "der(x) = -k * x;"
    assert {:error, _} = GlobeMode.format("model Decay")
    assert is_list(GlobeMode.lint("model Decay Real x; end Decay;"))
    assert {:error, _} = GlobeMode.graph(ws, "NoSuchModel")
    # S116: 列の道。x(1) が e⁻¹、点の数が CSV の行数 - 1 と同じ
    {:ok, layout, [_time, xs]} = GlobeMode.simulate_columns(ws, @model)
    assert layout["names"] == ["time", "x"] and layout["response"]["csv"] == nil
    assert layout["defaults"] == ["x"]  # S190
    assert layout["points"] == length(String.split(res["csv"], "\n", trim: true)) - 1
    <<_::binary-size(byte_size(xs) - 8), x1::float-little>> = xs
    assert x1 == res["final"]["x"]
    {t_json, _} = :timer.tc(fn -> GlobeMode.simulate(ws, @model) end)
    {t_cols, _} = :timer.tc(fn -> GlobeMode.simulate_columns(ws, @model) end)
    IO.puts("columns: points=#{layout["points"]}, x(1)=#{x1}, json=#{t_json} us, columns=#{t_cols} us")
    # S118: FMU（2.0、build なし）。展開したディレクトリに modelDescription.xml がある
    out = Path.join(System.tmp_dir!(), "gm-elixir-fmu")
    {:ok, fmu} = GlobeMode.export_fmu(ws, @model, out)
    assert File.regular?(fmu)
    assert File.regular?(Path.join(out, "GlobeModeValidation_Simulation_Decay/modelDescription.xml"))
    assert {:error, _} = GlobeMode.export_fmu(ws, @model, out, %{fmi_version: 4})
    # モデル・設定の誤りは ok: false と diagnostics で返る
    assert {:ok, %{"ok" => false}} = GlobeMode.simulate(ws, @model, %{solver: "no-such-solver"})
  end

  test "読み取りの口 outline・deps・flat（S191）" do
    {:ok, ws} = GlobeMode.new()
    GlobeMode.set_file(ws, "A.mo", "model A Real x(start = 1, fixed = true); equation der(x) = -x; end A;")
    GlobeMode.set_file(ws, "Uses.mo", "model Uses A a; end Uses;")
    GlobeMode.set_file(ws, "Bad.mo", "model Bad Real x; equation y = 1; end Bad;")
    assert {:ok, %{"classes" => [%{"name" => "A", "kind" => "model"}]}} = GlobeMode.outline(ws, "A.mo")
    assert {:error, _} = GlobeMode.outline(ws, "Nope.mo")
    assert GlobeMode.deps(ws, "Uses") == {:ok, ["A.mo", "Uses.mo"]}
    assert GlobeMode.deps(ws, "NoSuchModel") == {:ok, []}
    assert {:ok, %{"variables" => 1, "equations" => 1}} = GlobeMode.flat(ws, "A")
    assert GlobeMode.flat(ws, "Bad") == {:ok, nil}
  end

  # S131（ADR-0033）: 積分に 100 ms より長くかかる（t が途中で知らされる）ように、細かく振れる入力を長く積む（S129 と同じ）
  @wiggle "model Wiggle Real x(start = 0, fixed = true); equation der(x) = sin(100 * time); end Wiggle;"

  test "Task で回し、最初の t の進捗で別のプロセスから cancel すると cancelled（S131）" do
    {:ok, ws} = GlobeMode.new()
    GlobeMode.set_file(ws, "Wiggle.mo", @wiggle)
    token = GlobeMode.cancel_token()
    me = self()
    settings = %{solver: "esdirk34", stop_time: 1000, interval: 1}
    task = Task.async(fn -> GlobeMode.simulate_observed(ws, "Wiggle", settings, token, me) end)
    assert_receive {:globe_progress, %{stage: "integration", time: t}} when is_float(t), 30_000
    assert GlobeMode.cancel(token) == :ok
    {:ok, res} = Task.await(task, 30_000)
    assert res["ok"] and res["termination"]["kind"] == "cancelled", inspect(res["termination"])
    assert res["termination"]["time"] > 0 and res["termination"]["time"] < 1000
    assert "GM-I11004" in Enum.map(res["diagnostics"], & &1["code"])
  end

  # S214: NIF が Mutex を持ったまま panic しても、長く生きる workspace は使い続けられる（poison から回復する）
  test "panic の後も同じ workspace が使える" do
    {:ok, ws} = GlobeMode.new()
    assert catch_error(:globe_mode_nif.panic_for_test(ws))
    assert {:ok, _} = GlobeMode.set_file(ws, "A.mo", "model A Real x; equation x = 1; end A;")
    assert {:ok, %{"passed" => true}} = GlobeMode.check(ws, "A")
  end
end

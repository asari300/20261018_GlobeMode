defmodule GlobeModeTest do
  use ExUnit.Case

  @lib Path.expand("../../../Modelica/GlobeModeValidation", __DIR__)
  @model "GlobeModeValidation.Simulation.Decay"

  test "Decay を check → simulate して最後の時刻が 1" do
    {:ok, ws} = GlobeMode.new([@lib])
    assert {:ok, %{"passed" => true}} = GlobeMode.check(ws, @model)
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
    assert layout["points"] == length(String.split(res["csv"], "\n", trim: true)) - 1
    <<_::binary-size(byte_size(xs) - 8), x1::float-little>> = xs
    assert x1 == res["final"]["x"]
    {t_json, _} = :timer.tc(fn -> GlobeMode.simulate(ws, @model) end)
    {t_cols, _} = :timer.tc(fn -> GlobeMode.simulate_columns(ws, @model) end)
    IO.puts("columns: points=#{layout["points"]}, x(1)=#{x1}, json=#{t_json} us, columns=#{t_cols} us")
    # モデル・設定の誤りは ok: false と diagnostics で返る
    assert {:ok, %{"ok" => false}} = GlobeMode.simulate(ws, @model, %{solver: "no-such-solver"})
  end
end

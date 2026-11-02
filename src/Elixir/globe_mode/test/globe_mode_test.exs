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
    assert {:error, _} = GlobeMode.graph(ws, "NoSuchModel")
    # モデル・設定の誤りは ok: false と diagnostics で返る
    assert {:ok, %{"ok" => false}} = GlobeMode.simulate(ws, @model, %{solver: "no-such-solver"})
  end
end

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
    # モデル・設定の誤りは ok: false と diagnostics で返る
    assert {:ok, %{"ok" => false}} = GlobeMode.simulate(ws, @model, %{solver: "no-such-solver"})
  end
end

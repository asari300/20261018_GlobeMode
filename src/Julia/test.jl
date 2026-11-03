# S075: GlobeModeValidation.Simulation.Decay を check → simulate → format し、JSON の長さと最後の時刻を表示する。
# julia test.jl <ライブラリのディレクトリ> <モデル>（mise run c:julia）
include("globe_mode.jl")
using Test

lib, model = ARGS
ws = GlobeMode.Workspace([lib])
checked = GlobeMode.check(ws, model)
@test occursin("\"passed\":true", checked)
json = GlobeMode.simulate(ws, model)
@test occursin("\"ok\":true", json)
# shortcut: JSON の標準ライブラリが無いので、CSV の最後の行（後ろが \n" の行）を正規表現で拾う。JSON.jl を入れるなら置き換える
t = parse(Float64, match(r"\\n([^,\\]+),[^\\]*\\n\"", json)[1])
println("simulate: json=$(length(json)) bytes, last time=$t")
@test t ≈ 1.0
# S111: 参照シナリオの x(1) と format（JSON は正規表現で拾う。上の shortcut と同じ）
x = parse(Float64, match(r"\"final\":\{[^}]*\"x\":([^,}]+)", json)[1])
@test x ≈ exp(-1) atol = 1e-4
formatted = GlobeMode.format(read(joinpath(lib, "Simulation", "Decay.mo"), String))
@test occursin("der(x) = -k * x;", formatted)
@test occursin("\"error\"", GlobeMode.format("model Decay"))
@test startswith(GlobeMode.lint("model Decay Real x; end Decay;"), "[")
@test occursin("\"nodes\"", GlobeMode.graph(ws, model))
@test GlobeMode.load_dir(GlobeMode.Workspace(), lib) > 0
println("x(1)=$x, format: $(length(formatted)) bytes")
# S116: 列の道。x(1) が e⁻¹、点の数が CSV の行数 - 1（JSON の中の \n の数 - 1）と同じ
layout, cols = GlobeMode.simulate_columns(ws, model)
@test occursin("\"names\":[\"time\",\"x\"]", layout)
@test length(cols[1]) == count("\\n", json) - 1
@test cols[2][end] == x
t_json = @elapsed GlobeMode.simulate(ws, model)
t_cols = @elapsed GlobeMode.simulate_columns(ws, model)
println("columns: points=$(length(cols[1])), x(1)=$(cols[2][end]), json=$(round(t_json * 1e3; digits=2)) ms, columns=$(round(t_cols * 1e3; digits=2)) ms")

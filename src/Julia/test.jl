# S075: GlobeModeValidation.Simulation.Decay を check → simulate し、JSON の長さと最後の時刻を表示する。
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

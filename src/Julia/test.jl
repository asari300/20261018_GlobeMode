# S075: GlobeModeValidation.Simulation.Decay を check → simulate → format し、JSON の長さと最後の時刻を表示する。
# julia test.jl <ライブラリのディレクトリ> <モデル>（mise run c:julia）
include("globe_mode.jl")
using Test

lib, model = ARGS
ws = GlobeMode.Workspace([lib])
checked = GlobeMode.check(ws, model)
@test occursin("\"passed\":true", checked)
@test occursin("\"summary\":\"", checked)  # S184
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
@test occursin("\"defaults\":[\"x\"]", layout)  # S190
t_json = @elapsed GlobeMode.simulate(ws, model)
t_cols = @elapsed GlobeMode.simulate_columns(ws, model)
println("columns: points=$(length(cols[1])), x(1)=$(cols[2][end]), json=$(round(t_json * 1e3; digits=2)) ms, columns=$(round(t_cols * 1e3; digits=2)) ms")
# S118: FMU（2.0、build なし）。展開したディレクトリに modelDescription.xml がある
out = mktempdir()
fmu = GlobeMode.export_fmu(ws, model, out)
@test occursin("\"path\"", fmu)
@test isfile(joinpath(out, replace(model, "." => "_"), "modelDescription.xml"))
@test occursin("\"error\"", GlobeMode.export_fmu(ws, model, out, "{\"fmi_version\": 4}"))
println("fmu: $fmu")
# S131（ADR-0033）: 進捗と中断。S129 と同じ、積分に 100 ms より長くかかるモデルを最初の t の進捗で止める
GlobeMode.set_file(ws, "Wiggle.mo",
    "model Wiggle Real x(start = 0, fixed = true); equation der(x) = sin(100 * time); end Wiggle;")
# S191: 読み取り
@test occursin("\"name\":\"Wiggle\"", GlobeMode.outline(ws, "Wiggle.mo"))
@test occursin("\"error\"", GlobeMode.outline(ws, "Nope.mo"))
@test GlobeMode.deps(ws, "Wiggle") == "[\"Wiggle.mo\"]"
@test occursin("\"variables\":1", GlobeMode.flat(ws, "Wiggle"))
@test GlobeMode.flat(ws, "NoSuch") == "null"
seen = []
res = GlobeMode.simulate_observed(ws, "Wiggle", p -> (push!(seen, p); p.time !== nothing),
    "{\"solver\": \"esdirk34\", \"stop_time\": 1000, \"interval\": 1}")
@test occursin("\"kind\":\"cancelled\"", res)
@test occursin("GM-I11004", res)
@test seen[1].stage == "lowering" && seen[end].time !== nothing
println("cancel: progress=$(length(seen)), stages=$(unique(p.stage for p in seen))")
# S202: コールバックの中から同じ ws を使うと誤り（使用中）が返り、gm_workspace_free は何もしない。プロセスは落ちず、ws はその後も使える
errs = String[]
GlobeMode.simulate_observed(ws, "Wiggle", p -> begin
        if isempty(errs)
            push!(errs, GlobeMode.check(ws, "Wiggle"), GlobeMode.simulate(ws, "Wiggle"))
            ccall((:gm_workspace_free, GlobeMode.LIB), Cvoid, (Ptr{Cvoid},), ws.ptr)
        end
        true
    end, "{\"solver\": \"esdirk34\", \"stop_time\": 1000, \"interval\": 1}")
@test length(errs) == 2 && all(e -> occursin("\"error\"", e) && occursin("使用中", e), errs)
@test occursin("\"passed\"", GlobeMode.check(ws, "Wiggle"))
println("reentry: $(errs[1])")

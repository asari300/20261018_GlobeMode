# S075: globe-mode-c の C ABI（src/Rust/mod/globe-mode-c/include/globe_mode.h）を ccall だけで呼ぶ。
# パッケージにはしない。`include("globe_mode.jl")` して使う。結果は JSON の文字列のまま返す。
module GlobeMode

# 共有ライブラリのパス。環境変数 GLOBE_MODE_LIB（mise run c:julia が渡す）か、既定の target/release。
const LIB = get(ENV, "GLOBE_MODE_LIB",
    joinpath(@__DIR__, "..", "..", "target", "release", "libglobe_mode_c.so"))

mutable struct Workspace
    ptr::Ptr{Cvoid}
    function Workspace(libraries::Vector{String}=String[])
        json = "[" * join(("\"" * escape_string(l) * "\"" for l in libraries), ",") * "]"
        p = ccall((:gm_workspace_new, LIB), Ptr{Cvoid}, (Cstring,), json)
        p == C_NULL && error("gm_workspace_new が失敗した（理由は stderr）")
        finalizer(w -> ccall((:gm_workspace_free, LIB), Cvoid, (Ptr{Cvoid},), w.ptr), new(p))
    end
end

# ライブラリが返した char* を String に写して放す。
function take(p::Ptr{UInt8})
    p == C_NULL && error("NULL が返った")
    s = unsafe_string(p)
    ccall((:gm_string_free, LIB), Cvoid, (Ptr{UInt8},), p)
    s
end

set_file(ws::Workspace, path, text) =
    ccall((:gm_set_file, LIB), Cint, (Ptr{Cvoid}, Cstring, Cstring), ws.ptr, path, text) == 0 ||
    error("gm_set_file が失敗した")

# 読んだ .mo の数（S111）。
load_dir(ws::Workspace, dir) =
    (n = ccall((:gm_load_dir, LIB), Int64, (Ptr{Cvoid}, Cstring), ws.ptr, dir)) >= 0 ? n :
    error("gm_load_dir が失敗した")

check(ws::Workspace, model) =
    take(ccall((:gm_check, LIB), Ptr{UInt8}, (Ptr{Cvoid}, Cstring), ws.ptr, model))

simulate(ws::Workspace, model, settings=C_NULL) =
    take(ccall((:gm_simulate, LIB), Ptr{UInt8}, (Ptr{Cvoid}, Cstring, Cstring), ws.ptr, model, settings))

# S116: 列の道。(レイアウトの JSON, 列ごとの Vector{Float64}) を返す。列は Julia の配列に写してから放す。
function simulate_columns(ws::Workspace, model, settings=C_NULL)
    data, len = Ref{Ptr{Float64}}(C_NULL), Ref{Csize_t}(0)
    layout = take(ccall((:gm_simulate_columns, LIB), Ptr{UInt8},
        (Ptr{Cvoid}, Cstring, Cstring, Ref{Ptr{Float64}}, Ref{Csize_t}), ws.ptr, model, settings, data, len))
    flat = copy(unsafe_wrap(Vector{Float64}, data[], len[]))
    ccall((:gm_doubles_free, LIB), Cvoid, (Ptr{Float64}, Csize_t), data[], len[])
    # shortcut: JSON の標準ライブラリが無いので points を正規表現で拾う（test.jl と同じ）
    m = match(r"\"points\":(\d+)", layout)
    points = m === nothing ? 0 : parse(Int, m[1])
    layout, points == 0 ? Vector{Float64}[] : [flat[i:i+points-1] for i in 1:points:length(flat)]
end

# S118: FMU の書き出し。{"path": ...} か {"error": ...} の JSON。options は {"fmi_version": 2, "build": false, ...}（省略可）。
export_fmu(ws::Workspace, model, out_dir, options=C_NULL) =
    take(ccall((:gm_export_fmu, LIB), Ptr{UInt8}, (Ptr{Cvoid}, Cstring, Cstring, Cstring), ws.ptr, model, out_dir, options))

graph(ws::Workspace, model, depth=1) =
    take(ccall((:gm_graph, LIB), Ptr{UInt8}, (Ptr{Cvoid}, Cstring, UInt32), ws.ptr, model, depth))

# S111: 整形（{"text": ...} か {"error": ...} の JSON）と lint（JSON の配列）。Workspace は要らない。
format(text; indent=2, insert_origin=true) =
    take(ccall((:gm_format, LIB), Ptr{UInt8}, (Cstring, UInt32, Cint), text, indent, insert_origin))

lint(text) = take(ccall((:gm_lint, LIB), Ptr{UInt8}, (Cstring,), text))

end

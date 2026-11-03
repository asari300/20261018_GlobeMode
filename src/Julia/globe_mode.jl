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

graph(ws::Workspace, model, depth=1) =
    take(ccall((:gm_graph, LIB), Ptr{UInt8}, (Ptr{Cvoid}, Cstring, UInt32), ws.ptr, model, depth))

# S111: 整形（{"text": ...} か {"error": ...} の JSON）と lint（JSON の配列）。Workspace は要らない。
format(text; indent=2, insert_origin=true) =
    take(ccall((:gm_format, LIB), Ptr{UInt8}, (Cstring, UInt32, Cint), text, indent, insert_origin))

lint(text) = take(ccall((:gm_lint, LIB), Ptr{UInt8}, (Cstring,), text))

end

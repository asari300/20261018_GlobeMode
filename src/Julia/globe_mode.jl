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

check(ws::Workspace, model) =
    take(ccall((:gm_check, LIB), Ptr{UInt8}, (Ptr{Cvoid}, Cstring), ws.ptr, model))

simulate(ws::Workspace, model, settings=C_NULL) =
    take(ccall((:gm_simulate, LIB), Ptr{UInt8}, (Ptr{Cvoid}, Cstring, Cstring), ws.ptr, model, settings))

end

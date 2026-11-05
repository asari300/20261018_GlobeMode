# src/Zig

Zig のラッパ（P7、S075）。`globe-mode-c` の手書きのヘッダ `src/Rust/mod/globe-mode-c/include/globe_mode.h` を `b.addTranslateC` で読み、共有ライブラリ `libglobe_mode_c.so` に link する。ロジックは Rust 側に置く。

English: a Zig example that links the GlobeMode C ABI (`globe-mode-c`) and runs `check` then `simulate` on `GlobeModeValidation.Simulation.Decay`.

```bash
mise run c:zig   # cargo build --release -p globe-mode-c → zig build run
```

最初の例（自分の `main.zig` に置くと、`src/Zig` で `zig build run -Dlib-dir=<target/release>` で回る。`src/main.zig` を縮めたもの。`mise run c:zig` は元の `main.zig` を回す）:

```zig
const std = @import("std");
const gm = @import("gm");

pub fn main(init: std.process.Init) !void {
    const arena = init.arena.allocator();
    const ws = gm.gm_workspace_new("[\"../Modelica/GlobeModeValidation\"]") orelse return error.WorkspaceNew;
    defer gm.gm_workspace_free(ws);
    const model = "GlobeModeValidation.Simulation.Decay";
    const check = gm.gm_check(ws, model); // JSON の文字列。必ず gm_string_free で放す
    defer gm.gm_string_free(check);
    const checked = try std.json.parseFromSliceLeaky(std.json.Value, arena, std.mem.span(@as([*:0]const u8, @ptrCast(check))), .{});
    std.debug.print("passed={}\n", .{checked.object.get("passed").?.bool});
    const sim = gm.gm_simulate(ws, model, null); // settings は NULL か {"stop_time": 2} など
    defer gm.gm_string_free(sim);
    const res = try std.json.parseFromSliceLeaky(std.json.Value, arena, std.mem.span(@as([*:0]const u8, @ptrCast(sim))), .{});
    std.debug.print("x(1)={d}\n", .{res.object.get("final").?.object.get("x").?.float}); // 0.3678…（e⁻¹）
}
```

- `build.zig`: `-Dlib-dir=<target/release>`（既定 `../../target/release`）。rpath にも同じディレクトリを入れる。
- `src/main.zig`: 引数は `<ライブラリのディレクトリ> <モデル>`。`check` の `passed` と、`simulate` の JSON の長さと CSV の最後の時刻を表示する。
- Zig 0.17（mise の `zig = "latest"`）で確かめた。std の API は版ごとに変わる。

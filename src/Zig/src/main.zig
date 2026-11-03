//! S075: C ABI（globe_mode.h）で GlobeModeValidation.Simulation.Decay を check → simulate → format し（S111 で format、S116 で列の道、S118 で FMU、S131 で進捗と中断）、
//! 結果の JSON の長さと最後の時刻、x(1) を表示する。引数: <ライブラリのディレクトリ> <モデル>
const std = @import("std");
const gm = @import("gm");

/// ライブラリが返した文字列を Zig の slice にする（放すのは呼び手が gm_string_free で）。
fn text(p: [*c]u8) []const u8 {
    return std.mem.span(@as([*:0]const u8, @ptrCast(p)));
}

/// S131: 最初の t（積分の時刻）の進捗で 1 を返して止める。`user` は呼ばれた回数。
fn stopAtFirstTime(p: [*c]const gm.GmProgress, user: ?*anyopaque) callconv(.c) c_int {
    const calls: *u32 = @ptrCast(@alignCast(user.?));
    calls.* += 1;
    return @intFromBool(p.*.has_time != 0);
}

pub fn main(init: std.process.Init) !void {
    const arena = init.arena.allocator();
    const args = try init.minimal.args.toSlice(arena);
    if (args.len != 3) return error.Usage;

    const libs = try std.fmt.allocPrintSentinel(arena, "[\"{s}\"]", .{args[1]}, 0);
    const model = args[2];
    const ws = gm.gm_workspace_new(libs.ptr) orelse return error.WorkspaceNew;
    defer gm.gm_workspace_free(ws);

    const check = gm.gm_check(ws, model.ptr);
    defer gm.gm_string_free(check);
    const checked = try std.json.parseFromSliceLeaky(std.json.Value, arena, text(check), .{});
    const passed = checked.object.get("passed").?.bool;
    std.debug.print("check: passed={}\n", .{passed});
    if (!passed) return error.CheckFailed;

    const sim = gm.gm_simulate(ws, model.ptr, null);
    defer gm.gm_string_free(sim);
    const json = text(sim);
    const res = try std.json.parseFromSliceLeaky(std.json.Value, arena, json, .{});
    if (!res.object.get("ok").?.bool) {
        std.debug.print("{s}\n", .{json});
        return error.SimulateFailed;
    }
    // CSV の最後の行の最初の列が最後の時刻
    const csv = std.mem.trimEnd(u8, res.object.get("csv").?.string, "\n");
    const last = csv[(std.mem.lastIndexOfScalar(u8, csv, '\n') orelse 0) + 1 ..];
    const t = try std.fmt.parseFloat(f64, last[0..std.mem.indexOfScalar(u8, last, ',').?]);
    const x = res.object.get("final").?.object.get("x").?.float;
    std.debug.print("simulate: json={d} bytes, last time={d}, x={d}\n", .{ json.len, t, x });
    if (@abs(x - @exp(-1.0)) > 1e-4) return error.WrongValue;

    // S116: 列の道。点の数が CSV の行数 - 1（＝改行の数）と同じで、最後の値が x(1)
    var data: [*c]f64 = null;
    var len: usize = 0;
    const cols = gm.gm_simulate_columns(ws, model.ptr, null, &data, &len);
    defer gm.gm_string_free(cols);
    defer gm.gm_doubles_free(data, len);
    const layout = try std.json.parseFromSliceLeaky(std.json.Value, arena, text(cols), .{});
    const points: usize = @intCast((layout.object.get("points") orelse {
        std.debug.print("{s}\n", .{text(cols)});
        return error.ColumnsFailed;
    }).integer);
    if (points != std.mem.count(u8, csv, "\n") or len != 2 * points) return error.ColumnsFailed;
    std.debug.print("columns: points={d}, x={d}\n", .{ points, data[len - 1] });
    if (data[len - 1] != x) return error.WrongValue;

    // S131（ADR-0033）: 進捗と中断。S129 と同じ、積分に 100 ms より長くかかるモデルを最初の t の進捗で止める
    const wiggle = "model Wiggle Real x(start = 0, fixed = true); equation der(x) = sin(100 * time); end Wiggle;";
    if (gm.gm_set_file(ws, "Wiggle.mo", wiggle) != 0) return error.SetFile;
    var calls: u32 = 0;
    const obs = gm.gm_simulate_observed(ws, "Wiggle", "{\"solver\": \"esdirk34\", \"stop_time\": 1000, \"interval\": 1}", stopAtFirstTime, &calls);
    defer gm.gm_string_free(obs);
    const observed = try std.json.parseFromSliceLeaky(std.json.Value, arena, text(obs), .{});
    const term = observed.object.get("termination") orelse {
        std.debug.print("{s}\n", .{text(obs)});
        return error.CancelFailed;
    };
    const kind = term.object.get("kind").?.string;
    std.debug.print("cancel: calls={d}, termination={s}\n", .{ calls, kind });
    if (!std.mem.eql(u8, kind, "cancelled") or std.mem.indexOf(u8, text(obs), "GM-I11004") == null) return error.CancelFailed;

    // S111: 参照シナリオの最後（format）
    const path = try std.fmt.allocPrint(arena, "{s}/Simulation/Decay.mo", .{args[1]});
    const src = try std.Io.Dir.cwd().readFileAllocOptions(init.io, path, arena, .unlimited, .of(u8), 0);
    const fmt = gm.gm_format(src.ptr, 2, 1);
    defer gm.gm_string_free(fmt);
    const formatted = try std.json.parseFromSliceLeaky(std.json.Value, arena, text(fmt), .{});
    const body = (formatted.object.get("text") orelse {
        std.debug.print("{s}\n", .{text(fmt)});
        return error.FormatFailed;
    }).string;
    if (std.mem.indexOf(u8, body, "der(x) = -k * x;") == null) return error.FormatFailed;
    std.debug.print("format: {d} bytes\n", .{body.len});

    // S118: FMU（2.0、build なし）。展開したディレクトリに modelDescription.xml がある
    const out = "zig-out/gm-zig-fmu";
    const fmu = gm.gm_export_fmu(ws, model.ptr, out, null);
    defer gm.gm_string_free(fmu);
    const exported = try std.json.parseFromSliceLeaky(std.json.Value, arena, text(fmu), .{});
    const fmu_path = (exported.object.get("path") orelse {
        std.debug.print("{s}\n", .{text(fmu)});
        return error.FmuFailed;
    }).string;
    const name = try std.mem.replaceOwned(u8, arena, model, ".", "_");
    const md_path = try std.fmt.allocPrint(arena, "{s}/{s}/modelDescription.xml", .{ out, name });
    const md = try std.Io.Dir.cwd().readFileAllocOptions(init.io, md_path, arena, .unlimited, .of(u8), 0);
    if (std.mem.indexOf(u8, md, "fmiModelDescription") == null) return error.FmuFailed;
    std.debug.print("fmu: {s}, modelDescription.xml={d} bytes\n", .{ fmu_path, md.len });
}

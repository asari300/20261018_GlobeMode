//! S075: globe-mode-c の共有ライブラリ（libglobe_mode_c.so）に link して例を回す。
//! `zig build run -Dlib-dir=<cargo の target/release> -- <src/Modelica> <モデル>`（mise run c:zig）。
const std = @import("std");

pub fn build(b: *std.Build) void {
    const target = b.standardTargetOptions(.{});
    const optimize = b.standardOptimizeOption(.{});
    const lib_dir = b.option([]const u8, "lib-dir", "libglobe_mode_c.so のあるディレクトリ") orelse "../../target/release";

    const header = b.addTranslateC(.{
        .root_source_file = b.path("../Rust/mod/globe-mode-c/include/globe_mode.h"),
        .target = target,
        .optimize = optimize,
    });
    const exe = b.addExecutable(.{
        .name = "globe-mode-zig",
        .root_module = b.createModule(.{
            .root_source_file = b.path("src/main.zig"),
            .target = target,
            .optimize = optimize,
            .link_libc = true,
            .imports = &.{.{ .name = "gm", .module = header.createModule() }},
        }),
    });
    exe.root_module.addLibraryPath(.{ .cwd_relative = lib_dir });
    exe.root_module.addRPath(.{ .cwd_relative = lib_dir });
    exe.root_module.linkSystemLibrary("globe_mode_c", .{});
    b.installArtifact(exe);

    const run = b.addRunArtifact(exe);
    run.addPassthruArgs();
    b.step("run", "例を回す").dependOn(&run.step);
}

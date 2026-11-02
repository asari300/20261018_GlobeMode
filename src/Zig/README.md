# src/Zig

Zig のラッパ（P7、S075）。`globe-mode-c` の手書きのヘッダ `src/Rust/mod/globe-mode-c/include/globe_mode.h` を `b.addTranslateC` で読み、共有ライブラリ `libglobe_mode_c.so` に link する。ロジックは Rust 側に置く。

English: a Zig example that links the GlobeMode C ABI (`globe-mode-c`) and runs `check` then `simulate` on `GlobeModeValidation.Simulation.Decay`.

```bash
mise run c:zig   # cargo build --release -p globe-mode-c → zig build run
```

- `build.zig`: `-Dlib-dir=<target/release>`（既定 `../../target/release`）。rpath にも同じディレクトリを入れる。
- `src/main.zig`: 引数は `<ライブラリのディレクトリ> <モデル>`。`check` の `passed` と、`simulate` の JSON の長さと CSV の最後の時刻を表示する。
- Zig 0.17（mise の `zig = "latest"`）で確かめた。std の API は版ごとに変わる。

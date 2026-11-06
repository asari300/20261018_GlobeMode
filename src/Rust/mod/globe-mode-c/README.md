# globe-mode-c

GlobeMode の C ABI（P7、S075）。facade `globe-mode` の `Workspace` を C の関数で包む。入出力は JSON の文字列で、ロジックは Rust 側に置く。関数の一覧と約束はヘッダ [`include/globe_mode.h`](include/globe_mode.h) にある（手書き。`tests/header.rs` が `src/lib.rs` の署名と突き合わせる）。

`gm_outline(ws, path)`・`gm_deps(ws, model)`・`gm_flat(ws, model)` は読み取り（S191。`globe outline` / `deps` / `flat --json` と同じ JSON）。`gm_simulate_columns` の JSON の `defaults` は既定で描く信号の名前（S190）。

English: the C ABI of GlobeMode (`libglobe_mode_c.so`); `include/globe_mode.h` lists the functions. Strings returned as `char*` are JSON and must be freed with `gm_string_free`. Zig and Julia call this ABI ([`src/Zig`](../../../Zig/README.md), [`src/Julia`](../../../Julia/README.md)).

最初の例（リポジトリの根から）:

```bash
cargo build --release -p globe-mode-c   # target/release/libglobe_mode_c.so
cc first.c -Isrc/Rust/mod/globe-mode-c/include -Ltarget/release -lglobe_mode_c -Wl,-rpath,$PWD/target/release -o first && ./first
```

```c
#include <stdio.h>
#include <string.h>
#include "globe_mode.h"

int main(void) {
    GmWorkspace *ws = gm_workspace_new("[\"src/Modelica/GlobeModeValidation\"]");
    if (!ws) return 1;
    const char *model = "GlobeModeValidation.Simulation.Decay";
    char *checked = gm_check(ws, model);   /* JSON。必ず gm_string_free で放す */
    printf("passed: %s\n", strstr(checked, "\"passed\":true") ? "true" : "false");
    gm_string_free(checked);
    char *res = gm_simulate(ws, model, NULL);   /* settings は NULL か {"stop_time": 2} など */
    const char *x = strstr(res, "\"final\":{\"x\":");   /* 最後の時刻の値。JSON の解析はここでは省く */
    printf("x(1) = %.12s\n", x ? x + strlen("\"final\":{\"x\":") : "?");   /* 0.3678794681 */
    gm_string_free(res);
    gm_workspace_free(ws);
    return 0;
}
```

## スレッドとコールバックの決まり（S202）

- `GmWorkspace` は作ったスレッドからだけ使う。同時に 2 つのスレッドから使わない（salsa のデータベースが Sync でない）。
- 進捗のコールバックは、実行を回しているスレッドで、実行の途中に呼ばれる。その間 `GmWorkspace` は「使用中」で、**コールバックの中から同じ `ws` を使う `gm_*` を呼ぶと `{"error": "ワークスペースは使用中…"}` が返る**（`gm_set_file`・`gm_load_dir` は -1）。`gm_workspace_free(ws)` は何もせず stderr に 1 行出す（放すのは実行が返ったあと）。別の `ws` は使える。
- 仕組み: `GmWorkspace` が使用中の印（`AtomicBool`）を持ち、`ws_arg` が生のポインタから印だけを読んでから `&mut Workspace` を貸す。印を見る前に `&mut GmWorkspace` も `&mut Workspace` も作らないので、実行中の `&mut` と別名にならない。
- コールバックから `longjmp`・C++ の例外で抜けない（Rust のフレームを越える）。止めたいときは非 0 を返す。
- `GmProgress` の並び（64 ビットで 40 バイト）は `tests/header.rs` がヘッダの `typedef` と Rust の `repr(C)` で突き合わせる。
- 【Miri】この環境に `cargo miri` が無いので回していない（`rustup` に nightly を入れていない）。

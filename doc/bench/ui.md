# UI の比較（S125〜）

<!-- src/Svelte/tests/bench_ui.py report が target/ui-bench.json から作る。手で直さない -->

測り方は [ADR-0034](../adr/0034-ui-bench-method.md)。数は回の中央値（バンドルは 1 回）。「未」はまだ測っていない（UI が無い・OS が matrix に無い）。

| 測るもの | Svelte ブラウザ Linux | Svelte ブラウザ Windows | Svelte ブラウザ macOS | Svelte Tauri Linux | Svelte Tauri Windows | Svelte Tauri macOS | Leptos Linux | Leptos Windows | Leptos macOS | Dioxus Linux | Dioxus Windows | Dioxus macOS |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 起動から操作可能まで（ms） | 1,232 | 未 | 未 | 114 | 178 | 未 | 未 | 未 | 未 | 未 | 未 | 未 |
| Decay の検査・実行・プロット（ms） | 258 | 未 | 未 | 54 | 43 | 未 | 未 | 未 | 未 | 未 | 未 | 未 |
| MSL を読み ChuaCircuit を検査（ms） | 2,800 | 未 | 未 | 1,045 | 1,927 | 未 | 未 | 未 | 未 | 未 | 未 | 未 |
| 最大の RSS（MiB） | 345.5 | 未 | 未 | 151.6 | 128.7 | 未 | 未 | 未 | 未 | 未 | 未 | 未 |
| バンドル（MiB） | 15.6 | 未 | 未 | 22.7 | 20.6 | 未 | 未 | 未 | 未 | 未 | 未 | 未 |

## 出典

- svelte-browser/linux: 手元（WSL2、load 9.05）（3 回）
- svelte-tauri/linux: desktop.yml run 41622934035（ubuntu-24.04、xvfb）（3 回）
- svelte-tauri/windows: desktop.yml run 41622934035（windows-2025）（3 回）

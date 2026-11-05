# UI の比較（S125〜）

<!-- src/Svelte/tests/bench_ui.py report が target/ui-bench.json から作る。手で直さない -->

測り方は [ADR-0034](../adr/0034-ui-bench-method.md)。数は回の中央値（バンドルは 1 回）。「未」はまだ測っていない（UI が無い・OS が matrix に無い）。

| 測るもの | Svelte ブラウザ Linux | Svelte ブラウザ Windows | Svelte ブラウザ macOS | Svelte Tauri Linux | Svelte Tauri Windows | Svelte Tauri macOS | Leptos Linux | Leptos Windows | Leptos macOS | Dioxus Linux | Dioxus Windows | Dioxus macOS |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 起動から操作可能まで（ms） | 1,232 | 未 | 未 | 114 | 178 | 未 | 154 | 未 | 未 | 141 | 未 | 未 |
| Decay の検査・実行・プロット（ms） | 258 | 未 | 未 | 54 | 43 | 未 | 195 | 未 | 未 | 172 | 未 | 未 |
| MSL を読み ChuaCircuit を検査（ms） | 2,800 | 未 | 未 | 1,045 | 1,927 | 未 | 1,924 | 未 | 未 | 1,457 | 未 | 未 |
| 最大の RSS（MiB） | 345.5 | 未 | 未 | 151.6 | 128.7 | 未 | 323.4 | 未 | 未 | 324.2 | 未 | 未 |
| バンドル（MiB） | 15.6 | 未 | 未 | 22.7 | 20.6 | 未 | 12.5 | 未 | 未 | 12.7 | 未 | 未 |

## 出典

- svelte-browser/linux: 手元（WSL2、load 9.05）（3 回）
- svelte-tauri/linux: desktop.yml run 41622934035（ubuntu-24.04、xvfb）（3 回）
- svelte-tauri/windows: desktop.yml run 41622934035（windows-2025）（3 回）
- leptos/linux: 手元（WSL2、load 約 2、S153）。同じ条件で測り直した Svelte ブラウザは 324 / 196 / 2,064 ms・RSS 583.5 MiB・バンドル 18.7 MiB（上の Svelte の列は S125 の load 9.05 の値）。Svelte 版は CodeMirror・プロット・graph・Worker を持ち、同じ仕事ではない（3 回）
- dioxus/linux: 手元（WSL2、load 約 0.7、S155）。同じ条件で測り直した Svelte ブラウザは 289 / 176 / 1,868 ms・RSS 582.2 MiB・バンドル 18.7 MiB、Leptos は 128 / 142 / 1,500 ms・RSS 323.3 MiB・バンドル 12.5 MiB（上の列は S125 と S153 の値）。Svelte 版は CodeMirror・プロット・graph・Worker を持ち、同じ仕事ではない（3 回）

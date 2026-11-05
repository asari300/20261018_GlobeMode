# UI の比較（S125〜）

<!-- src/Svelte/tests/bench_ui.py report が target/ui-bench.json から作る。手で直さない -->

測り方は [ADR-0034](../adr/0034-ui-bench-method.md)。数は回の中央値（バンドルは 1 回）。「未」はまだ測っていない欄（今は無い。Svelte Tauri の macOS は S160 で `desktop.yml` に足した）。Linux は手元の WSL2 と、GitHub Actions（`ui-bench.yml`、S156）の 2 列。

| 測るもの | Svelte ブラウザ Linux | Svelte ブラウザ Linux（Actions） | Svelte ブラウザ Windows | Svelte ブラウザ macOS | Svelte Tauri Linux | Svelte Tauri Windows | Svelte Tauri macOS | Leptos Linux | Leptos Linux（Actions） | Leptos Windows | Leptos macOS | Dioxus Linux | Dioxus Linux（Actions） | Dioxus Windows | Dioxus macOS |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 起動から操作可能まで（ms） | 1,232 | 304 | 192 | 450 | 114 | 178 | 190 | 154 | 150 | 70 | 234 | 141 | 146 | 94 | 251 |
| Decay の検査・実行・プロット（ms） | 258 | 174 | 156 | 227 | 54 | 43 | 256 | 195 | 193 | 126 | 250 | 172 | 193 | 158 | 233 |
| MSL を読み ChuaCircuit を検査（ms） | 2,800 | 2,061 | 1,352 | 2,106 | 1,045 | 1,927 | 2,200 | 1,924 | 1,632 | 980 | 2,007 | 1,457 | 1,565 | 934 | 1,650 |
| 最大の RSS（MiB） | 345.5 | 583.4 | 576.4 | 611.6 | 151.6 | 128.7 | 164.4 | 323.4 | 320.0 | 294.6 | 305.8 | 324.2 | 325.6 | 297.4 | 305.5 |
| バンドル（MiB） | 15.6 | 18.7 | 18.7 | 18.6 | 22.7 | 20.6 | 20.9 | 12.5 | 12.5 | 12.5 | 12.5 | 12.7 | 12.7 | 12.7 | 12.7 |

## 出典

- svelte-browser/linux: 手元（WSL2、load 9.05）（3 回）
- svelte-browser/linux-actions: ui-bench.yml run 41984237319 (ubuntu-latest)（3 回）
- svelte-browser/windows: ui-bench.yml run 41984237319 (windows-latest)（3 回）
- svelte-browser/macos: ui-bench.yml run 41984237319 (macos-latest)（3 回）
- svelte-tauri/linux: desktop.yml run 41622934035（ubuntu-24.04、xvfb）（3 回）
- svelte-tauri/windows: desktop.yml run 41622934035（windows-2025）（3 回）
- svelte-tauri/macos: desktop.yml run 41994245407（macos-latest＝macos-26-arm64、未署名の .app を直に起動）（3 回）
- leptos/linux: 手元（WSL2、load 約 2、S153）。同じ条件で測り直した Svelte ブラウザは 324 / 196 / 2,064 ms・RSS 583.5 MiB・バンドル 18.7 MiB（上の Svelte の列は S125 の load 9.05 の値）。Svelte 版は CodeMirror・プロット・graph・Worker を持ち、同じ仕事ではない（3 回）
- leptos/linux-actions: ui-bench.yml run 41984237319 (ubuntu-latest)（3 回）
- leptos/windows: ui-bench.yml run 41984237319 (windows-latest)（3 回）
- leptos/macos: ui-bench.yml run 41984237319 (macos-latest)（3 回）
- dioxus/linux: 手元（WSL2、load 約 0.7、S155）。同じ条件で測り直した Svelte ブラウザは 289 / 176 / 1,868 ms・RSS 582.2 MiB・バンドル 18.7 MiB、Leptos は 128 / 142 / 1,500 ms・RSS 323.3 MiB・バンドル 12.5 MiB（上の列は S125 と S153 の値）。Svelte 版は CodeMirror・プロット・graph・Worker を持ち、同じ仕事ではない（3 回）
- dioxus/linux-actions: ui-bench.yml run 41984237319 (ubuntu-latest)（3 回）
- dioxus/windows: ui-bench.yml run 41984237319 (windows-latest)（3 回）
- dioxus/macos: ui-bench.yml run 41984237319 (macos-latest)（3 回）

## ランナー（ui-bench.yml）

GitHub-hosted runner の標準（公開リポジトリ）。出典: <https://docs.github.com/en/actions/reference/runners/github-hosted-runners>（確認日 2026-11-05）。イメージは run 41984237319 のログの `Image:`。

| ラベル | 実際のイメージ | CPU | RAM | ディスク | アーキテクチャ |
| --- | --- | ---: | ---: | ---: | --- |
| `ubuntu-latest` | ubuntu-24.04（20261025.353） | 4 | 16 GB | 14 GB | x64 |
| `windows-latest` | windows-2025-vs2026（20261030.294） | 4 | 16 GB | 14 GB | x64 |
| `macos-latest` | macos-26-arm64（20261026.0455） | 3（M1） | 7 GB | 14 GB | arm64 |

- **ランナーは共有で、値は目安。** 同じ仕様でも回ごとに揺れ、しきい値は置かない。OS 間の差には、CPU の種類（macOS は arm64、他は x64）と OS ごとの Chromium の違いが混ざる。
- 手元の WSL2 の列と Actions の Linux の列は、条件が違う（手元は load と RAM 3 GB の WSL2、Actions は 4 vCPU・16 GB）。
- RSS: Linux は Chromium の子孫の `VmHWM`、Windows は `peak_wset`（ピーク）、macOS は終了時点の rss（ピークが取れず、過小になりうる）。Svelte Tauri の列は Desktop の webview の木で、ブラウザ版とは測るものが違う（Linux は `time -v` の木の中の最大、Windows はプロセスごとの `PeakWorkingSet` の最大、macOS は `ps` を 0.2 s ごとに見たアプリ・sidecar・WebKit のプロセスの rss の最大で、サンプリングの間のピークは落ちうる）。macOS の Desktop は署名・公証なし（ad-hoc の linker 署名だけ）。
- 「Svelte ブラウザ」は CodeMirror・プロット・graph・Worker を持ち、Leptos・Dioxus（textarea と表だけ）と同じ仕事ではない。

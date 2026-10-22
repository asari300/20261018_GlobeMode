# ADR-0019 Raspberry Pi（aarch64）を開発と MSL の全体実行の機械にする

- 状態: Accepted（2026-10-22、S017）
- 関係: [ADR-0003](0003-toolchain.md)（ツールの管理）、[ADR-0006](0006-incremental-ci.md)（CI は GitHub Actions の x86_64 のまま）。製品のコードは変えない

## 背景

- 2026-10-22 から Claude の実行場所を x86_64 の機械から Raspberry Pi（aarch64、Linux 6.12 `rpt-rpi-2712`、4 コア、RAM 7.9 GiB、glibc 2.36（Debian 12）、ディスクの空き 66 GB）に移した（ユーザーの指示、[roadmap.md](../roadmap.md) の「S017 の最初にやること」）。
- `cargo xtask fetch-rumoca` は Linux x86_64 でないと断っていた。Rumoca v0.10.2 の配布物には `rumoca-linux-aarch64`（静的リンクの ELF、sha256 `c12704c0…`）がある（https://github.com/CogniPilot/rumoca/releases/tag/v0.10.2 、2026-10-22 確認）。
- `mise.lock` のツールは linux-arm64 の項を持っていたが、erlang 27.3.4.19 の arm64 ビルド（hex.pm の ubuntu-26.04 向け）は `GLIBC_2.38`（libm は 2.43）を要求し、RPi の glibc 2.36 では `beam.smp` が動かない。そのため erlang に依存する elixir 1.18.3-otp-27 のインストールが失敗する。rust・cargo-deny・wasm-bindgen・zig・julia・gleam・uv は入った。
- `msl:funnel` は 1 プロセスで 566 本を検査し、x86_64 では最大の常駐メモリ（VmHWM）が 7.1〜7.2 GiB だった（50 本ごとに Session を戻しても）。RPi は RAM 7.9 GiB のうちデスクトップ（orca-ide・chromium・ghostty）で約 2.7 GiB を使っていて、swap は dphys-swapfile の 512 MB だった。そのままでは OOM で落ちる見込みだった。

## 決定

1. **`fetch-rumoca` は `target_arch` で配布物を選ぶ。** `RUMOCA_ASSET: Option<(ファイル名, sha256)>` を `cfg!` の const で持ち、Linux の x86_64 と aarch64 に対応する。URL は `RUMOCA_VERSION` と組み立てる。`msl-compare` の `--rumoca` の既定（`RUMOCA_DEFAULT_PATH`）も同じ名前にする。対応していない機械（Windows の CI など）では `None` にしてビルドは通し、実行時に断る。
2. **メモリは swap で補う。** `/etc/dphys-swapfile` の `CONF_SWAPSIZE` と `CONF_MAXSWAP` を 8192 にし、`/var/swap` を 8 GB で作り直す（再起動後も残る。元の設定は `/etc/dphys-swapfile.bak-s017`）。`msl:funnel`・`msl:sim` は RPi でも全体を回す。ユーザーの選択（2026-10-22）。

比べた案と、採らなかった理由は次の通り。

| 案 | 採らなかった理由 |
| --- | --- |
| A. 先に重いパッケージだけ `--filter` で実測し、足りないときだけ swap | 実測にも落ちる危険があり、結局 swap が要る見込みだった |
| B. xtask にパッケージごとの別プロセス実行を足す | コードが増える。山が 1 本の巨大なモデル（`EngineV6` など）なら分けても下がらない |
| C. RPi では全体を回さない | MSL の全例題の判定が開発の主な検証なので、回せないと計画が進まない |

3. **elixir は入れない（このセッション）。** `MISE_DISABLE_TOOLS=elixir` で外して進める。erlang を kerl でソースビルドする（`mise.toml` の `compile = true`。apt の依存が要る）か、apt の erlang を使うかは、Elixir のラッパを触るセッションで決める。
4. **CI は変えない。** GitHub Actions は ubuntu-24.04（x86_64）と windows-2025 のまま。aarch64 の検査は RPi の手元だけ。

## 結果

- `mise run msl:compare`: RPi で 20 本中 20 本が一致（aarch64 の Rumoca CLI は x86_64 と同じ判定を返した）。xtask の debug ビルドを含めて 1055 s。
- `mise run check`: 全段通過。3798 s（x86_64 より大幅に遅い。clippy と test の debug ビルドが支配的）。
- `mise run msl:funnel`（release、swap 8 GB）: 566 本: OK 369・NG 13・GM-U 184・panic 0（x86_64 の S010 以降と同じ本数）。検査 1312.7 s（1 本 2.32 s。x86_64 は 1.50 s）、全体 24.6 分、最大の常駐メモリ（VmHWM）6.5 GiB。終了時の swap の使用 1.1 GiB。**RPi でも全体を回せる。**
- テストは workspace 全体で 487 本が通過・10 本 ignore。
- RPi では release ビルドと全体の実行に時間がかかるので、`msl:funnel`・`msl:sim` は裏で回し、他の作業と重ねる。cargo のロックがあるので、同じ worktree で 2 つの cargo を同時に走らせない。

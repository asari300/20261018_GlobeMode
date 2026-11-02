# S090 `cli-glibc-zig`

| | |
| --- | --- |
| 日付 | 2026-11-02 |
| ブランチ | `feature/s090-cli-glibc-zig` |
| worktree | `.git-worktree/s090-cli-glibc-zig` |
| Phase | P8 |
| セッション文書 | `doc/sessions/S090-cli-glibc-zig/` |
| セッション記録 | （親が写す） |

## 依頼

S086 の持ち越し。`release.yml` の `build` で作る `globe` の tarball（linux x86_64 / aarch64）がランナーの glibc 2.39 に link されていて古い Linux で動かないので、zig を cc/linker にして glibc 2.28 を狙う。CI の定義だけで、Rust のコードは変えない。ci.yml・`ci_plan.py`・`mise.toml`・pyproject.toml は触らない（S087 が同時に変えている）。

## 決めたこと

- `cargo-zigbuild`（MIT）を CI で `cargo install --locked` して使う。`CC="zig cc -target …"` と `CARGO_TARGET_*_LINKER` を自前で渡す道は、linker に引数を渡せないため wrapper のスクリプトが要り、長くなる。
- Linux と Windows を 1 つの段で分け、バイナリの場所を `BIN` として `GITHUB_ENV` に書く。後の段は `$BIN` を使う（cargo-zigbuild の出力は `target/<triple>/release/`）。
- 「動くか確かめる」の段で `objdump -T` の最大の `GLIBC_` を出す。
- aarch64 はネイティブのランナーで同じ行（クロスはしない）。
- sundials は release に入っていないので見ない（S044 の注記どおり）。
- README の配布の注記は変えない（2.28 で通ったため、「2.39 以上」と書く必要がない）。

## やったこと

- `.github/workflows/release.yml`: Linux で `cargo install --locked cargo-zigbuild` → `cargo zigbuild --release --locked -p globe-mode-cli --target "$TARGET.2.28"`。Windows は `cargo build` のまま。`BIN` と `objdump -T` の行を足し、まとめる段から使わなくなった `EXE` を外した。
- 文書: `doc/sessions/S090-cli-glibc-zig/`、索引、roadmap.md の持ち越しに取り消し線、CHANGELOG.md に 1 行。

## 検証

- 手元（WSL、x86_64）: `cargo zigbuild --release --locked -j 4 -p globe-mode-cli --target x86_64-unknown-linux-gnu.2.28` が 67 分 16 秒で成功（共有 target のロック待ちを含む）。`globe` 44,613,776 B、最大の記号 `GLIBC_2.28`。`globe check src/Modelica/GlobeModeValidation --model GlobeModeValidation.Simulation.Decay` は OK（S0〜S10、終了コード 0）。
- CI: `gh workflow run release.yml --ref feature/s090-cli-glibc-zig`（run 41399388807）が success（約 21 分）。
  - x86_64: `globe 0.0.1`、`GLIBC_2.28`、tarball 16,679,369 B（S086 の run 41393637221 は `GLIBC_2.39`・18,598,129 B）。build の段 10 分 49 秒。
  - aarch64: `globe 0.0.1`、`GLIBC_2.28`、tarball 14,450,491 B。build の段 11 分 46 秒。
  - Windows は変えていない（16 分 38 秒、success）。wheel の pytest は 4 passed。
- コードの検査: Rust のコードを変えていないので `mise run check` は回していない。
- `check_docs.py`: デッキ 91 件 OK / 文書 365 件・相対リンク 1760 件中の切れ 0 件。`render_slides.py` は 1 回試して時間切れ（30 s）、CI に任せる
- セッション記録の `--check`: 親が写す
- ステージした差分の漏洩検査: 0 件

## 残したこと

- 古い glibc の実機（Debian 10・RHEL 8 など）で動かしてはいない。記号の版で判断した。
- PyPI への公開と macOS の wheel・`globe`。

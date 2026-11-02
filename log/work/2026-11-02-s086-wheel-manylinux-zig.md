# S086 `wheel-manylinux-zig`

| | |
| --- | --- |
| 日付 | 2026-11-02 |
| ブランチ | `feature/s086-wheel-manylinux-zig` |
| worktree | `.git-worktree/s086-wheel-manylinux-zig` |
| Phase | P8 |
| セッション文書 | `doc/sessions/S086-wheel-manylinux-zig/` |
| セッション記録 | [2026-10-31-0b797c72-9651-45d3-b466-98f7c341d068.md](../sessions/2026-10-31-0b797c72-9651-45d3-b466-98f7c341d068.md) |

## 依頼

S077 の持ち越し。`release.yml` の Linux の wheel は `manylinux_2_39` で Debian 12（glibc 2.36）・RHEL 9（2.34）に入らない。maturin の `--zig` か manylinux の docker image で古い glibc を狙う。CI の定義だけで、Rust のコードは変えない。通らなければ `manylinux_2_39` のままにして注記を書く。

## 決めたこと

- `--zig` を取った（docker image より小さい。zig は `mise.toml` にある）。段は `manylinux_2_28`（manylinux2014 より新しく、手元の wheel の最大の記号が 2.28 で通った）。
- `uv build --wheel` はそのままにし、`MATURIN_PEP517_ARGS` に `--zig` を足しただけ。`uvx maturin build` に替えるより差分が小さい。`maturin[zig]` は足さず、mise の zig（PATH）を使う。pyproject.toml は変えていない。
- aarch64 も同じ行（ネイティブのランナーで `--zig`）。pytest の段は S077 のまま。

## やったこと

- `.github/workflows/release.yml`: Linux の mise-action の `install_args` に `zig`、wheel の段の `MATURIN_PEP517_ARGS` を `--compatibility manylinux_2_28 --zig`、冒頭の注記。
- 文書: `doc/sessions/S086-wheel-manylinux-zig/`、索引、roadmap.md の S077 の持ち越し、CHANGELOG.md。

## 検証

- コードの検査: Rust は変えていないので回していない
- 手元: `uvx maturin build --release --zig --compatibility manylinux_2_28` が通った（14 分 05 秒、zig 0.17.0）。`globe_mode-0.1.0-cp312-abi3-manylinux_2_28_x86_64.whl`（16,050,735 B）、最大の記号 `GLIBC_2.28`、wheel を入れた pytest 4 passed
- CI（release.yml の dry-run、run 41393637221）: success。`globe_mode-0.1.0-cp312-abi3-manylinux_2_28_x86_64.whl`（16,077,951 B）・`manylinux_2_28_aarch64`（14,137,068 B）・`win_amd64`（16,039,284 B）、pytest は 3 つとも 4 passed。build の段は x86_64 13 分 33 秒・aarch64 11 分 26 秒・Windows 17 分
- `check_docs.py`: デッキ OK / リンク 1698 件中の切れ 0 件。`render_slides.py` は手元で 110 s の時間切れ（CI に任せる）
- セッション記録の `--check`: 親が写す
- ステージした差分の漏洩検査: 0 件

## 残したこと

- `globe` の CLI 本体（`build` の段）はランナーの glibc 2.39 のまま。
- main へのマージ・push、remote の feature ブランチの削除、セッション記録の写しは親が行う。

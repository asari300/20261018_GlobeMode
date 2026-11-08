# S234 `fmu-export-cross-os`

| | |
| --- | --- |
| 日付 | 2026-11-08 |
| ブランチ | `feature/s234-fmu-export-cross-os` |
| worktree | `.git-worktree/s234-fmu-export-cross-os` |
| Phase | P5 |
| セッション文書 | `doc/sessions/S234-fmu-export-cross-os/` |
| セッション記録 | `（親が写す）` |

## 依頼

roadmap の「FMU の Export の残り」から Windows・macOS 向けの FMU と、Windows・macOS の fmusim。

## 決めたこと

- P5a の `--target` は S105 から windows・macOS（aarch64）を受けていた。足したのは `x86_64-macos` の確認（zig が書く Mach-O の magic）と、FMI 2.0 の `darwin64` の重なりを断る守り。
- fmusim は release に Windows（x86_64）と macOS（aarch64-darwin）がある（x86_64-darwin は無い）。`fmu-cross` に取得と実行を足した。
- P5b は cross せず、`fmu-native`（3 OS）でネイティブにビルド。`tests/fmpy.rs` の cfg を外し、cdylib の名前と置き場は OS で決める。`resource_path` の Windows は S213 で済んでおり変更なし。
- 1 つの FMU に P5b の複数 OS は対象外。

## 検証

- 手元: `cargo test -p globe-mode-fmu --test fmpy` 16 件 OK、`zig_cross_builds_win_and_macos_binaries`（PE と Mach-O）OK、`globe-mode-fmi` の export の試験 4 件 OK。
- `fmu-export.yml`（feature ブランチ、run 42472525126）success。fmu-export・fmu-cross（3 OS）・fmu-native（3 OS）。3 OS で FMPy の ME・CS と fmusim の ME・CS（macOS の 2.0 の simulate だけ飛ばす）。`tests/fmpy.rs` の 16 件は 3 OS で通った。
- 落ちた原因: github.com の release の直リンクが 504 を返し続けた（mise の `packslip.sigstore.json`・本体、fmusim。run 42465688885、42466331645、42467166460、42467877785、42468883216、42469109384、42470816597、42471079551）。`fmu-cross`・`fmu-native` は mise をやめてランナーの rustup と pipx の uv にし、fmusim は `gh release download`。macOS の fmusim 0.15.0 は FMI 2.0 でも `aarch64-darwin` を探すので、その 1 組だけ飛ばす。
- `mise run check` EXIT=0。`check_docs.py` 切れ 0 件、`render_slides.py` 問題 0 件。

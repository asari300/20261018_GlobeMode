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
- ランナー: （run の結果は報告に書く）

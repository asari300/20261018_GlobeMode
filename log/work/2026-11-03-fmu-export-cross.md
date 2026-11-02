# S105 `fmu-export-cross`

| | |
| --- | --- |
| 日付 | 2026-11-03 |
| ブランチ | `feature/s105-fmu-export-cross` |
| worktree | `.git-worktree/s105-fmu-export-cross` |
| Phase | P5（P5a の 3 回目） |
| セッション文書 | `doc/sessions/S105-fmu-export-cross/` |
| セッション記録 | （親が写す） |

## 依頼

S094/S098 の持ち越しの「Windows・macOS 向けの FMU のビルド」。`globe fmu export --build` に `--target <triple>`（複数可、既定は host）を足し、`zig cc -target …` で `win64`・`darwin64`（3.0 は `x86_64-windows`・`aarch64-darwin`）にも共有ライブラリを置く。1 つの FMU に複数の `binaries/<platform>/` を同梱する。検査は `fmu-export.yml` に Windows・macOS のジョブを足して FMPy で validate と ME・CS を回す。手元では zig で `.dll` ができる試験を 1 本。親（並列のセッション）から頼まれた。

## 決めたこと

- `--target` は `--build` を要し、何度でも付けられる。省略は今までどおり host を `cc`（`CC` で上書き可）。既存の呼び出しが変わらない。
- `cc` crate は使っておらず `Command` で呼んでいたので、`--target` のときだけ `zig cc -target <triple>`（`ZIG` で上書き可）に替える。新しい依存は無し。
- 置き場の名前は triple の arch と OS から決める（`platform()`、単体の試験あり）。FMI 2.0 は `linux64`・`win64`（x86_64 だけ）と `darwin64`（arch を区別しない）、3.0 は `<arch>-<os>`。それ以外は誤りを返す。
- zig の Windows 向けが置く `.pdb`（2.0 で 1,110,016 B）と import library `model.lib` を消す。
- `-lm` は Windows 向けにもそのまま渡す（`zig cc -target x86_64-windows-gnu … -lm` が通るのを確かめたので分岐を書かなかった）。
- CI は既存のジョブ `fmu-export` に zig を入れて 3 つの `--target` の FMU（2.0・3.0）を作り artifact `fmu-cross` に置き、新しいジョブ `fmu-cross`（ubuntu-24.04・windows-latest・macos-latest の matrix、checkout なし、mise で uv だけ）が FMPy 0.3.32 で validate・ME・CS を回す。|x(1) − e⁻¹| > 1e-6 で落とす。Decay は軽いので macOS も ME・CS の両方を回した。

## やったこと

- `src/Rust/mod/globe-mode-fmi/src/export.rs`: `write_fmu` に `targets: &[String]`、`build_linux64` を `build(…, triple: Option<&str>)` にし、`platform()` と単体の試験。
- `src/Rust/mod/globe-mode-cli/src/main.rs`: `globe fmu export --target`。
- `src/Rust/mod/globe-mode-cli/tests/fmu_export.rs`: `zig_cross_builds_a_win64_dll`（2.0 と 3.0、`.dll` が 1 つだけで先頭が `MZ`。zig か uv・FMPy が無ければ飛ばす）。
- `.github/workflows/fmu-export.yml`: zig の FMU と artifact、ジョブ `fmu-cross`。
- 文書: 解説書・スライド、ADR-0031 の追記、roadmap の P5 の行と「S105 で済んだこと」、索引。

## 検証

- `cargo test -p globe-mode-cli --test fmu_export`: 5 本通過（FMPy の ME −2.6075851755624768e-8、3.0 の CS 3.3306690738754696e-16 は S098 と同じ）。
- `cargo test -p globe-mode-fmi --features export --lib export`: 1 本通過。
- `fmu-export.yml`（feature から `gh workflow run`、run 41531879481）: 4 ジョブとも success。FMPy 0.3.32（相対許容誤差 1e-8）、2.0・3.0 とも同じ値。

  | ランナー | validate | ME の x(1) − e⁻¹ | CS の x(1) − e⁻¹ |
  | --- | --- | --- | --- |
  | ubuntu-24.04 | No problems found. | −2.608e-8 | 3.331e-16 |
  | windows-latest | No problems found. | −2.608e-8 | 3.331e-16 |
  | macos-latest（arm64） | No problems found. | −2.608e-8 | 3.331e-16 |

- FMU の中の大きさ（2.0、CI）: `linux64/*.so` 37,784 B、`win64/*.dll` 226,304 B、`darwin64/*.dylib` 55,504 B。
- コードの検査（`mise run check`）: CHECK_RESULT
- `check_docs.py`: DOCS_RESULT
- セッション記録の `--check`: 親が写す
- ステージした差分の漏洩検査: LEAK_RESULT

## 残したこと

- Windows・macOS の fmusim は回していない（fmusim は Linux だけ）。
- `aarch64-linux`（3.0）・`x86_64-macos` は置き場の名前を出せるが、ランナーで確かめていない。
- 入力・事象のあるモデル（P5b、S099）。fmuCheck は開発終了なので扱わない。

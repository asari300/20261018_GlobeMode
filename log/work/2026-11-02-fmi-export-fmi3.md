# S098 `fmi-export-fmi3`

| | |
| --- | --- |
| 日付 | 2026-11-02 |
| ブランチ | `feature/s098-fmi-export-fmi3` |
| worktree | `.git-worktree/s098-fmi-export-fmi3` |
| Phase | P5（P5a の 2 回目） |
| セッション文書 | `doc/sessions/S098-fmi-export-fmi3/` |
| セッション記録 | [2026-10-31-0b797c72-9651-45d3-b466-98f7c341d068.md](../sessions/2026-10-31-0b797c72-9651-45d3-b466-98f7c341d068.md) |

## 依頼

S094 の持ち越しのうち 3 つ: (1) FMI 3.0 の Export（`--fmi-version 2|3`、既定 2、`binaries/x86_64-linux/`、`--build` は 3.0 のヘッダ）、(2) FMPy で 3.0 の validate と ME・CS の simulate を試験に足し、Decay の x(1) と e⁻¹ の差を数字で書く、(3) FMU に Rumoca の Apache-2.0 の表示を同梱し、licenses.md に 1 段落。fmusim は手元で実行できなければ手動の新しい workflow `fmu-export.yml` で回す。main へのマージ・push は親。

## 決めたこと

- 版は `export_me_sources(model, fmi_version: u8)` と CLI の `--fmi-version`（clap で 2..=3、既定 2）。新しい型は作らない。
- 3.0 の `--build` は `sources/buildDescription.xml` の `PreprocessorDefinition` を読んで `-D` で渡す（roxmltree は既存の依存）。最初は渡しておらず、`.so` の関数が `Rumoca_<guid>_fmi3*` になり FMPy が `fmi3ActivateModelPartition` を引けなかった。`FMI3_OVERRIDE_FUNCTION_PREFIX=1` を固定で書くより、テンプレートが宣言した定義に従う方が Rumoca の更新に強い。
- ライセンス: `documentation/licenses/Rumoca-LICENSE.txt` に出典・著作権表示（`Copyright 2025-2026 CogniPilot Foundation`、Rumoca の LICENSE から）・Apache-2.0 の全文（リポジトリの `LICENSE-APACHE` を `include_str!`。Rumoca の LICENSE とは著作権表示の 1 行と空行だけが違うのを `diff` で確かめた）。2.0 と 3.0 の両方に入れる。
- fmusim は modelica/fmusim v0.15.0 の `fmusim-x86_64-linux.zip`（S094 と同じ出典、sha256 を固定）。Reference-FMUs の release には fmusim の資産が無い（fmusim は別リポジトリに移った）。
- `gh workflow run` は main に無いワークフローを 404 で断ったので、feature ブランチで 1 度だけ `push` のトリガを足して回し（`7f11b05`）、次のコミットで外した。
- 新しい依存は無い。Rumoca のコードは改変していない。

## やったこと

- `src/Rust/mod/globe-mode-fmi/src/export.rs`: `fmi_version`、3.0 のヘッダと置き場、`buildDescription.xml` の定義、ライセンスの同梱。
- `src/Rust/mod/globe-mode-cli/src/main.rs`: `--fmi-version`。
- `src/Rust/mod/globe-mode-cli/tests/fmu_export.rs`: 3.0 のソースの FMU の試験と、FMPy の試験を版で共有する補助（3.0 は ME と CS）。2 本 → 4 本。
- `.github/workflows/fmu-export.yml`（新規、手動だけ）。
- `doc/licenses.md`、`doc/adr/0031-fmi-export-p5a-first.md`（追記）、`doc/roadmap.md`、`doc/sessions/README.md`、`doc/sessions/S098-fmi-export-fmi3/`。

## 検証

- `cargo test -p globe-mode-cli --test fmu_export`: 4 本とも通過（FMPy 0.3.32、uv の Python 3.12）。
- FMPy（相対許容誤差 1e-8）: 2.0・3.0 とも ME の x(1) = 0.3678794150955906（e⁻¹ との差 −2.6e-8）、CS 0.36787944117144267（差 3.3e-16）。validate は「No problems found.」。S094 の「2.6e-9」は書き誤り。
- fmusim（run 41445403219、success、`fmusim --version` は 0.14.0）: 2.0・3.0 とも validate を通過、simulate（既定）は ME −1.567e-4、CS 3.3e-16。
- コードの検査（`mise run check`、`MISE_DISABLE_TOOLS=elixir`）: fmt・clippy（`-D warnings`）ok、`cargo test --workspace` は 611 通過・失敗 0・無視 11、`cargo deny check` は advisories・bans・licenses・sources とも ok、`uv sync --locked` ok。1 回目は共有 target の古い `globe-mode-fmi` で E0061 が出たので、触った crate を touch して回し直した。
- `check_docs.py`: デッキ S098 OK / 文書 418 件・相対リンク 2051 件中の切れ 0 件。`render_slides.py`（S098、desktop-light・dark・mobile の 6 枚）は問題 0 件。
- セッション記録の `--check`: 親が写すので対象外
- ステージした差分の漏洩検査: メールアドレス・ホームのパス・トークン 0 件

## 残したこと

- fmuCheck での検査。
- main に入った後に `gh workflow run fmu-export.yml` で回し直す。
- fmusim の ME の許容誤差を絞る試し。Windows・macOS 向けのビルド。入力・事象のあるモデル（P5b）。

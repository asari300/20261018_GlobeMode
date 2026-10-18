# S001 research-plan

| | |
| --- | --- |
| 日付 | 2026-10-18 |
| ブランチ | `feature/s001-research-plan` |
| worktree | `.git-worktree/s001-research-plan` |
| Phase | P0 運用基盤 |
| セッション文書 | [doc/sessions/S001-research-plan/](../../doc/sessions/S001-research-plan/解説書.md) |
| セッション記録 | [log/sessions/2026-10-18-cf689fe6-f0d2-41f8-a25d-1a34aca756c7.md](../sessions/2026-10-18-cf689fe6-f0d2-41f8-a25d-1a34aca756c7.md) |

## 依頼

Modelica 言語の IDE を作る。以下の要件について現在の技術を調査し、計画を立てる。

- 最初のステップとして、OpenModelica と Rumoca を参考に必要な機能を整理する
- セッションごとに HTML スライドと Markdown 解説書を作る
- セッション履歴の保存、CLAUDE.md、SKILL.md などを作る
- 要件
  - 主要言語は Rust。API は Rust / Gleam / Elixir / Zig / Julia / Python
  - デスクトップ、ブラウザ（WASM でゲストの端末のリソースを使う）、CLI、Notebook（marimo）
  - OS は Debian 系、Windows 11、Android
  - connect から Obsidian のようなネットワークグラフを描く
  - OSS として公開するので、依存ライブラリのライセンスに注意する
  - 類似プロジェクトとして Rumoca を参考にする
  - 遅延評価で処理を軽くし、シミュレーションできるモデルかをシミュレーション時にアサートする
  - ソルバは OpenModelica や Dymola のように複数に対応する
  - FMI（2.0 と 3.0）、Co-Simulation などは必須

## 決めたこと

### ユーザーと決めたこと（AskUserQuestion で確認）

| 項目 | 決定 | 選ばなかった案 |
| --- | --- | --- |
| FMI の版 | 「3.0&3.0」は「2.0 と 3.0 の両対応」の意味。ME / CS / SE の Import と Export を必須にする | 3.0 のみ / Export は 3.0 のみ |
| ライセンス | MIT OR Apache-2.0 | MPL-2.0 / Apache-2.0 のみ / GPL-3.0 系 |
| 名称 | GlobeMode のまま | 後で決める |
| 今回の範囲 | 調査・計画文書と運用基盤まで。コードは書かない | ＋Cargo の雛形 / ＋最小パーサの PoC |
| コンパイラ | まず Rumoca を git 依存で取り込み、余裕ができたら自前コアに移る。Rumoca は評価用の参照として残す。あわせて「ライセンス上問題ないか」を聞かれた → 問題ないと回答（下記） | 最初から自前コア / Rumoca を fork |
| UI | まず Tauri 2 + Svelte 5 で作る。Leptos と Dioxus は余裕ができたら試験機能として部分的に作り、負荷を比べる | Leptos のみ / Dioxus のみ |
| 配置 | ユーザーがセッション中に作った言語別の `src/<Lang>/` に合わせる。`src/Zig/` を追加する | ルート直下の `crates/` 構成 |
| Streamlit | 含める（結果ダッシュボード） | 計画に含めない |
| セッション記録 | 整形版を伏せ字にしてコミットし、生ログは管理外 | ローカル保存のみ / 両方コミット |

### Rumoca のライセンスについての回答

2026-10-18 に、Rumoca の `LICENSE`（Apache-2.0）、NOTICE ファイルの有無（無い）、`rust-toolchain.toml`、`Cargo.toml` を raw.githubusercontent.com で直接確かめた。

- Apache-2.0 なので、git 依存にしてよい。
- 配布時に守ること:
  - ライセンス文を同梱する。
  - 改変したら明記する（§4b）。
  - 製品名に「Rumoca」を使わない。
- 注意するのは推移的依存。umbrella crate `rumoca` の既定 feature が zenoh・gamepad・UDP を引き込むので、`rumoca-compile` などに絞る。
- 実務上のリスクは、nightly の固定と API の変化。

### 既定のまま進めたこと

- 文書は日本語。README には英語の要約を付けた。
- セッションの番号を S001 から振る。ブランチは `feature/sNNN-<topic>`。
- フックは兄弟リポジトリ `20260816_exam` のものを元にした。変えた点:
  - 出力先を `log/local/` にした。
  - 伏せ字を加えた。
  - 手動実行の引数（`--session` / `--transcript` / `--latest` / `--out` / `--check`）を足した。
- フックの出力先を、コミットする `log/sessions/` と分けて `log/local/` にした。
  - フックは main のチェックアウトに書く。worktree の側で同じパスにコミットすると、マージ時に main の未追跡ファイルとぶつかるため。
  - finish-session が伏せ字版をコピーしてコミットする。
- 同じプロジェクトで別のセッション（mise の手順書を作ったものと思われる）が並行して動いていた。
  - `--latest` だけでは取り違える恐れがあるので、`--match <ブランチ名>` で transcript を絞れるようにした。
  - 今回の記録は main の `log/local/` に書かず、scratchpad に出してからコピーした。main の `.gitignore` にはまだ `log/local/` が無いので、main に未追跡ファイルを増やさないため。
- スライドは外部 CDN を使わない。mermaid も使わず、図はインライン SVG で描いた。この環境は `file://` でオフラインで開く前提のため。
- 設計エージェントが指摘した「Python パッケージ名が数字で始まり import できない」問題は、P1 で workspace 化と一緒に直す。今回は触らない。

## やったこと

- **調査**: 3 つの観点で並行して調べた。
  - Rumoca / OpenModelica / Dymola
  - FMI と Rust の crate（ソルバ、遅延評価）
  - UI・グラフ・言語バインディング
  - 結果は `doc/research/` の 5 ファイルに、出典 URL と【未確認】の印を付けて書いた。
- **正典の文書**:
  - `doc/features.md`（6 領域の表）
  - `doc/architecture.md`
  - `doc/roadmap.md`
  - `doc/licenses.md`
  - `doc/adr/0001`〜`0005`
- **運用**:
  - `CLAUDE.md`
  - `.claude/settings.json`: main に未コミットで置かれていた権限設定 3 件をそのまま含め、フックと読み取り専用の権限を足した。
  - `.claude/hooks/save_session_log.py`
  - スキル 4 つ（start-session、finish-session、session-docs、add-dependency）
  - `check_docs.py`
- **セッション文書**:
  - `doc/assets/slides.{css,js}`: 共通。ライトとダーク、キーボードとスワイプ、一覧表示、印刷に対応。
  - `doc/sessions/_template/`
  - `doc/sessions/S001-research-plan/{slides.html,解説書.md}`: スライドは 20 枚。
  - `doc/sessions/README.md`
- **記録**: `log/README.md`、`log/work/_TEMPLATE.md`、この作業記録、`log/sessions/` の伏せ字版。
- **その他**:
  - `README.md`
  - `LICENSE-MIT`
  - `LICENSE-APACHE`: apache.org の原文。
  - `.gitignore`: `.git-worktree/`、`log/local/`、`data/msl/`、`node_modules/`、mise の個人用ファイル、`.claude/` のローカル状態を足した。
  - `src/{Rust,Python,Elixir,Gleam,Julia,Zig,Modelica,Svelte}/README.md`

## 検証

- コードの検査: 対象外（Rust のコードは書いていない）
- `check_docs.py`:
  - デッキ 2 件とも OK。S001 は 20 枚で、インライン SVG が 6 つ、外部参照・`<style>`・`style=` は 0 件。雛形は 6 枚。
  - 文書 37 件、相対リンク 100 件で、切れは 0 件（見出しの anchor も含む）。
- Markdown の lint: `pymarkdown`（`-d md013`、front-matter 拡張あり）で指摘 0 件。
  - 最初の実行では MD036（太字の行を見出し代わりにしている）9 件と MD033 1 件が出たので、直した。
  - IDE の markdownlint 診断も 0 件。
- フックの単体確認（合成した transcript）:
  - 伏せ字: メール・`/home/<user>`・`C:\Users\<user>`・`/mnt/c/Users/<user>`・トークン（`ghp_`、`AKIA`、`token=`）・slug に残るユーザー名
  - 伏せ字版の `--check` は 0 件。生ログ相当のファイルでは 10 件を検出した（検出できることの確認）。
  - 壊れた入力を渡しても終了コードは 0 だった。
- セッション記録の `--check`: 取りこぼし 0 件。
  - 追加で直接検索した結果、ログイン名・git のメールアドレス・`gmail`・`/home/` はいずれも 0 件だった。
  - 途中で、マスク済みのメール（`n***@…`）が伏せ字にならないことに気づいた。メールの正規表現のローカル部に `*` を足して直した。
- ステージした差分の漏洩検査（ログイン名・git のメールアドレス・トークンの形）: 0 件。
- ブラウザでの見た目（Playwright 1.64、Chromium build 1248）: `render_slides.py` で問題 0 件。
  - 2 デッキ × 3 表示（デスクトップのライト・デスクトップのダーク・スマートフォン）で確かめた。
  - 最初の実行で、機械検査は 0 件だったが、スクリーンショットを見て次の 4 点を直した。
    - 右下の操作ボタンがページ番号を隠していた。`.foot` の右に余白を取り、この重なりを検査に加えた。
    - スマートフォンでは横長の図が縮みすぎて読めなかった。図に最小幅を付け、スライドの中で横にスクロールさせた。図・表・コードの上のスワイプはページ送りにしない。
    - S17 で、Track B の説明文が枠からはみ出していた。
    - S05 のファネル図の文字が小さかった。

## 追記: main の取り込み（同じ日、ユーザーの依頼で）

- main に `db789c4 Added manage project file` が入っていた。内容は mise の導入。
  - `mise.toml`、`mise.lock`、`doc/mise-setup.html`、`.claude/settings.json`、`.gitignore`、`pyproject.toml` の uv_build の範囲。
- そのコミットには、worktree `.git-worktree/s001-research-plan` が **gitlink（mode 160000）** として入っていた。
  - クローンすると壊れたサブモジュールになるので、取り込むときに index から外した（`git rm --cached`）。
  - `.gitignore` に `.git-worktree/` があるので、今後は入らない。
- 衝突は 2 件で、どちらもこのブランチの版を採った。
  - `.claude/settings.json`: main の権限 3 件をすでに含む上位互換。
  - `.gitignore`: mise の 2 行をすでに含む。
- ユーザーの指示「言語管理は mise・uv・cargo」に合わせて、次を直した。
  - ADR-0003、architecture、CLAUDE.md、roadmap、スキル、スライド、解説書
  - `rust-toolchain.toml` は置かない。mise の `RUSTUP_TOOLCHAIN` が優先されるため。手順書の方針に合わせた。
  - worktree では `mise trust && mise install --locked` を実行してから `uv sync` する。
- ブラウザでの確認は Playwright に決めた。
  - `uv add --dev playwright`（Apache-2.0）で開発依存として入れた。
  - `.claude/skills/session-docs/render_slides.py` を足した。

## 残したこと

- （解決済み）main に未コミットだった `.claude/settings.json` と `doc/mise-setup.html` は、ユーザーが main にコミットした。上の「追記」のとおり取り込んだ。
- `pyproject.toml` の `uv_build>=0.12.6,<0.13.1` は、mise の uv 0.13.1 を範囲に含まない。
  - ビルドのたびに uv_build 0.13.0 が PyPI から取得される。動作はする。
  - 意図した範囲かどうかは S002 で確かめる。
- 調査文書の【未確認】の項目。P1〜P5 で一次資料に当たって確かめる。
- `mise.toml` の rust に wasm と Android の `targets` を足すのは S002 で行う。
- git の作者のメールアドレスはコミットのメタデータに入る。公開前に、GitHub の noreply アドレスに変えるかを検討する。

# S194 `docs-decisions`

| | |
| --- | --- |
| 日付 | 2026-11-06 |
| ブランチ | `feature/s194-docs-decisions` |
| worktree | `.git-worktree/s194-docs-decisions` |
| Phase | 文書 |
| セッション文書 | `doc/sessions/S194-docs-decisions/` |
| セッション記録 | （親が写す） |

## 依頼

2026-11-06 にユーザーが決めた 4 点を文書に反映する（文書と再現用 `.mo` の整形だけ）。(1) CLAUDE.md の 2 章を現状に合わせる、(2) ADR-0032・0034・0035・0036 を Accepted に、(3) 上流の再現用 `.mo` を `globe fmt` で整形、(4) roadmap 末尾の表を更新。

## 決めたこと

- ユーザーと決めた: CLAUDE.md は 2 章の表と説明だけ変える（1・3〜7 章は不変）。ADR の 4 本は Accepted。再現用 `.mo` は整形する。FMI Import は案 A と案 B の両方（実装は別の WP）、Python の例外の階層・補完中の Tab・CLI の終了コードなどは別の WP。
- 既定で進めた: workflow の項目は CLAUDE.md には 5 本を短く書き、細部は architecture.md の 2.5 章へのリンクにした。報告 5 本（`msl-funnel.md` など）は「手で編集しない」の行にまとめた。
- 再現用 `.mo` は、全クラスの `globe check` の診断（コード、重大度）と止まった段が前後で同じなら整形する基準にした。34 本すべてが基準を満たした。
- スライド `_jp.html` のコードは .mo の抜粋ではなく縮めた擬似コードなので変えなかった。本文の診断の `行:列` は整形後の位置に直した。

## やったこと

- `CLAUDE.md`: 2 章の表（apps、Svelte、VSCode、Astro、Python、Elixir 以下、Modelica、doc の行）、workflow の項目 5 つ、mise の表、共有 target の項目。
- `doc/adr/0032`・`0034`・`0035`・`0036` の状態、`doc/adr/README.md`、`doc/roadmap.md`（本文 3 か所と末尾の表）、`doc/architecture.md`（1 か所）。
- `doc/research/upstream/*.mo` 34 本を整形、9 本の `.md` の診断の位置、`README.md` に規則を 1 行。
- `doc/sessions/S194-docs-decisions/`、`doc/sessions/README.md`、`CHANGELOG.md`。

## 検証

- 整形した `.mo`: 34 本、比べたクラス 270、診断（コード、重大度）・passed・止まった段が前後で違ったクラス 0。
- `globe fmt --check doc/research/upstream`: 34 ファイルすべて整形済み（`globe` は main の先頭で 08:15 にビルドされた 0.2.0 を `~/.cache/globe-mode-s194/` に写して使った。fmt と frontend はそれ以後変わっていない）。
- コードの検査（fmt / clippy / test / deny）: Rust を触っていないので回していない。
- `check_docs.py`: デッキ 全件 OK（S194 を含む）/ リンク 3876 件中の切れ 0 件。`render_slides.py`（S194 の 5 枚、ライト・ダーク・スマートフォン）: 問題 0 件。`_jp.html` は触っていないので回していない。
- セッション記録の `--check`: 親が写す。
- ステージした差分の漏洩検査: 0 件（メールアドレス・ホームのパス・トークン）。

## 残したこと

- 実装の WP: FMI Import（ADR-0026 の本文）、Python の例外の階層、補完中の Tab、CLI の終了コードなど。
- 公開・鍵・Rumoca の rev・上流 issue の提出は変わらずユーザーの判断。
- CLAUDE.md の 4 章以降の古い記述は触っていない（規則は変えない約束）。

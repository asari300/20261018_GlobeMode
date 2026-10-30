# S033 `parallel-plan`

| | |
| --- | --- |
| 日付 | 2026-10-30 |
| ブランチ | `feature/s033-parallel-plan` |
| worktree | `.git-worktree/s033-parallel-plan` |
| Phase | 計画（P3 と P4 の間） |
| セッション文書 | `doc/sessions/S033-parallel-plan/` |
| セッション記録 | `log/sessions/2026-10-30-0e301cfb-c9ff-440e-b48e-89148a3cfe93.md` |

## 依頼

「リポジトリを俯瞰し設計方針へのアドバイス、並行して進められそうな作業プランを考えて他のエージェントに与えられる形にして」。リリースまでの時間とトークン量の見積もり。Codex を監査役にするのでレビュー履歴だけを残すディレクトリを log に。途中で追加: Orca / Hermes Agent / OpenRouter の Nemotron など無料プロバイダをワーカーにする戦略、Jev などの判断特化モデルの活用、このセッションのサブエージェントは Opus / medium。

## 決めたこと

- 見積もりは `log/local/raw/*.jsonl.gz` の `usage` の合計（主セッション 10 本。サブエージェント分は含まない）から出し、roadmap の目安に係数 2 を掛けた。金額は書かない。
- 設計の助言: ソルバの追加を止める（`ida` は P8）、次の 1 回は protocol と facade、プロセス構成を P4 の最初に ADR、UI はブラウザ版 → Tauri、Track B は P4 の後。
- レビュー履歴は `log/review/` に 1 レビュー 1 枚（指摘は原文、採否は実装側、差分は書かない）。`/finish-session` の検査の後、コミットの前。
- 無料プロバイダは機械的に検証できる作業だけに渡し、worktree を分けて push させない。判断特化モデルは採否の判定だけ。どちらも導入の具体は【未確認】のまま。
- 既定のまま進めたこと: サブエージェントは使わなかった（読むだけで足りた）。worktree で `mise install`・`uv sync` は回していない（文書だけ。Playwright は main の `.venv` で回した）。

## やったこと

- `doc/plan/2026-10-30-release-estimate-and-parallel-plan.md`（新規）: 俯瞰、助言 8 点、実測、見積もり、WP-0〜WP-J の指示書、Codex の運用、無料プロバイダと判断特化モデル。
- `log/review/README.md`・`_TEMPLATE.md`（新規）。`CLAUDE.md` の表と「6. ログ」、`log/README.md` に 1 項目ずつ。
- `doc/roadmap.md` の「運用上の決まり」から計画へリンク。
- `doc/sessions/S033-parallel-plan/`、`doc/sessions/README.md` の行。

## 検証

- コードの検査: 触っていないので回していない。
- 生ログの集計: ターン 5,427、出力 7.06 M、キャッシュ書き込み 18.8 M、キャッシュ読み出し 1.38 G。
- `check_docs.py`: デッキ 39 件 OK / リンク 937 件中の切れ 0 件
- `render_slides.py`: デッキ 1 件 / 問題 0 件（図の文字のはみ出しを 1 枚直した）
- セッション記録の `--check`: 取りこぼし 0 件（10 件を検査）
- ステージした差分の漏洩検査: 0 件

## 残したこと

- 見積もりはサブエージェントの transcript を含まない。UI の生産性は未測定で、P4 の最初の 2 回で測り直す。
- Orca / Hermes Agent / Jev の導入手順と規約は【未確認】。導入したら `doc/research/` に記録し、ADR-0008 に追記する。
- 次: WP-0（protocol と facade）。

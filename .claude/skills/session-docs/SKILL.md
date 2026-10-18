---
name: session-docs
description: GlobeMode のセッションごとの HTML スライド（slides.html）と Markdown 解説書（解説書.md）を書く。共通の CSS と JS だけを使い、外部 CDN を使わずオフラインで開けるように作り、check_docs.py で検査する。「スライドを作って」「解説書を書いて」「セッションのまとめ」と言われたとき、または finish-session の前に使う。
---

# セッションのスライドと解説書

毎セッション `doc/sessions/SNNN-<topic>/` に2つを作る。雛形は [doc/sessions/_template/](../../../doc/sessions/_template/)。

| ファイル | 読み手 | 役割 |
| --- | --- | --- |
| `slides.html` | 後でこのセッションを振り返る人、レビューする人 | 決めたことを一枚ずつ見せる。1 枚 1 メッセージ |
| `解説書.md` | 次のセッションの作業者（人と Claude） | 決定と理由、再現手順を文章で残す。正典の文書（`doc/*.md`、ADR）へリンクする |

## 共通の章立て

どちらも次の順に書く。スライドは各章を 1〜4 枚にする。

1. **目的**: このセッションで何をするか。依頼の要約と、どの Phase の作業か
2. **決定と理由**: 選択肢と、なぜそれを選んだか。ユーザーと決めたことは明記する
3. **図**: 構成・流れ・対応表
4. **再現手順**: 同じ結果を得るためのコマンドと、検証の数字
5. **次回**: 積み残しと、次のセッションでやること

## スライド（slides.html）の約束

- 読むのは `../../assets/slides.css` と `../../assets/slides.js` の **2 つだけ**。
  - 外部 CDN（フォント、mermaid、highlight.js など）を読まない。`file://` でオフラインのまま開けるようにするため。
  - `<style>` 要素も `style=` 属性も書かない。新しい見た目が要るなら、[doc/assets/slides.css](../../../doc/assets/slides.css) にクラスを足す。
- 色は CSS 変数（`--bg` `--fg` `--muted` `--accent` `--accent-2` `--warn` `--ok` `--line` `--panel` `--code-bg`）だけを使う。ライトとダークは `prefers-color-scheme` で切り替わる。
- 1 枚は `<section class="slide" id="S01">`。id は S01 から連番にする。
- 図はインライン SVG で描く。
  - 色は `fill="currentColor"` か、CSS のクラス（`.svg-box` `.svg-accent` `.svg-line` `.svg-text` など）で付ける。生の色値を書くと、ライトかダークのどちらかで読めなくなる。
  - `<svg>` には `viewBox` を付け、幅は CSS に任せる。
- 使えるクラス（詳細は slides.css）:

  | クラス | 用途 |
  | --- | --- |
  | `.tag` | 左上の章ラベル |
  | `.lead` | 副題 |
  | `.cols-2` `.cols-3` | 段組み |
  | `.card` | 囲み |
  | `.big` | 中央の大きな結論 |
  | `.kpi` | 数字のタイル |
  | `.ok` `.warn` `.ng` | 状態の色 |
  | `table.compact` | 詰めた表 |
  | `.foot` | フッター（slides.js が「n / 全体」を入れる） |

- 操作: ← → / PageUp PageDown / Space / Home End / `#S05` のハッシュで移動する。`p` で全スライドの一覧、印刷は 1 枚 1 ページ。

## 解説書（解説書.md）の約束

- 冒頭に表を置く: セッション番号、日付、ブランチ、Phase、スライドへのリンク。
- 図は mermaid のコードブロックで書く（GitHub で描画される）。
- 調査した事実には出典を付ける。正典の文書にあるものはリンクで済ませ、二重に書かない。
- markdownlint の警告を出さない。

## 検査

```bash
python3 -I .claude/skills/session-docs/check_docs.py
```

デッキが NG、またはリンク切れが 1 件でもあれば直す。

ブラウザでの見た目は、ユーザーに `file://` で開いてもらって確かめる。この環境にはブラウザが無い。

# Modelica の整形の規則（`globe-mode-fmt`）

S053 でユーザーと対話して決めた既定の形。実装は `src/Rust/mod/globe-mode-fmt`（依存 0、wasm と CLI でそのまま使える）。
入口は `globe fmt` / `globe lint`、LSP の `textDocument/formatting`、wasm の `format` / `lint`、ブラウザ UI の「整形」。設計の判断は [ADR-0024](adr/0024-own-formatter-and-lint.md)。

## 1. 規則

| 項目 | 既定 |
| --- | --- |
| インデント | 空白 2（`--indent 4` で明示したときだけ 4） |
| 行幅 | 80 桁（文字単位）。収まらない呼び出し `Name(` と配列 `{` は開き括弧で改行し、中身を 1 段下げ、閉じ括弧を元の段に単独で置く |
| 引数の詰め方 | 収まる引数は同じ行に詰める。展開した引数の後と、並べ替えのグループの境目では必ず改行する |
| 図形プリミティブの引数の並べ替え | `Rectangle` `Ellipse` `Polygon` `Line` `Text` `Bitmap` の中だけ。順は ① 位置・形状（`visible origin rotation extent radius smooth startAngle endAngle closure`）→ ② `points arrow arrowSize` → ③ `fillColor fillPattern` → ④ `lineColor pattern lineThickness borderPattern` → ⑤ その他（元の順）。位置引数が混ざっていれば並べ替えない |
| `annotation` と `Documentation` の引数の並べ替え（S061） | `annotation(…)` は `experiment` → `Diagram` → `Icon` → `Documentation` → その他（`uses` など、元の順）。`Documentation(…)` は header（`__OpenModelica_infoHeader`）→ `info` → `revisions` → その他。ユーザーの指定は experiment → Diagram → Documentation で、`Icon` は `Diagram` の隣に置いた。整形が並べ替え、lint の `annotation-order` が通知する |
| `origin` の補完 | 図形プリミティブに `origin` が無ければ `origin = {0, 0}` を足す。`--no-insert-origin`（`Options.insert_origin = false`）で止められる |
| 空行 | `within …;` の後、クラス頭（`model X "…"`）の後、`end X;` の前に 1 行。`equation` `algorithm`（`initial` 付きも）の前後に 1 行（S061。`--no-blank-around-sections`、`Options.blank_around_sections = false`、VS Code の `globeMode.blankAroundSections` で止められる）。連続する空行は 1 つに畳む。ほかは書いたとおり |
| 節の段 | `equation` `algorithm` `initial …` `public` `protected` と、クラス直下の `annotation(` は**クラスと同じ段**。宣言・式・入れ子のクラスは 1 段下げる。`if` / `for` / `while` / `when` の中は 1 段下げ、`else` と `end if;` は開いた段に戻す |
| 空白 | 二項演算子（`= := + - * / < > <= >= == <> .+ .- .* ./ .^`）の両側に 1 つ。カンマは後ろだけ。`^` `:` `.` は詰める。単項の `-` `+` は詰める。括弧の内側に空白なし。識別子と `(` `[` は詰める（`der(x)`、`x[1]`、`annotation(`）。`if (` `then (` など予約語の後ろの `(` は空ける |
| 修飾子の `=` | `start = 1`（詰めない。リポジトリの Modelica と同じ） |
| 行末 | 行末の空白を消し、末尾に改行 1 つ。改行コードは入力に CRLF があれば CRLF |
| 触らないもの | 文字列とコメントの中身。行コメントは元の行に残す |
| 壊れた入力 | 括弧や `end` が合わない・`;` が抜けているときは、本文に触らず `Err`（CLI は終了コード 1） |

安全装置: 整形の出力を字句解析し直し、空白以外のトークン列が意図した列（並べ替えと `origin` の補完を適用したもの）と一致しなければ `Err` にして本文を変えない。

## 2. 例

Before（`src/Rust/mod/globe-mode-fmt/tests/data/objects_before.mo`）:

```modelica
within Alfheim;
model Objects_Before
  annotation(
    Diagram(graphics = {Rectangle(fillColor = {7, 84, 112}, fillPattern = FillPattern.Solid, borderPattern = BorderPattern.Raised, extent = {{-100, 100}, {100, -100}}, radius = 25), Polygon(origin = {0, 40}, fillColor = {207, 241, 244}, fillPattern = FillPattern.Solid, points = {{0, -40}, {-80, 0}, {0, 40}, {80, 0}, {0, -40}})}));
end Objects_Before;
```

After（抜粋。全文は `objects_after.mo`）:

```modelica
within Alfheim;

model Objects_Before

annotation(
  Diagram(
    graphics = {
      Rectangle(
        origin = {0, 0}, extent = {{-100, 100}, {100, -100}}, radius = 25,
        fillColor = {7, 84, 112}, fillPattern = FillPattern.Solid,
        borderPattern = BorderPattern.Raised
      ),
      Polygon(
        origin = {0, 40},
        points = {{0, -40}, {-80, 0}, {0, 40}, {80, 0}, {0, -40}},
        fillColor = {207, 241, 244}, fillPattern = FillPattern.Solid
      )
    }
  )
);

end Objects_Before;
```

## 3. lint の規則

| 規則 | LSP のコード | 中身 |
| --- | --- | --- |
| `line-too-long` | `GM-L0001` | 整形しても行幅を超える行（長い文字列など） |
| `class-name-pascal-case` | `GM-L0002` | クラス名が大文字で始まらない |
| `missing-description` | `GM-L0003` | 長いクラス（`end` を持つもの）に説明の文字列が無い |
| `annotation-order` | `GM-L0004`（通知の重さ） | `annotation(…)` と `Documentation(…)` の引数が 1 章の順でない（S061）。整形で直る |

`globe lint --disable <rule>` で止められる。意味の検査（S0〜S10）は従来どおり `globe check` と LSP の診断。

## 4. 使い方

```bash
globe fmt                           # カレントディレクトリの下の .mo を整形（ruff format と同じ）
globe fmt src/Modelica a.mo         # ファイルとディレクトリを混ぜてよい
globe fmt --check .                 # 書き換えず、整形が要るファイルを列挙して終了コード 1
globe lint                          # カレントディレクトリの下を lint（ruff check と同じ）
echo 'model M end M;' | globe fmt --json   # 標準入力。{"ok":true,"text":"…","error":null}
```

ディレクトリの辿り方は ruff に倣う（S060）。

- `.gitignore` に書いたものと、隠しディレクトリ（`.git` や `.venv` など）は辿らない。git の外でも `.gitignore` を読む。明示したファイルは除外しない。
- 壊れたファイルがあっても残りを続け、最後に要約（「3 ファイルを整形した、66 ファイルは変更なし」）を出す。壊れたファイルがあれば終了コード 2。
- パスを省いたとき、`--json` を付けたときか入力がパイプのときは標準入力を読む（ウェブアプリとエディタの呼び方）。`-` でも標準入力。

VS Code では拡張 [src/VSCode/globe-mode](../src/VSCode/globe-mode/README.md)（S059）を入れると、Shift+Alt+F（拡張が `.mo` に割り当てる。S060）で整形し、lint は編集中に波線で出る。保存時の整形は設定 `globeMode.formatOnSave`（既定はオフ）。拡張は `.mo` の既定を `editor.tabSize: 2` にする（LSP の整形は tabSize をインデントに使うため）。

S053 の時点で `src/Modelica/` の 69 ファイルすべてが整形で変わる（空行の規則のため）。既存ファイルの書き換えはしていない。

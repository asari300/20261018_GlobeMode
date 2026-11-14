# S278 `trackb-flatten`

| | |
| --- | --- |
| 日付 | 2026-11-15 |
| ブランチ | `feature/s278-trackb-flatten` |
| worktree | `.git-worktree/s278-trackb-flatten` |
| Phase | Track B（B3 の最初の一切れ） |
| セッション文書 | `doc/sessions/S278-trackb-flatten/` |
| セッション記録 | なし（このセッションの記録が `log/local/` に無いため省く） |

## 依頼

B3 flatten の最初の一切れ。新しい crate で、モデルを平らな変数と方程式の列にする（部品の入れ子・`extends`・修飾・parameter の束縛・名前の付け替え・`connect`・大きさが定数の 1 次元までの配列と `for`）。`redeclare`・`replaceable`・外部関数・条件つきの宣言は断る。Rumoca の flat と `GlobeModeValidation` で突き合わせ、MSL の例題 566 本で「平らにできた本数」を数える。resolve の中は S275 が触るので変えない。

## 決めたこと

- 新しい crate `globe-mode-flat`（依存は syntax・resolve。rowan は `TextRange` の型だけで、両方と同じ）。resolve は `bases` を公開していないので、`extends` の名前の解決表から基底を `Project` 側で補った。
- 式は評価せず、名前を付け替えた印字で残す。評価は大きさ・`for` の範囲・添字だけ。
- 束縛は変数に残す（Rumoca の flat と同じ）。`when` は別の列。
- 突き合わせで分かった Rumoca の flat の定義に合わせた: 最上位のコネクタの flow = 0、`equations` に束縛と `when` を数えない、`StateSelect` の変数を入れない、`(a, b) = f(x)` は出力の数。
- Rumoca の flat の変数名を取るため `FlatSummary::variable_names` を足した（`#[serde(skip)]`。JSON は変わらない）。
- `inner`/`outer`・条件つきの宣言（`if` が定数なら外せる）は、依頼のとおり今回は断る。数字を見て次の切れで決める。

## やったこと

- `src/Rust/mod/globe-mode-flat/`（新規）: `lib.rs`（`Project`・出力の型・`Unsupported`）、`flatten.rs`（インスタンス化・修飾・印字・定数の評価・`for` の展開・`connect`）、`tests/flat.rs`（単体 7 本）、`tests/compare.rs`（`#[ignore]` の計測 4 本）。
- `globe-mode-ir` / `globe-mode-frontend`: `FlatSummary::variable_names`（突き合わせ用。`#[serde(skip)]`）。
- doc: ADR-0039 の付記、roadmap（Track B の 2 か所）、architecture の crate 表、features、セッションの索引、セッション文書。

## 検証

- `cargo test -p globe-mode-flat`: 7 pass・4 ignored。clippy `-D warnings` 警告 0、fmt ok。
- `GlobeModeValidation` の 75 本: 自前で平らにできた 71 本（断った: `redeclare` 3、`inner`/`outer` 1）。Rumoca の flat まで通った 74 本のうち両方で通った 70 本は、名前の集合 70/70・方程式の数 70/70 が一致（初回は 69 / 52 で、上の 4 つの定義の差を直した）。
- MSL 4.1.0 の例題 566 本: 自前で平らにできた **67 本**（Rumoca の flatten は 562）。両方で通った 66 本で名前 66/66・方程式 64/66。断った 499 本は `redeclare` 205・条件つきの宣言 131・`inner`/`outer` 43・大きさが定数でない 35・レコードの束縛 33・`[:]` 25・`class extends` 11・`replaceable` 5・配列の型 5・配列のコネクタの一括接続 4・`expandable` 2。
- 注意: 共有 target に別の worktree（S275）の resolve のビルドが混ざり、最初の MSL の計測が 11 本だった（`a.b` が型を通した要素に解決されていた）。変えていない crate も touch してから回すと 67 本になる。結果の数字はすべて touch 後のもの。

## 残したこと

- `redeclare` / `replaceable`（S275 と合わせる）、条件つきの宣言、`inner`/`outer`、配列のコネクタの一括接続、レコードの束縛、`[:]` の大きさ（束縛が関数の戻り値）、B4（DAE）、`xtask diff-frontend` への組み込み。
- MSL で方程式の数が合わない 2 本（`UniformNoise` 10 対 12、`UniformNoiseXorshift64star` 10 対 11）の原因。

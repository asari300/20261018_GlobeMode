# 下書き（未提出）43: 組み込みの列挙 `AssertionLevel`（と `StateSelect`）の parameter が flat のモデルから消える

対象: Rumoca（<https://github.com/CogniPilot/rumoca>）。英語の本文は [43-predefined-enumeration-parameter-dropped.md](43-predefined-enumeration-parameter-dropped.md)、再現は [.mo](43-predefined-enumeration-parameter-dropped.mo)。

判定（GlobeMode、S373）: **軽いバグ**（変数が無くなる。誤った方程式は見つけていない）。`parameter AssertionLevel level = AssertionLevel.warning;` は、組み込みの列挙型 `AssertionLevel` の普通の parameter（MLS 3.7 の 8.3.7 は `AssertionLevel` を組み込みの列挙型と呼ぶ。2026-11-16 に <https://specification.modelica.org/maint/3.7/equations.html> で読んだ。その型の宣言を禁じる文は無いが、許す文も見つけていない【未確認】）。Rumoca の flat のモデルには変数 `level` が無い。`parameter StateSelect sel = StateSelect.prefer;` も同じ。ユーザーが宣言した列挙の parameter は残る（対照 `C`）。omc 1.27.2 は両方残す。

状態（S373、2026-11-16）: v0.10.2（GlobeMode の固定）と v0.10.4（最新のリリース）で再現。Rumoca の main はビルドしていない。

## 概要

`A`・`B` の flat のモデル（`--emit flat-json`、`variables` のキー）は `x` だけ。`C` は `level` と `x`。方程式の中で消えた parameter を読む式は値に置き換わる（`x = if level == AssertionLevel.warning then time else 0` は `(AssertionLevel.warning == AssertionLevel.warning)` になる）ので、モデルはコンパイルできる。失われるのは変数そのもの: 平らにした後で外から修飾できず、parameter の一覧に出ず、変数の名前の集合がほかの道具と食い違う。

## 再現手順

```bash
rumoca compile 43-predefined-enumeration-parameter-dropped.mo -m Rep43.A --emit flat-json   # AssertionLevel
rumoca compile 43-predefined-enumeration-parameter-dropped.mo -m Rep43.B --emit flat-json   # StateSelect
rumoca compile 43-predefined-enumeration-parameter-dropped.mo -m Rep43.C --emit flat-json   # 対照: ユーザーの列挙
```

`variables` のキーを見る。

## 期待

`Rep43.A` は `level` と `x`、`Rep43.B` は `sel` と `x`。`C` と omc と同じ。

## 実際

| モデル | `variables` のキー（v0.10.2） | `variables` のキー（v0.10.4） |
| --- | --- | --- |
| `A`（`AssertionLevel`） | `x` | `x` |
| `B`（`StateSelect`） | `x` | `x` |
| `C`（対照: ユーザーの列挙） | `level`、`x` | `level`、`x` |

omc 1.27.2 の `instantiateModel(Rep43.A)` は `parameter enumeration(warning, error) level = AssertionLevel.warning;`、`Rep43.B` は `parameter enumeration(never, avoid, default, prefer, always) sel = StateSelect.prefer;`。

## 環境

- Rumoca v0.10.2（配布の CLI `rumoca 0.10.2`。GlobeMode は rev `276e82bd0fd48960ac201e782918d28bf19cf7ce` に固定）と v0.10.4（配布の `rumoca 0.10.4`、タグのコミット `9c8c56fcd9f5ab9beb871f09bf5d9eed82971c54`、<https://github.com/CogniPilot/rumoca/releases/tag/v0.10.4>。2026-11-16 でも最新のリリース。S359 で確認）。Rumoca の main はビルドしていない。
- Linux x86_64（WSL2、Ubuntu）。omc 1.27.2。

## GlobeMode 側の回避

要らない。GlobeMode の自前の flat（`globe-mode-flat`）は `AssertionLevel` の parameter を残し（omc と同じ）、`StateSelect` のものは変数の名前の集合を Rumoca に合わせるためだけに落とす（`flatten.rs` の `ty == "StateSelect"`）。`AssertionLevel` の名前の差は `cargo xtask diff-frontend` が報告する。

## 補足

- ThermofluidStream 1.4.1（コミット `a2faa411`）で見つけた。`ThermofluidStream.DropOfCommons` が `parameter AssertionLevel assertionLevel = AssertionLevel.error;` を宣言する。`inner DropOfCommons dropOfCommons` を持つ `ThermofluidStream.Idealized.Examples.TUMExercisesThermodynamicCycles.Exercise3OttoEngine.BaseModel` と `Exercise4DieselEngine.BaseModel` は、自前の flat に変数 `dropOfCommons.assertionLevel` があり、Rumoca の flat に無い（変数は 22 対 21 と 19 対 18。方程式は 0 と 0）。このライブラリで Rumoca が平らにできた例題 25 本のうちの 2 本（S373 が S369 の `libraries.yml` の run を見直した）。
- 出典: GlobeMode の S373（S369 の `libraries.yml` の突き合わせから）。

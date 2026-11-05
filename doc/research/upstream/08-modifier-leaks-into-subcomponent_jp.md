# 下書き（未提出）08: 部品への修正が、中の部品の同名の変数にも漏れる

対象: Rumoca（<https://github.com/CogniPilot/rumoca>）。英語の本文は [08-modifier-leaks-into-subcomponent.md](08-modifier-leaks-into-subcomponent.md)、再現は [.mo](08-modifier-leaks-into-subcomponent.mo)、1 枚の解説は [_jp.html](08-modifier-leaks-into-subcomponent_jp.html)。

判定（GlobeMode、S009。S162 で v0.10.4 で再確認）: **Rumoca のバグ**（修正の範囲）。v0.10.4 でも再現し、flatten 後の属性を直接確かめられた。

## 概要

MSL の `Mechanics.MultiBody.Parts.RollingWheelSet` の形では、部品 `Part` が自分の `theta1(fixed = true)` を決め、内側の `Joint` には `joint.theta1(fixed = false)` を渡す。例題が `Part part(theta1(fixed = true))` と書くと、Rumoca は `part.joint.theta1` と `part.joint.der_theta1` も `fixed = true` にする。外側の修正が 1 段下の同名の変数に漏れる。初期化に冗長な初期条件が 2 つ（別名の両方が fixed）できる。MSL の `RollingWheelSetDriving`・`RollingWheelSetPulling` で、GlobeMode の初期化の対応づけが冗長な条件 7 個を報告し、omc は報告しない。

## 再現手順

```bash
rumoca compile 08-modifier-leaks-into-subcomponent.mo -m ModifierScope --emit flat-mo
```

`part.joint.theta1` と `part.joint.der_theta1` の `fixed` を読む。外側の修正を外す（`Part part;`）と正しい（初期条件 2 個に対し 2 個）。

## 期待

`part.theta1.fixed = true`、`part.joint.theta1.fixed = false`。

## 実際

| Rumoca | `part.joint.theta1` | `part.joint.der_theta1` |
| --- | --- | --- |
| v0.10.2 | `(start = 0, fixed = true)` | `(start = 0, fixed = true)` |
| v0.10.4（S162） | `(start = 0, fixed = true)` | `(start = 0, fixed = true)` |

下書きは GlobeMode の初期化の対応づけで見ただけ（flatten 後の属性の出力は【未確認】）だった。S162 で `--emit flat-mo` の出力で直接確かめた。v0.10.4 は点つきの名前を引用符で囲み（`'part.joint.theta1'`）、`StateSelect.prefer` と小文字になる。これは無関係。

## 影響

MSL `RollingWheelSet` の形【本数未確認】。

## 環境

- Rumoca v0.10.2（rev `276e82bd0fd48960ac201e782918d28bf19cf7ce`）と配布バイナリ。
- 2026-11-05（S162）に最新のリリース Rumoca v0.10.4（`9c8c56fc`、2026-11-02 公開、2026-11-05 でも最新）で再確認。main（`590ac739`）はビルドしていない【未確認】。
- MSL 4.1.0。Linux x86_64（WSL2、Ubuntu）。

## GlobeMode 側の回避

なし。既知の限界として記録する（検証モデル `GlobeModeValidation.Init.ModifierScope` と、直ると落ちる db の試験）。

## 備考

- 下書き 07 と同じ族（修正の範囲）。

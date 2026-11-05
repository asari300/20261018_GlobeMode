# 下書き（未提出）01: インデックス 2 以上の DAE を EL005 `structurally singular` で断る

対象: Rumoca（<https://github.com/CogniPilot/rumoca>）。英語の本文は [01-el005-index2-structurally-singular.md](01-el005-index2-structurally-singular.md)、1 枚の解説は [_jp.html](01-el005-index2-structurally-singular_jp.html)。再現用の `.mo` は無い（MSL の例題と CLI のコマンドで再現する）。

判定（GlobeMode、S011。S162 で v0.10.4 で再確認）: **Rumoca の未対応**（指数の低減が無い。MSL の例題 18 本）。v0.10.4 でも再現する。

## 概要

`rumoca sim` が、指数の低減が要る MSL の例題（微分された代数変数、制約のある機械・電気の閉路）で `[EL005] DAE structural proof failed: structurally singular system` で止まる。最小は `Modelica.Blocks.Examples.FilterWithDifferentiation`（15 本の方程式のうち 14 本が対応）。GlobeMode の MSL の検査（S011）では 18 本がこの誤りで止まり、どれも Rumoca 自身の `msl_quality_baseline.json` の `sim_ok` に入っていない。回帰でなく既知の穴と見える。聞きたいのは、指数の低減（ダミー導関数など）の予定があるか、または対応しなかった変数・方程式を示すメッセージにできるか。

## 再現手順

```bash
rumoca sim "Modelica 4.1.0/package.mo" -m Modelica.Blocks.Examples.FilterWithDifferentiation --source-root <MSL のルート>
```

`.mo` は要らない（MSL の例題）。`--source-root` には `Modelica 4.1.0` を含むフォルダ（GlobeMode では `data/msl`）を渡す。

## 期待

例題が実行できる（通常の MSL の例題。Rumoca 自身の基準にも `sim_ok` としては無い）。

## 実際

| Rumoca | CLI の出力 |
| --- | --- |
| v0.10.2 | `[EL005] DAE structural proof failed: structurally singular system: 14 matched out of 15 equations and 15 unknowns` |
| v0.10.4（S162） | 同じ文（約 7 秒） |

英語の本文の「Actual」にある `[StructuralAnalysis] solve-IR evaluation failed:` の接頭辞は、GlobeMode が使うライブラリ API の道のもの。CLI は付けない。変圧器・誘導機・FluxTubes・Media の 17 本は v0.10.4 で回していない【未確認】。

## 影響

MSL 18 本（S011）。

## 環境

- Rumoca v0.10.2（rev `276e82bd0fd48960ac201e782918d28bf19cf7ce`）と配布バイナリ。
- 2026-11-05（S162）に最新のリリース Rumoca v0.10.4（`9c8c56fc`、2026-11-02 公開、配布バイナリ。2026-11-05 でも最新）で再確認。main（`590ac739`）はビルドしていない【未確認】。
- MSL 4.1.0。Linux x86_64（WSL2、Ubuntu）。

## GlobeMode 側の回避

なし。実行できないものとして報告する（`GM-U0004`）。

## 備考

- 別の EL005 の形（`DAE system is not computable: ... linear solve requires an aggregate function owner`、例 `InverseParameterization`）は別の症状で、この下書きには入れない。
- 提出の順は 8 位（MSL 18 本。指数の低減の予定を尋ねる）。

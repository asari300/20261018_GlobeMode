# 下書き（未提出）16: `external "builtin"` で組み込み関数を呼ぶだけの関数を実行できない（MSL 4.0.0 の `Modelica.Math`）

対象: Rumoca（<https://github.com/CogniPilot/rumoca>）。英語の本文は [16-external-builtin-math-functions.md](16-external-builtin-math-functions.md)、再現は [.mo](16-external-builtin-math-functions.mo)、1 枚の解説は [_jp.html](16-external-builtin-math-functions_jp.html)。

判定（GlobeMode、S067・S068。S162 で v0.10.4 で再確認）: **Rumoca の未対応**（MLS §12.9.1 の組み込み関数）。v0.10.4 でも再現する。メッセージの文面は変わった。

## 概要

本体が `external "builtin" y = sin(u);` の関数は、フロントエンドと DAE（`ExternalLanguage::Builtin`、記号 `sin`）には受け入れられるが、どの実行の道も実行を断る。MSL 4.0.0 は `Modelica.Math` の 13 個の関数（`sin`・`cos`・`tan`・`asin`・`acos`・`atan`・`atan2`・`sinh`・`cosh`・`tanh`・`exp`・`log`・`log10`）をこう書く。4.0.0 のモデルでこれらに届くもの（`Modelica.Blocks.Math.Sin`・MultiBody の frame 関数を含む）は実行できない。MSL 4.1.0 は `algorithm y := .sin(u);` に書き直したので影響しない。DAE には翻訳先の `PureBuiltin`（`Sin`・`Cos` など）が既にある。

## 再現手順

[16-external-builtin-math-functions.mo](16-external-builtin-math-functions.mo)（自己完結。MSL の版に依らない）。

```bash
rumoca sim 16-external-builtin-math-functions.mo -m ExternalBuiltin -o out.csv
```

MSL 本体では、`MODELICAPATH` に MSL 4.0.0 を置いて `Modelica.Mechanics.MultiBody.Examples.Elementary.Pendulum`。

## 期待

モデルが走り、`mySin(u)` が `sin(u)` と等しい。

## 実際

| Rumoca | `ExternalBuiltin`（`mySin`） | MSL 4.0.0 の `Pendulum` |
| --- | --- | --- |
| v0.10.2 | `[EL005] DAE structural proof failed: checked DAE scalar projection failed: external builtin function 'mySin' calls 'sin', which this runtime cannot execute` | `[EL005] ... external builtin function 'Modelica.Math.cos' calls 'cos', which this runtime cannot execute` |
| v0.10.4（S162） | `[EL001] unsupported checked DAE semantics: external builtin function 'mySin' calls 'sin', which the Solve runtime cannot execute` | `[EL001] unsupported checked DAE semantics: external builtin function 'Modelica.Math.cos' calls 'cos', which the Solve runtime cannot execute` |

追記（S073）の「パラメータの束縛の中だと、関数名を名指しせず汎用の契約の誤りになる」は部分的に直った。`final parameter Real c = mycos(til)` の形で、v0.10.4 は原因を示す。

| Rumoca | `InParam` |
| --- | --- |
| v0.10.2 | `[EL005] invalid Solve IR contract: pure-call argument or slot interface is invalid` |
| v0.10.4（S162） | `[EL001] unsupported checked DAE semantics: parameter 'c' depends on other parameters, so its binding must be re-applied after parameter changes, and its expression has no checked initialization-update lowering: external builtin function 'mycos' calls 'cos', which the Solve runtime cannot execute` |

根本の原因（`external "builtin"` を実行しない）は変わらない。追記に挙げた Buildings の例題は v0.10.4 で回していない【未確認】。

## 影響

MSL 4.0.0 で、Buildings・AixLib・IBPSA・DeltaRobot の 70 本がこの理由だけで失敗する（S067）。

## 環境

- Rumoca v0.10.2（rev `276e82bd0fd48960ac201e782918d28bf19cf7ce`）と配布バイナリ。
- 2026-11-05（S162）に最新のリリース Rumoca v0.10.4（`9c8c56fc`、2026-11-02 公開、2026-11-05 でも最新）で再確認。main（`590ac739`）はビルドしていない【未確認】。
- MSL 4.0.0（`Math/package.mo`）。Linux x86_64（WSL2、Ubuntu）。

## GlobeMode 側の回避

あり（S068、[ADR-0029](../../adr/0029-external-builtin-rewrite.md)）。compile の後で、検査済みの `Dae` を直列化の形で書き換える。`external "builtin"` で、記号が上の組み込みのどれかで、引数が入力そのままの順の関数の `call` を、同じ被演算子の `builtin` に置き換えて読み戻す（Rumoca が検査し直す）。直列化の形は安定した API でない。

## 備考

- 直し方は、DAE を作るときに `PureBuiltin` への呼び出しに翻訳するか、数値評価器と lowering が `ExternalLanguage::Builtin` を評価する。
- 提出の順は 6 位（70 本。MSL 4.0.0 だけ。GlobeMode に回避あり）。

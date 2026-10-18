# src/Modelica

検証用のサンプルモデル。MSL 4.1.0 は同梱せず、`data/msl/` に取得する（P1 で取得スクリプトを作る）。

| ファイル | 内容 | 期待する結果 |
| --- | --- | --- |
| `GlobeModeExamples/package.mo` の `RC` | 自前のピン・抵抗・コンデンサ・電源・接地で組んだ RC 回路（connect 4 本） | 検査をすべて通る。方程式 20 本、変数 20 個 |
| `GlobeModeExamples/package.mo` の `Unbalanced` | `y` の方程式が無い | S5（GM-E5002）で止まる |
| `SyntaxError.mo` | `Real x` の後の `;` が無い | S0（GM-E0001）で止まる |

```bash
cargo run -p globe-mode-cli -- outline src/Modelica/GlobeModeExamples/package.mo
cargo run -p globe-mode-cli -- check src/Modelica/GlobeModeExamples/package.mo --model GlobeModeExamples.RC
```

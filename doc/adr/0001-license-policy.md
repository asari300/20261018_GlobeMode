# ADR-0001 ライセンス方針

- 状態: Accepted（2026-10-18、S001）

## 背景

GlobeMode は OSS として公開する。Modelica の周辺には、OpenModelica（OSMC-PL / AGPL）や SuiteSparse の一部（GPL / LGPL）のようなコピーレフトのライセンスが多い。

## 決定

- GlobeMode 自身のライセンスは **MIT OR Apache-2.0** にする。Rust のエコシステムの標準で、Apache-2.0 は特許条項を含む。
- 許可・条件付き・禁止の区分を [licenses.md](../licenses.md) に置く。依存を足すときはこの区分に従う。
- GPL / AGPL / LGPL を静的リンクしない。OpenModelica は別プロセスでの照合用途に限る。
- cargo-deny、cargo-about、`pnpm licenses`、SBOM で機械的に監査する（P1 で導入する）。

## 結果

- KLU や UMFPACK に頼れない。疎行列の LU は faer で行い、SUNDIALS（BSD-3）は任意の feature にする。
- Radau5 は自前で実装する（P8）。
- 寄稿も MIT OR Apache-2.0 で受け入れる。

within GlobeModeValidation.Preflight;
model NotFiniteParameter "パラメータの値が有限にならない（S10 の GM-E10001）"
  // r = sqrt(area) の area が負なので、r は NaN になる。Rumoca はコンパイル（S0〜S8）では止めない
  // （字面の 0 除算 `1 / 0` は、Rumoca が DAE への変換で ED019 として止める）。
  // S10 の GM-E10001 は r に 1 件だけ（r を使う rate は、r の失敗を受け継いだだけなので数えない）。
  parameter Real area = -1 "面積（負にしてしまった）";
  parameter Real r = sqrt(area) "半径の目安";
  parameter Real rate = 2 * r "r を使うパラメータ";
  Real x(start = 0, fixed = true);
equation
  der(x) = rate - x;
  annotation (experiment(StopTime = 1));
end NotFiniteParameter;

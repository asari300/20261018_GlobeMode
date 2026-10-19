within GlobeModeValidation.Simulation;
model Parameters "パラメータの上書き（tunable な k と、k から決まる c。変えられない構造パラメータ・Evaluate・final・定数）"
  // 実行の前に変えられるのは k だけ。k = 2 で回すと x(1) = exp(-2)、c = 4、y = {4, 4} になる。
  parameter Real k = 1 "減衰の速さ（tunable）";
  parameter Real c = 2 * k "k から決まる（k を変えると従う）";
  parameter Integer n = 2 "配列の大きさ（構造パラメータ）";
  parameter Real e = 1 "Evaluate = true（コンパイルで畳み込む）" annotation (Evaluate = true);
  final parameter Real f = 1 "final";
  constant Real g = 1 "定数";
  Real x(start = 1, fixed = true) "状態";
  Real y[n] "c を n 個並べる";
equation
  der(x) = -k * e * f * g * x;
  y = fill(c, n);
  annotation (experiment(StopTime = 1, Tolerance = 1e-8));
end Parameters;

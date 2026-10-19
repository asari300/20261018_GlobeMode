within GlobeModeValidation.Preflight;
model Valid "パラメータと start 値がすべて評価でき、experiment annotation も妥当（S10 を通る）"
  // 1 次遅れ T * der(x) = k - x。T = tau / 2 は別のパラメータ tau から決まり、x の start も k から決まる。
  // パラメータ 3（tau、T、k）と start 2（x、y）をすべて評価でき、失敗は 0 件。
  // experiment は StartTime = 0 < StopTime = 2、Interval = 0.01、Tolerance = 1e-6 で、どれも妥当。
  parameter Real tau = 1 "元の時定数";
  parameter Real T = tau / 2 "時定数";
  parameter Real k = 3 "目標値";
  Real x(start = k / 2, fixed = true);
  Real y "出力";
equation
  T * der(x) = k - x;
  y = 2 * x;
  annotation (experiment(StartTime = 0, StopTime = 2, Interval = 0.01, Tolerance = 1e-6));
end Valid;

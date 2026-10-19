within GlobeModeValidation.Simulation;
model Decay "1 次の減衰 der(x) = -k*x（解析解 x(t) = exp(-k*t)）"
  // 実行系の基本の検証。t = 1 で x = exp(-1) ≈ 0.36788 になる。
  parameter Real k = 1 "減衰の速さ";
  Real x(start = 1, fixed = true) "状態";
equation
  der(x) = -k * x;
  annotation (experiment(StopTime = 1, Tolerance = 1e-8));
end Decay;

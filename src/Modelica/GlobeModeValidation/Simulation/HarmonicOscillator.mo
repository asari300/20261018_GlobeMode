within GlobeModeValidation.Simulation;
model HarmonicOscillator "調和振動子 der(x) = v、der(v) = -w^2*x（解析解 x(t) = cos(w*t)、v(t) = -w*sin(w*t)）"
  // 状態 2 つの線形の ODE。減衰が無いので、誤差が振幅のずれとしてそのまま残る（Decay より厳しい）。
  // w = 2 なので、t = 1 で x = cos(2) ≈ -0.41615、v = -2*sin(2) ≈ -1.81859 になる。
  parameter Real w = 2 "角振動数";
  Real x(start = 1, fixed = true) "位置";
  Real v(start = 0, fixed = true) "速度";
equation
  der(x) = v;
  der(v) = -w ^ 2 * x;
  annotation (experiment(StopTime = 1, Tolerance = 1e-8));
end HarmonicOscillator;

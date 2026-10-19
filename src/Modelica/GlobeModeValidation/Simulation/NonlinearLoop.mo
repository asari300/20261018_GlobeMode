within GlobeModeValidation.Simulation;
model NonlinearLoop "積分の各点で解く非線形の代数ループ（解析解 y = 1 + time、z = 3 + time、x = time + time^2/2）"
  // y と z は 2 本の式に一緒に現れる（大きさ 2 の非線形のループ。S9 は GM-I9001）。解は 2 組あり
  // （(1 + t, 3 + t) と (3 + t, 1 + t)）、差がいつも 2 なので交わらない。start 値で前の組を選ぶ。
  // x は y を積分した状態で、t = 1 で x = 1.5、y = 2、z = 4 になる。
  Real x(start = 0, fixed = true) "y を積分した状態";
  Real y(start = 1) "ループの未知数 1";
  Real z(start = 3) "ループの未知数 2";
equation
  der(x) = y;
  y + z = 4 + 2 * time;
  y * z = 3 + 4 * time + time ^ 2;
  annotation (experiment(StopTime = 1, Tolerance = 1e-8));
end NonlinearLoop;

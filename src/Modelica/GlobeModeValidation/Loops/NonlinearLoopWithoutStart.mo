within GlobeModeValidation.Loops;
model NonlinearLoopWithoutStart "未知数 2 つの非線形の代数ループ。反復変数に start が無い（S9 は警告を出す）"
  // NonlinearLoopWithStart と同じ方程式で、start を与えていない。
  // 反復変数の初めの値は既定の 0 になり、x = 0 では x * y = 1 を満たせない（Jacobian も特異になりうる）。
  // S9 は GM-W9001（非線形のループの反復変数に start が無い）を出す（エラーにはしない）。
  Real x "未知数 1";
  Real y "未知数 2";
equation
  x * y = 1;
  x + y = 3;
  annotation (experiment(StopTime = 1));
end NonlinearLoopWithoutStart;

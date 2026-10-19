within GlobeModeValidation.Loops;
model NonlinearLoopWithStart "未知数 2 つの非線形の代数ループ。反復変数に start がある（S9 は GM-I9001 だけで、GM-W9001 は出さない）"
  // x * y = 1 は x と y の積なので、x と y のどちらについても非線形。x + y = 3 は線形。
  // 2 本は一緒に解くしかない（大きさ 2 のループ）。非線形のループは反復（Newton 法）で解くので、
  // 反復変数の初めの値（start）が要る。ここでは x と y の両方に start を与えている
  // （tearing がどちらを反復変数に選んでも、start がある）。解は x = (3 + sqrt(5)) / 2、y = 1 / x の近く。
  Real x(start = 2.5) "未知数 1";
  Real y(start = 0.5) "未知数 2";
equation
  x * y = 1;
  x + y = 3;
  annotation (experiment(StopTime = 1));
end NonlinearLoopWithStart;

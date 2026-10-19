within GlobeModeValidation.Loops;
model NoLoop "代数ループの無いモデル（方程式を 1 本ずつ順に解ける。S9 は何も言わない）"
  // der(x) = -x で x の微分が、y = 2 * x で y が、z = y ^ 2 で z が順に決まる（ブロックはどれも大きさ 1）。
  // z = y ^ 2 は y について非線形だが、ループではないので反復は要らない（y は先に決まっている）。
  Real x(start = 1, fixed = true) "状態";
  Real y "x から決まる";
  Real z "y から決まる（y について非線形）";
equation
  der(x) = -x;
  y = 2 * x;
  z = y ^ 2;
  annotation (experiment(StopTime = 1));
end NoLoop;

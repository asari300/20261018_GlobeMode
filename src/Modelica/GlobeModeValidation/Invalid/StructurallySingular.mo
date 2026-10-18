within GlobeModeValidation.Invalid;
model StructurallySingular "方程式数と未知数は同じだが、方程式と未知数を 1 対 1 に対応づけられない（構造特異）"
  // 方程式 3 本 = 未知数 3 個（x、y、der(z)）なので S5 は通る。
  // x だけを含む方程式が 2 本あり（過剰）、y と z を 1 本の方程式で決めることになる（不足）。
  // 状態 z と微分 der(z) を 1 つの未知数にまとめても完全マッチングが無い（Hall の条件を満たさない）ので、
  // S6（GM-E6001）で止まる。
  Real x "2 回決めている変数";
  Real y "決める方程式が無い変数";
  Real z(start = 0, fixed = true) "状態";
equation
  x = 1;
  x = 2;
  y + z = der(z);
  annotation (experiment(StopTime = 1));
end StructurallySingular;

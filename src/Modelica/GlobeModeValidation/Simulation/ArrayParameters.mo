within GlobeModeValidation.Simulation;
model ArrayParameters "配列パラメータへの上書き（全体・要素・同名の修飾で展開される配列・時間イベント）"
  // a は直接使う配列、src.k は同名の修飾（final k = k）で内側へ写される配列、t は時間イベントに使う配列。
  // 既定: y_a = 2、y_k = 20、y_t = 0（time < 0.8 で 0、後は 1）。
  block Inner
    parameter Real k[2] = {10, 20};
    Real y = k[2];
    Real s = if time < k[1] then 0 else 1;
  end Inner;

  block Outer
    parameter Real k[2] = {10, 20};
    Inner i(final k = k);
    Real y = i.y;
    Real s = i.s;
  end Outer;

  parameter Real a[3] = {1, 2, 3};
  parameter Real t[2] = {0.8, 0.9};
  Outer src(k = {0.8, 20});
  Real y_a = a[2];
  Real y_t = if time < t[1] then 0 else 1;
  annotation (experiment(StopTime = 1));
end ArrayParameters;

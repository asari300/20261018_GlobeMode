within GlobeModeValidation.Simulation;
model ExternalBuiltin
  "external \"builtin\" で組み込みの数学関数を呼ぶだけの関数（MSL 4.0.0 の Modelica.Math.sin と同じ書き方）"
  // 実行系の検証（S068）。MSL 4.0.0 の Modelica.Math は sin・exp などを `external "builtin"` で書く。
  // 解析解: x(t) = 1 - cos(t)、z(t) = exp(t)、w(t) = atan2(t, 2)。t = 1 で x ≈ 0.45970、z ≈ 2.71828、w ≈ 0.46365。
  function mySin
    input Real u;
    output Real y;
    external "builtin" y = sin(u);
  end mySin;

  function myExp
    input Real u;
    output Real y;
    external "builtin" y = exp(u);
  end myExp;

  function myAtan2
    input Real u1;
    input Real u2;
    output Real y;
    external "builtin" y = atan2(u1, u2);
  end myAtan2;

  Real x(start = 0, fixed = true) "der(x) = sin(time)";
  Real z "exp(time)";
  Real w "atan2(time, 2)（引数が 2 つ）";
equation
  der(x) = mySin(time);
  z = myExp(time);
  w = myAtan2(time, 2);
  annotation (experiment(StopTime = 1, Tolerance = 1e-8));
end ExternalBuiltin;

within GlobeModeValidation.Preflight;
model FreeParameterDependency "初期化で決まるパラメータに依存するパラメータは評価しない（S10 を通る）"
  // u0 は fixed = false（初期化で決まる。start は既定の 0）。k = 1 / u0 を start の 0 で評価すると 0 除算になるが、
  // u0 は初期化で決まるので、S10 は u0 も k も評価しない（偽陽性を出さない）。
  // 初期化: der(x) = 0 から u0 = x = 2、k = 0.5。
  parameter Real T = 0.5 "時定数";
  parameter Real u0(fixed = false) "定常状態になる入力";
  parameter Real k = 1 / u0 "u0 の逆数";
  Real x(start = 2, fixed = true);
  Real y "出力";
initial equation
  der(x) = 0;
equation
  T * der(x) = u0 - x;
  y = k * x;
  annotation (experiment(StopTime = 1));
end FreeParameterDependency;

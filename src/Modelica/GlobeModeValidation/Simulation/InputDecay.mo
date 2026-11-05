within GlobeModeValidation.Simulation;

model InputDecay "入力のある 1 次の系 der(x) = -x + u と出力 y = 2*x（FMU の入力・出力の検証）"

  // u を外から一定値 U で与えると x(t) = U + (1 - U)*exp(-t)。U = 2 なら x(1) = 2 - exp(-1)。
  // 外から与えない実行（globe simulate）は、Rumoca が駆動されない入力として断る（GM-U0004、EX002）。
  input Real u(start = 0) "外から与える入力";
  output Real y "出力";
  Real x(start = 1, fixed = true) "状態";

equation

  der(x) = -x + u;
  y = 2 * x;
annotation(experiment(StopTime = 1, Tolerance = 1e-8));

end InputDecay;

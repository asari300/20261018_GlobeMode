within GlobeModeValidation.Simulation;

model LinearSystem "入力 1・出力 2・状態 2 の線形系（線形化の検証。A・B・C・D が解析的に分かる）"

  // der(x) = A x + B u、y = C x + D u で、A = [-2 1; 1 -2]、B = [1; 2]、C = [1 2; 1 -1]、D = [3; 0]。
  // 代数変数 w を挟む（w = x1 + 2 x2）ので、線形化は代数の解き直しを通る。
  // 入力を外から与えない実行（globe simulate）は、Rumoca が駆動されない入力として断る（GM-U0004）。
  input Real u(start = 0.5) "入力";
  output Real y1 "出力 1 = w + 3 u";
  output Real y2 "出力 2 = x1 - x2";
  Real x1(start = 1, fixed = true) "状態 1";
  Real x2(start = -1, fixed = true) "状態 2";
  Real w "代数変数 = x1 + 2 x2";

equation

  w = x1 + 2 * x2;
  der(x1) = -2 * x1 + x2 + u;
  der(x2) = w - 4 * x2 + 2 * u;
  y1 = w + 3 * u;
  y2 = x1 - x2;
annotation(experiment(StopTime = 1, Tolerance = 1e-8));

end LinearSystem;

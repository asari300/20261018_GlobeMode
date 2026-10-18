within GlobeModeValidation.Init;
model OverDetermined "fixed = true の状態に initial equation も書いた（S8 の GM-E8001 で止まる）"
  // 1 次遅れ T * der(x) = 1 - x。状態は x の 1 つ。
  // x に fixed = true（start = 1）を付けたうえで、initial equation でも x = 2 と決めている。
  // 要る初期条件 1（状態 1）に対して、与えられた初期条件 2（fixed = true の状態 1 + 初期方程式 1）で多すぎるので、
  // S8 の GM-E8001（Error）。位置は initial equation の x = 2 を指す。
  // 方程式 1 = 未知数 1 で、S5〜S7 は通る（初期化系だけの誤り）。
  parameter Real T = 0.1 "時定数";
  Real x(start = 1, fixed = true);
initial equation
  x = 2;
equation
  T * der(x) = 1 - x;
  annotation (experiment(StopTime = 1));
end OverDetermined;

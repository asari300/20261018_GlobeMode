within GlobeModeValidation.Init;
model Balanced "状態 2 つの初期条件が、fixed と initial equation でちょうど足りている（S8 を通る）"
  // ばね・質量・ダンパを部品を使わずに式で書いたもの。状態は位置 x と速度 v の 2 つ。
  // x は fixed = true で start = 1 を初期値にし、v は initial equation（v = 0）で決める。
  // 要る初期条件 2（状態 2）= 与えられた初期条件 2（fixed = true の状態 1 + 初期方程式 1）なので、S8 を通る。
  // パラメータにはすべて値があるので、GM-W8003 も出ない。
  parameter Real m = 1 "質量";
  parameter Real k = 10 "ばね定数";
  parameter Real d = 0.5 "減衰係数";
  Real x(start = 1, fixed = true) "位置";
  Real v "速度";
initial equation
  v = 0;
equation
  der(x) = v;
  m * der(v) = -k * x - d * v;
  annotation (experiment(StopTime = 5));
end Balanced;

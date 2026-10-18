within GlobeModeValidation.Init;
model FreeParameter "初期化で決めるパラメータ（fixed = false）を定常状態の条件で求める（S8 を通る）"
  // 1 次遅れ T * der(x) = u0 - x の入力 u0 を、値を書かないパラメータ（fixed = false）にする。
  // u0 は initial equation の der(x) = 0（定常状態）で決まる。x は fixed = true（start = 2）なので、u0 も 2 になる。
  // 要る初期条件 2（状態 1 + fixed = false のパラメータ 1）= 与えられた初期条件 2
  // （fixed = true の状態 1 + 初期方程式 1）なので、S8 を通る。
  // u0 には値も start も無いが、fixed = false なので GM-W8003 は出ない。
  parameter Real T = 0.5 "時定数";
  parameter Real u0(fixed = false) "定常状態になる入力";
  Real x(start = 2, fixed = true);
initial equation
  der(x) = 0;
equation
  T * der(x) = u0 - x;
  annotation (experiment(StopTime = 1));
end FreeParameter;

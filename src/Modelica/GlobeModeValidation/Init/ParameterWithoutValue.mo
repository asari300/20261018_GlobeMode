within GlobeModeValidation.Init;
model ParameterWithoutValue "値（束縛）の無いパラメータを使う（S8 の警告 GM-W8003。検査は通る）"
  // 1 次遅れ T * der(x) = 1 - x の時定数 T に、値も start も書いていない。
  // fixed は既定の true なので、初期化で決めるパラメータでもない。T の宣言に S8 の GM-W8003（Warning）が 1 件。
  // 初期条件は x の fixed = true の 1 つで足りているので、GM-E8001 / GM-W8001 は出ない。
  // 方程式 1 = 未知数 1 で、S5〜S7 も通る。
  parameter Real T "時定数（値を書き忘れた）";
  Real x(start = 0, fixed = true);
equation
  T * der(x) = 1 - x;
  annotation (experiment(StopTime = 1));
end ParameterWithoutValue;

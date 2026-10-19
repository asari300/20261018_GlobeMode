within GlobeModeValidation.Loops;
model LinearLoop "未知数 2 つの線形の代数ループ（連立 1 次方程式。S9 は線形のループとして通す）"
  // x と y は 2 本の方程式に一緒に現れるので、どちらか一方だけでは解けない（大きさ 2 のループ）。
  // どちらの式も x と y について 1 次（係数は定数かパラメータ）なので、線形のループ。
  // 線形のループは start 値が要らない（連立 1 次方程式として直接解ける）ので、GM-W9001 は出ない（GM-I9001 だけ）。
  parameter Real a = 2 "y の係数";
  Real x "未知数 1";
  Real y "未知数 2";
equation
  x + y = time;
  x - a * y = 1;
  annotation (experiment(StopTime = 1));
end LinearLoop;

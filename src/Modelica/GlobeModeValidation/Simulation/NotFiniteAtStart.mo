within GlobeModeValidation.Simulation;
model NotFiniteAtStart "start 値で評価すると t0 で式の値が無限大になる（S10 は警告 GM-W10002。実行は失敗する）"
  // x は fixed = true で t0 に 0 なので、y = 1 / x は t0 で無限大になる。x の start 値そのものは有限なので、
  // S10 の静的な部分（GM-E10003）は通る。式の値は実行の直前に評価して初めて分かる。
  Real x(start = 0, fixed = true) "t0 で 0 になる状態";
  Real y "x の逆数";
equation
  der(x) = 1;
  y = 1 / x;
  annotation (experiment(StopTime = 1));
end NotFiniteAtStart;

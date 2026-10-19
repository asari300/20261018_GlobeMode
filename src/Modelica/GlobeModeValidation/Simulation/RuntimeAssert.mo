within GlobeModeValidation.Simulation;
model RuntimeAssert "実行中に assert が失敗する（t = 0.5 で x が上限に達する。実行は GM-E11004 で止まる）"
  // 検査（S0〜S10）は通る。式の形は正しく、assert の条件は実行してみないと破れるか分からない。
  // x = time なので、t = 0.5 で条件 x < 0.5 が偽になる。
  parameter Real limit = 0.5 "x の上限";
  Real x(start = 0, fixed = true) "時刻と同じ速さで増える状態";
equation
  der(x) = 1;
  assert(x < limit, "x が上限に達した");
  annotation (experiment(StopTime = 1));
end RuntimeAssert;

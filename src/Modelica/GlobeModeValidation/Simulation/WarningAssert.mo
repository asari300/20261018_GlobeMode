within GlobeModeValidation.Simulation;
model WarningAssert "warning レベルの assert が t = 0.5 で破れる（実行は止まらず、警告 GM-W11001 を付けて成功する）"
  // AssertionLevel.warning の assert は評価を止めない（MLS 8.3.7）。Rumoca は破れた箇所ごとに一度だけ
  // 知らせる（WX001）。RuntimeAssert と同じ条件だが、実行は終了時刻まで進む。
  parameter Real limit = 0.5 "x の上限";
  Real x(start = 0, fixed = true) "時刻と同じ速さで増える状態";
equation
  der(x) = 1;
  assert(x < limit, "x が上限を超えた", AssertionLevel.warning);
  annotation (experiment(StopTime = 1));
end WarningAssert;

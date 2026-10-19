within GlobeModeValidation.Simulation;
model EventNotSettling "イベントの反復が収束しない（t = 0.5 から k が 0 と 1 を行き来する。実行は GM-E11003 で止まる）"
  // k は when の外の式で決まる離散変数。x <= 0.5 の間は k = 0 で pre(k) と一致して落ち着く。
  // t = 0.5 で x > 0.5 が真になると、k = 1 - pre(k) は反復のたびに 0 と 1 を行き来し、決して落ち着かない。
  Real x(start = 0, fixed = true) "時刻と同じ速さで増える状態";
  Integer k(start = 0, fixed = true) "行き来する離散変数";
equation
  der(x) = 1;
  k = if x > 0.5 then 1 - pre(k) else 0;
  annotation (experiment(StopTime = 1));
end EventNotSettling;

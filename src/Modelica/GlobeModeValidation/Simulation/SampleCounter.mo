within GlobeModeValidation.Simulation;
model SampleCounter "sample() の時刻イベント（Integer の数え上げと、連続の状態の標本化）"
  // sample(0.05, 0.1) は t = 0.05, 0.15, ..., 0.95 で真になる（StopTime = 1 までに 10 回）。
  // 区間の端（t = 0 と t = 1）に当たらないようにずらしてあるので、端の扱いで回数が揺れない。
  // xs は最後の標本の時刻の x（= 0.95）を持つ。時刻イベントの位置がずれると、この値がずれる。
  parameter Real period = 0.1 "標本の間隔";
  Real x(start = 0, fixed = true) "時刻と同じ速さで増える状態";
  Integer n(start = 0, fixed = true) "標本の回数";
  discrete Real xs(start = 0, fixed = true) "標本化した x";
equation
  der(x) = 1;
  when sample(0.05, period) then
    n = pre(n) + 1;
    xs = x;
  end when;
  annotation (experiment(StopTime = 1));
end SampleCounter;

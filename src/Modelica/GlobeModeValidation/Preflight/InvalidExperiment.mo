within GlobeModeValidation.Preflight;
model InvalidExperiment "experiment annotation の Tolerance と Interval が正でない（S10 の GM-E10004 が 2 件）"
  // Tolerance = 0 と Interval = -0.1。どちらも正でなければならない。StopTime = 1 は妥当。
  Real x(start = 1, fixed = true);
equation
  der(x) = -x;
  annotation (experiment(StopTime = 1, Interval = -0.1, Tolerance = 0));
end InvalidExperiment;

within GlobeModeValidation.Preflight;
model SuspiciousExperiment "experiment annotation の値が疑わしい（S10 の警告 GM-W10001 が 2 件）"
  // Interval = 5 が区間（0〜1）より長く、出力が 1 点になる。Tolerance = 0.5 は大きすぎる（0.1 以上）。
  // どちらも誤りではないので Warning。
  Real x(start = 1, fixed = true);
equation
  der(x) = -x;
  annotation (experiment(StopTime = 1, Interval = 5, Tolerance = 0.5));
end SuspiciousExperiment;

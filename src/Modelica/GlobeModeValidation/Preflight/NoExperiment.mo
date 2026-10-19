within GlobeModeValidation.Preflight;
model NoExperiment "experiment annotation が無い（S10 の情報 GM-I10001）"
  // 実行のときは、ツールの既定の区間（omc では 0〜1）を使うことになる。
  Real x(start = 1, fixed = true);
equation
  der(x) = -x;
end NoExperiment;

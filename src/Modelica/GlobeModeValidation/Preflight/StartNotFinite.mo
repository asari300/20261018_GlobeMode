within GlobeModeValidation.Preflight;
model StartNotFinite "start 値が有限にならない（S10 の GM-E10003）"
  // x の start は log(x0) で、x0 = 0 なので負の無限大になる（Rumoca はコンパイルでは止めない）。パラメータ x0 そのものは評価できる。
  // S10 の GM-E10003 が x に 1 件。
  parameter Real x0 = 0 "初期値の元";
  Real x(start = log(x0), fixed = true);
equation
  der(x) = -x;
  annotation (experiment(StopTime = 1));
end StartNotFinite;

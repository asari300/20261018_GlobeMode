within GlobeModeValidation.Preflight;
model StopBeforeStart "experiment annotation の StopTime が StartTime より前（S10 の GM-E10004）"
  // StartTime = 1、StopTime = 0.5 で、区間が負。Rumoca はこの StopTime を黙って捨てる
  // （CompilationResult::experiment_stop_time が None になる）ので、annotation を読み直して見つける。
  Real x(start = 1, fixed = true);
equation
  der(x) = -x;
  annotation (experiment(StartTime = 1, StopTime = 0.5));
end StopBeforeStart;

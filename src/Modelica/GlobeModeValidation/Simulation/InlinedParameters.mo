within GlobeModeValidation.Simulation;
model InlinedParameters "同名の修飾で展開される束縛へのパラメータの上書き（構造的な上書きの検証）"
  // MSL の SineVoltage（signalSource(final f = f)）と StepVoltage（startTime）と同じ書き方。Rumoca は
  // インスタンス化で内側の束縛を外側の値に展開して持つので、実行時の上書きは src.startTime だけを変え、
  // 内側の s.startTime（つまり time < startTime の分岐）には届かない。構造的な上書きなら届く。
  // src.startTime = 0.5 で回すと t = 1 で y = 0.5、既定（1）では y = 0。
  block Ramp
    parameter Real startTime = 1 "傾きの始まり";
    Real y = if time < startTime then 0 else time - startTime;
  end Ramp;

  block Source
    parameter Real startTime = 1 "傾きの始まり";
    Ramp r(final startTime = startTime);
    Real y = r.y;
  end Source;

  Source src(startTime = 1);
  parameter Integer n = 2 "Integer のパラメータ";
  parameter Boolean on = true "Boolean のパラメータ";
  Real z = if on then n else 0 "n と on の確認";
  annotation (experiment(StopTime = 1));
end InlinedParameters;

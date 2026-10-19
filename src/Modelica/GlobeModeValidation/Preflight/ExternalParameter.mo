within GlobeModeValidation.Preflight;
model ExternalParameter "外部関数で決まるパラメータ（S10 の情報 GM-I10002）"
  // g = externalSin(1) は C の sin を呼ぶので、静的には評価できない（実行時に決まる）。誤りではない。
  function externalSin "C の sin"
    input Real u;
    output Real y;
  external "C" y = sin(u);
  end externalSin;

  parameter Real g = externalSin(1) "外部関数で決まる係数";
  Real x(start = 0, fixed = true);
equation
  der(x) = g - x;
  annotation (experiment(StopTime = 1));
end ExternalParameter;

within GlobeModeValidation.Simulation;

model EnumString "Enumeration と String の変数を持つ減衰（FMU の Export の型。S224）"

  type Speed = enumeration(slow "遅い", fast "速い");

  parameter Speed speed = Speed.fast "減衰の速さの段";
  parameter String label = "decay" "名札";
  input Speed request = Speed.slow "入力の段";
  output Speed level = speed "出力の段";
  Real x(start = 1, fixed = true) "状態";

equation

  der(x) = -(if speed == Speed.fast then 2 else 1) * x;
annotation(experiment(StopTime = 1, Tolerance = 1e-8));

end EnumString;

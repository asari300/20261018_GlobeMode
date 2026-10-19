within GlobeModeValidation.Simulation;
model NoStates "状態の無いモデル（y = sin(time) だけ。積分器を使わず、時刻を進めて評価する）"
  // 連続の状態が 0 個でも正しいモデル。出力の時刻ごとに式を評価するだけで、t = 1 で y = sin(1) ≈ 0.84147。
  Real y "時刻の関数";
equation
  y = sin(time);
  annotation (experiment(StopTime = 1));
end NoStates;

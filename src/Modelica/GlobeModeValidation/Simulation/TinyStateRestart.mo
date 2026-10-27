within GlobeModeValidation.Simulation;
model TinyStateRestart "ほぼ 0 の状態から大きな導関数で再開する（最初の刻みの見積もりの検証）"
  // t = 1 の時刻イベントで導関数が 0 から 1e5 に跳ぶ。再開で diffsol の最初の刻みの見積もりが
  // 0.01·‖x‖/‖der(x)‖ ≈ 3e-18 になり、下限を割って 1 歩も進まなかった（S023）。t = 2 で x = 1e5。
  Real x(start = 3e-11, fixed = true) "状態";
equation
  der(x) = if time < 1 then 0 else 1e5;
  annotation (experiment(StopTime = 2));
end TinyStateRestart;

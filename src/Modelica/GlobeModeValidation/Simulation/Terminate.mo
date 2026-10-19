within GlobeModeValidation.Simulation;
model Terminate "terminate() で終了時刻の前に止まる（t = 0.5。失敗ではなく、情報 GM-I11001 を付けて成功する）"
  // x = time なので、t = 0.5 で x >= 0.5 が真になり、when の中の terminate() が呼ばれる。
  // 結果の時系列は t = 0.5 で終わる。
  parameter Real limit = 0.5 "止める x の値";
  Real x(start = 0, fixed = true) "時刻と同じ速さで増える状態";
equation
  der(x) = 1;
  when x >= limit then
    terminate("x が limit に達した");
  end when;
  annotation (experiment(StopTime = 1));
end Terminate;

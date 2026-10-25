within GlobeModeValidation.Simulation;
model DiodeSwitching "ダイオードの指数関数に矩形波の電流を流す（切り替えのたびに Newton 法が失敗する。esdirk34 の回帰、S021）"
  // S021 で MSL の Electrical.Analog.Examples（Differentiator・OvervoltageProtection など 7 本）で見つけた不具合の最小の再現。
  // diffsol の SDIRK は Newton 法の失敗を積分全体で累積して数え、既定の 50 回で止まる（BDF は 1 歩の中で数える）。
  // 1 ms ごとに電流の向きが変わり、時定数 R*C = 10 us の硬い過渡の先で、順方向はダイオードの膝（約 0.054 V。
  // 熱電圧を 1/10 にして膝を鋭くしてある）、逆方向は -I0*R = -1 V に落ち着く。膝の鋭さで切り替えのたびに
  // Newton 法が数回失敗し、累計が 50 回を超える（直す前は esdirk34 が t = 0.0027、trbdf2 が t = 5e-6 で止まった）。
  // 終点（t = 0.0985）は順方向の平らな所にあり、イベントの時刻のわずかな差で値が動かない（wasm とネイティブの突き合わせ）。
  parameter Real C = 1e-8 "容量";
  parameter Real R = 1e3 "並列の漏れ抵抗";
  parameter Real Ids = 1e-12 "飽和電流";
  parameter Real Vt = 0.0026 "熱電圧（実物の 1/10。膝を鋭くして Newton 法を失敗させる）";
  parameter Real I0 = 1e-3 "電流源の大きさ";
  parameter Real Maxexp = 40 "指数関数をこの先は直線で延ばす（MSL の Diode と同じ。Newton 法の試しの点で inf にしない）";
  Real v(start = 0, fixed = true) "ダイオードの電圧";
  Real i "電流源（順方向が正）";
  Integer k(start = 0, fixed = true) "切り替えた回数";
equation
  when sample(0.001, 0.001) then
    k = pre(k) + 1;
  end when;
  i = if mod(k, 2) == 0 then I0 else -I0;
  der(v) = (i - Ids * (smooth(1, if v / Vt > Maxexp then exp(Maxexp) * (1 + v / Vt - Maxexp) - 1 else exp(v / Vt) - 1)) - v / R) / C;
  annotation (experiment(StopTime = 0.0985, Interval = 1e-4, Tolerance = 1e-6));
end DiodeSwitching;

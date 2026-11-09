within GlobeModeValidation.Simulation;

model SampleAtZeroCrossing "sample() の時刻と状態事象が同じ時刻に重なる（Math.Mean が整流回路の中で値を失う形）"

  // Modelica.Blocks.Math.Mean と同じ形: x を u で積分し、sample(t0 + Ts, Ts) の時刻に yLast = f*pre(x) を取って x を 0 に戻す。
  // u = if s > 0 then s else 0 は s = sin(2 pi 50 t) が 0 を横切る t = 0.01, 0.02, ... に状態事象を起こし、
  // sample の時刻（0.01 の倍数）と重なる。重なった時刻の 2 回目の事象で when sample の中が回り直すと、
  // yLast が reinit の後の x（ほぼ 0）で書き直される。正しくは半周期ごとの平均 f * 2/(100 pi) = 0.6366 が次の標本（StopTime = 0.015 はその間）まで保たれる。
  parameter Real f = 100 "標本の周波数";
  parameter Real t0(fixed = false) "開始時刻";
  Real s = sin(2 * 3.141592653589793 * 50 * time) "0 を横切る信号";
  Real u = if s > 0 then s else 0 "半波整流";
  Real x(start = 0, fixed = true) "u の積分（標本の時刻に 0 へ戻る）";
  discrete Real yLast(start = 0, fixed = true) "標本の時刻の f*x";

initial equation

  t0 = time;

equation

  der(x) = u;
  when sample(t0 + 0.01, 0.01) then
    yLast = f * pre(x);
    reinit(x, 0);
  end when;
annotation(experiment(StopTime = 0.015));

end SampleAtZeroCrossing;

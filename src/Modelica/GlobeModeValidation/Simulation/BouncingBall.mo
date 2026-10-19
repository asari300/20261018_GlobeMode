within GlobeModeValidation.Simulation;
model BouncingBall "跳ねるボール（when と reinit。状態イベントで速度を反転し、跳ねた回数を数える）"
  // 高さ 1 から落とす。最初に床に着くのは t = sqrt(2/g) ≈ 0.4515 で、そのときの速さ sqrt(2*g) ≈ 4.429 に
  // 反発係数 e を掛けて上へ返す。次は 2*e*sqrt(2*g)/g ≈ 0.7224 秒後（t ≈ 1.174）、その次は t ≈ 1.752 で、
  // 4 回目は t ≈ 2.214 なので、StopTime = 2 までに跳ねるのは 3 回（n = 3）。
  // 跳ねる間隔が縮み続ける（Zeno）手前で止めるので、イベントの時刻はどの積分器でもほぼ同じになる。
  parameter Real e = 0.8 "反発係数";
  parameter Real g = 9.81 "重力加速度";
  Real h(start = 1, fixed = true) "高さ";
  Real v(start = 0, fixed = true) "速度";
  Integer n(start = 0, fixed = true) "跳ねた回数";
equation
  der(h) = v;
  der(v) = -g;
  when h < 0 then
    reinit(v, -e * pre(v));
    n = pre(n) + 1;
  end when;
  annotation (experiment(StopTime = 2));
end BouncingBall;

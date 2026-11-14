within GlobeModeValidation.Simulation;

model Pendulum "減衰のある振り子 der(th) = om、der(om) = -sin(th) - c*om（線形化の検証。A = [0 1; -cos(th) -c]）"

  // 初期の角 th = 0.5 での線形化は A = [0 1; -cos(0.5) -0.5]。th = 0 は平衡点で A = [0 1; -1 -0.5]。
  parameter Real c = 0.5 "減衰";
  parameter Real th0 = 0.5 "初期の角";
  Real th(start = th0, fixed = true) "角";
  Real om(start = 0, fixed = true) "角速度";

equation

  der(th) = om;
  der(om) = -sin(th) - c * om;
annotation(experiment(StopTime = 10, Tolerance = 1e-8));

end Pendulum;

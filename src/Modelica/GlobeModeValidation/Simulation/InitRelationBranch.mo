within GlobeModeValidation.Simulation;
model InitRelationBranch "初期化方程式が、t = 0 で反転する関係式を読む（Rumoca の初期化は関係式の既定値 false の枝で解き、解き直さない。S10 の GM-W10004）"
  // S016 で MSL の EngineV6（CriticalDamping の initType = SteadyState）で見つけた不具合の最小の再現。
  // t = 0 で v = -0.5 < 0 なので u = 1 で、定常（der(x) = 0）なら x = 1。
  // Rumoca は関係式 v < 0 の既定値 false（u = 2）で初期化方程式を解き、x = 2 のまま積分を始める。
  // 初期化の結果を初期化系に入れ直すと x が 1 に動くので、プリフライトが GM-W10004 を出す。
  Real s(start = 0, fixed = true) "位置";
  Real v "速度（t = 0 で負）";
  Real u "関係式の枝で決まる入力（t = 0 では 1）";
  Real x(start = 0) "定常で初期化する 1 次のフィルタの状態（正しくは x(0) = 1）";
  parameter Real T = 0.1 "フィルタの時定数";
equation
  der(s) = v;
  v = -0.5 * cos(s);
  u = if v < 0 then 1 else 2;
  der(x) = (u - x) / T;
initial equation
  der(x) = 0;
  annotation (experiment(StopTime = 1));
end InitRelationBranch;

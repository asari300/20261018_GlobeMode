within GlobeModeValidation.Simulation;
model InitNoSolution "初期化系に実数の解が無い（initial equation の x^2 = -1。実行は GM-E11001 で止まる）"
  // 初期条件の数は合う（fixed = false の状態 x 1 つに、初期方程式 1 本。S8 は 1 / 1）。
  // 構造の検査では分からず、初期化の Newton 法が収束しない。
  Real x(start = 1) "状態";
equation
  der(x) = -x;
initial equation
  x * x = -1;
  annotation (experiment(StopTime = 1));
end InitNoSolution;

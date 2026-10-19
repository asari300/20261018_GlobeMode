within GlobeModeValidation.Preflight;
model FunctionAssertion "パラメータの評価で、関数の中の assert が失敗する（S10 の GM-E10002）"
  // 関数 checkedSqrt は、負の引数を assert で拒む。d = checkedSqrt(area) の area が負なので、
  // d を評価すると assert が失敗する。S10 の GM-E10002 が d に 1 件。
  function checkedSqrt "負の数を拒む平方根"
    input Real v;
    output Real r;
  algorithm
    assert(v >= 0, "checkedSqrt: 引数が負");
    r := sqrt(v);
  end checkedSqrt;

  parameter Real area = -1 "面積（負にしてしまった）";
  parameter Real d = checkedSqrt(area) "直径の目安";
  Real x(start = 0, fixed = true);
equation
  der(x) = d - x;
  annotation (experiment(StopTime = 1));
end FunctionAssertion;

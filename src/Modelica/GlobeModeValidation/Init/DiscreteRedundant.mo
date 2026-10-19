within GlobeModeValidation.Init;
model DiscreteRedundant "when の中で決まる離散変数に fixed = true と初期方程式の両方がある（S8 は確かな過剰。Error）"
  // n は when の中だけで決まるので、初期化では n = pre(n)。fixed = true が pre(n) = 0 を、initial equation が
  // n = 0 を与え、どちらも同じ未知数を決める（初期条件が 1 個多い）。
  //
  // S009 の S8 は離散変数を含む過剰を確定させず GM-W8004 にしていた。S010 は n が when の中で決まると分かるので、
  // 連続の状態の Init.OverDetermined と同じく GM-E8001 にする（冗長の候補は initial equation n = 0 と n の fixed = true）。
  // omc 1.27.1（-d=initialization）は「The initial conditions are over specified. The following 1 initial
  // equations are redundant, so they are removed from the initialization system: $PRE.n = 0.」と言う
  // （値が整合するので外して進む。連続の状態の重複でも同じ言い方）。
  Real x(start = 1, fixed = true);
  Integer n(start = 0, fixed = true) "when の中で数える";
initial equation
  n = 0;
equation
  der(x) = -x;
  when x < 0.5 then
    n = pre(n) + 1;
  end when;
end DiscreteRedundant;

within GlobeModeValidation.Init;
model DiscreteWhenInitial "when initial() の枝で決まる離散変数は初期化で決まる（S8 は合う）"
  // when initial() then の枝は初期化で働く（MLS 3.6 §8.6）ので、m = 3 と d = 2 * x が初期化の方程式になり、
  // m と d の初期条件は要らない。elsewhen の枝（pre(m) を読む）は初期化では働かない。
  // pre(m) と pre(d) は初期化で働く式に読まれないので、未知数にならない。
  //
  // omc 1.27.1（-d=initialization）も初期化の警告を出さない。m(start = 1, fixed = true) を足しても、
  // fixed = true は pre(m) = 1 を与えるだけで m = 3 と重ならない（omc も警告なし。S8 も合う）。
  Real x(start = 1, fixed = true);
  Integer m "初期化で 3 にし、x が 0.2 を切るたびに 1 増やす";
  discrete Real d "初期化で 2 * x にし、x が 0.2 を切ったときの x を持つ";
equation
  der(x) = -x;
  when initial() then
    m = 3;
    d = 2 * x;
  elsewhen x < 0.2 then
    m = pre(m) + 1;
    d = x;
  end when;
end DiscreteWhenInitial;

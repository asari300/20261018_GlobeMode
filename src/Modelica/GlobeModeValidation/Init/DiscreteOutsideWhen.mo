within GlobeModeValidation.Init;
model DiscreteOutsideWhen "when の外の式で決まる離散変数には初期条件が要らない（S8 は合う）"
  // b と k は when の外の方程式で決まり、その方程式は初期化でも成り立つ（MLS 3.6 §8.6）ので、初期条件は要らない。
  // c は when b の中で数えるので、初期化では c = pre(c) が加わり、initial equation c = 0 がそれを決める。
  // when b then の条件は b を読むだけで pre(b) を読まない（edge(b) は pre(b) を読む。Init.DiscreteEdge）。
  //
  // S009 の S8 は離散変数を区別しなかったので、b と k の分を「2 個足りない見込み」としていた。
  // S010 は初期化でも成り立つ離散の方程式を行にするので、過剰も不足も無い。
  // omc 1.27.1（-d=initialization）も初期化の警告を出さない。
  Real x(start = 1, fixed = true);
  Boolean b = x > 0.5 "when の外の式で決まる";
  Integer k = integer(3 * x) "when の外の式で決まる";
  Integer c "when の中で数える";
initial equation
  c = 0;
equation
  der(x) = -x;
  when b then
    c = pre(c) + 1;
  end when;
end DiscreteOutsideWhen;

within GlobeModeValidation.Init;
model DiscreteWhen "when の中だけで決まる離散変数に初期条件が無い（S8 は離散変数の不足を確定させる。警告だけで通る）"
  // when x < 0.5 は initial() を含まないので、初期化では働かない。MLS 3.6 §8.6 により n = pre(n) と
  // d = pre(d) が加わり、n と d は pre(n)、pre(d) と同じ値になる。それを決める初期条件（fixed = true か
  // initial equation）が無いので、初期条件が 2 個足りない。
  //
  // S010 の S8 は離散変数の決まり方を区別して、この不足を確定させる（GM-W8001。柔らかい「足りない見込み」ではない）。
  // 決まらない未知数は pre(n) と pre(d)。状態 x は fixed = true なので GM-W8002 は出ない。
  // omc 1.27.1 の translateModel（-d=initialization）は「Assuming fixed start value for the following 2 variables:
  // d:DISCRETE(fixed = true) type: Real、n:DISCRETE(fixed = true) type: Integer」と言う（既定の start を使う）。
  Real x(start = 1, fixed = true);
  Integer n "when の中で数える（start も fixed も無い）";
  discrete Real d "when の中で標本化する（start も fixed も無い）";
equation
  der(x) = -x;
  when x < 0.5 then
    n = pre(n) + 1;
    d = x;
  end when;
end DiscreteWhen;

within GlobeModeValidation.Init;
model RedundantAndMissing "初期条件の数は合うが、fixed = true が拘束と重なり、別の状態には初期条件が無い（S8 のマッチングの検証。警告だけで通る）"
  // 状態 x と y、代数変数 u を部品を使わずに式で書いたもの（MSL の Fluid.Examples.NonCircularPipes の形。
  // 配管の最初の体積の圧力 x が、境界の圧力 pb と拘束 x = pb で等しい）。
  //
  // 拘束 x = pb は状態だけの式なので、S7 の Pantelides 法が 1 回微分して der(x) = 0 にする（構造インデックス 2、
  // 動的な状態は y の 1 つ）。要る初期条件 1 に対して、与えた初期条件は x の fixed = true の 1 つで、数は合う。
  // だが x は拘束で決まるので fixed = true は冗長で、y には初期条件が無い。
  //
  // S009 の S8 は初期化系をマッチングして、両方を名指しする:
  // - GM-W8004（Warning）: x の fixed = true が拘束と同じ変数を決めている（整合する値なら冗長なだけ。start = 1 = pb）
  // - GM-W8001（Warning）: y に初期条件が無い。y には start も無いので GM-W8002 も出る
  // 数と属性だけ（S007 の S8）では 1 / 1 で合うとしか分からなかった。
  // omc 1.27.1 の translateModel は「Assuming fixed start value for the following 1 variables: y」と言う
  // （y の不足を start 値で埋めた）。x は拘束で pb の別名になり、start = 1 = pb で整合するので警告は出ない
  // （MSL の Fluid.Examples.NonCircularPipes では境界の圧力がコンパイル時に評価できず、「over specified」の警告が出る）。
  parameter Real pb = 1 "境界の圧力（x を縛る）";
  Real x(start = 1, fixed = true) "拘束で決まる状態（fixed = true は冗長）";
  Real y "初期条件の無い状態";
  Real u "x の微分を釣り合わせる代数変数";
equation
  der(x) = u - y;
  x = pb;
  der(y) = y - x;
end RedundantAndMissing;

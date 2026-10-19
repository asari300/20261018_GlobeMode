within GlobeModeValidation.Init;
model DiscreteEdge "when の外で決まる離散変数でも、edge(b) が読む pre(b) には初期条件が要る（S8 は 1 個足りない。警告だけで通る）"
  // b は when の外の式で決まる（初期化でも成り立つ）が、when edge(b) の条件 b and not pre(b) が pre(b) を読む。
  // MLS 3.6 §8.6 は when の外で決まる離散変数に pre(b) = b を足さないので、pre(b) は b と別の未知数で、
  // それを決める初期条件が無い。c の pre(c) は initial equation c = 0 で決まる（when の中なので c = pre(c)）。
  //
  // S010 の S8 は pre(b) を別の列にして、「pre(b) が決まらない」と名指しする（GM-W8001）。
  // omc 1.27.1（-d=initialization）も「Assuming fixed start value for the following 1 variables:
  // b:DISCRETE(fixed = true) type: Boolean」と言う。b(start = false, fixed = true) にすれば pre(b) = false で合う。
  Real x(start = 1, fixed = true);
  Boolean b = x > 0.5 "when の外の式で決まる";
  Integer c "b の立ち上がりで数える";
initial equation
  c = 0;
equation
  der(x) = -x;
  when edge(b) then
    c = pre(c) + 1;
  end when;
end DiscreteEdge;

within GlobeModeValidation.Init;
model DiscretePreInitial "when の外の式が pre(y) を読み、initial equation pre(y) = … がそれを決める（S8 は合う）"
  // MSL の Blocks.Logical.Hysteresis と同じ形: y は when の外の式で決まり（初期化でも成り立つ）、その式が
  // pre(y) を読む。pre(y) は y と別の未知数で、initial equation pre(y) = false が決める。
  //
  // Rumoca v0.10.2 は initial equation pre(y) = false を y の初期化の時点の値の定義として取り分ける
  // （pre の区別を持たない）。フロントエンドは、その本文が pre( で始まるので pre(y) を決める式として扱う。
  // omc 1.27.1（-d=initialization）も初期化の警告を出さない。
  Real x(start = 1, fixed = true);
  Boolean y "x が 0.8 を超えると true、0.2 を切ると false（その間は前の値）";
initial equation
  pre(y) = false;
equation
  der(x) = -x;
  y = x > 0.8 or pre(y) and x > 0.2;
end DiscretePreInitial;

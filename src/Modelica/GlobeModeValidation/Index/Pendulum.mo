within GlobeModeValidation.Index;
model Pendulum "直交座標の平面の振り子（インデックス 3。S7 の Pantelides 法の検証）"
  // 質量 1 のおもりを、長さ L の糸で原点に吊るした振り子。部品を使わずに式で書いたもの。
  // 状態は x, y, vx, vy の 4 つ、代数変数は lambda（糸の張力を質量と長さで割ったもの）の 1 つ。
  //
  // 拘束 x^2 + y^2 = L^2 には lambda が現れないので、der の形（状態を既知とみなす形）では
  // どの未知数とも対応づかない。Pantelides 法は、拘束を 2 回、der(x) = vx と der(y) = vy を
  // 1 回ずつ微分して、構造インデックス 3 と見積もる（GM-I7001）。微分した方程式に x, y, vx, vy の
  // 最高階の微分が現れるので、この 4 つが状態選択の候補になる（GM-I7002）。
  //
  // 初期条件（S8）: 拘束 x^2 + y^2 = L^2 が位置を、その微分 x*vx + y*vy = 0 が速度を 1 つずつ縛るので、
  // 自由に決められるのは位置 1 つと速度 1 つ（動的な状態の見込みは 2）。そこで fixed は x と vx の 2 つにし、
  // y と vy は拘束から決める（start は推定値。y = -0.8 で、原点より下の解を選ぶ）。
  // x = L（糸が水平）で x を固定すると、∂(x^2 + y^2)/∂y = 2y = 0 で y が決まらない（初期化の
  // ヤコビ行列が特異になる）ので、x = 0.6 から静かに放す形にした。
  parameter Real L = 1 "糸の長さ";
  parameter Real g = 9.81 "重力加速度";
  Real x(start = 0.6, fixed = true) "おもりの x 座標";
  Real y(start = -0.8) "おもりの y 座標（拘束から決める）";
  Real vx(start = 0, fixed = true) "x 方向の速度";
  Real vy(start = 0) "y 方向の速度（拘束の微分から決める）";
  Real lambda "糸の張力 / (質量 × 長さ)";
equation
  der(x) = vx;
  der(y) = vy;
  der(vx) = lambda * x;
  der(vy) = lambda * y - g;
  x ^ 2 + y ^ 2 = L ^ 2;
  annotation (experiment(StopTime = 5));
end Pendulum;

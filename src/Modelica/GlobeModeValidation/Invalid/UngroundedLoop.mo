within GlobeModeValidation.Invalid;
model UngroundedLoop "接地の無い回路の部品を単体で検査する（数値的には特異だが、構造では見えない。S6 は通る）"
  // 2 本の抵抗を並列にして、外に出したピン p と n につないだ部品。接地（Ground）は無い。
  // 単体で検査すると、未接続のピンの flow は 0（p.i = 0、n.i = 0）になる。
  //
  // 方程式 10 = 未知数 10 で、x と der(x) をまとめた二部グラフには完全マッチングがある（S6 は通る）。
  // だが、p のキルヒホッフの電流則と n のそれは、2 本の抵抗の電流の保存（p.i + n.i = 0）を通して
  // 数値的に従属で、p.v と n.v は差しか決まらない（基準の電位が無い）。ヤコビ行列は特異になる。
  // 二部グラフ（どの方程式がどの未知数を含むか）だけでは、この従属は見えない。
  //   ・枝（抵抗）が 2 本、節点が 2 つで、枝 >= 節点 なら構造的には完全マッチングがある
  //   ・抵抗が 1 本（枝 1 < 節点 2）なら構造でも特異で、S6 の GM-E6001 で止まる
  // MSL の Magnetic.FluxTubes.Examples.SolenoidActuator.Components.AdvancedSolenoid（S008 で omc と食い違った 1 本）
  // はこの形で、globe は S6 を通し、omc 1.27.1 は記号的な簡約の後に「0.0 = 0.0」の方程式を見つけて
  // 「Model is structurally singular」で止まる（S009 で確かめた）。
  // 数値の検査（S10 のプリフライトで初期化のヤコビ行列の階数を見る）が候補。
  Connectors.Pin p "外に出したピン";
  Connectors.Pin n "外に出したピン";
  Components.Resistor r1(R = 1);
  Components.Resistor r2(R = 2);
equation
  connect(p, r1.p);
  connect(p, r2.p);
  connect(r1.n, n);
  connect(r2.n, n);
end UngroundedLoop;

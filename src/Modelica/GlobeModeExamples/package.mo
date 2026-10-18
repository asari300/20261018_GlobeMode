package GlobeModeExamples "GlobeMode の検証用サンプル（MSL を使わない）"
  connector Pin "電気のピン"
    Real v "電位";
    flow Real i "流れ込む電流";
  end Pin;

  partial model TwoPin "2 端子の部品"
    Pin p, n;
    Real v "端子間の電圧";
    Real i "p から n へ流れる電流";
  equation
    v = p.v - n.v;
    0 = p.i + n.i;
    i = p.i;
  end TwoPin;

  model Resistor
    extends TwoPin;
    parameter Real R = 1 "抵抗";
  equation
    v = R * i;
  end Resistor;

  model Capacitor
    extends TwoPin;
    parameter Real C = 1 "静電容量";
  equation
    C * der(v) = i;
  end Capacitor;

  model ConstantVoltage
    extends TwoPin;
    parameter Real V = 1 "電圧";
  equation
    v = V;
  end ConstantVoltage;

  model Ground
    Pin p;
  equation
    p.v = 0;
  end Ground;

  model RC "RC 回路。検査をすべて通る"
    Resistor r(R = 100);
    Capacitor c(C = 1e-3);
    ConstantVoltage src(V = 10);
    Ground gnd;
  equation
    connect(src.p, r.p);
    connect(r.n, c.p);
    connect(c.n, src.n);
    connect(src.n, gnd.p);
    annotation(experiment(StopTime = 1));
  end RC;

  model Unbalanced "y の方程式が無い。S5（方程式数と変数数）で止まる"
    Real x(start = 1);
    Real y;
  equation
    der(x) = -x;
  end Unbalanced;
end GlobeModeExamples;

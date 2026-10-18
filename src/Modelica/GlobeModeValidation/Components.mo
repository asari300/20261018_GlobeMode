within GlobeModeValidation;
package Components "テスト用の部品"
  partial model TwoPin "2 端子の部品"
    Connectors.Pin p;
    Connectors.Pin n;
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
    Connectors.Pin p;
  equation
    p.v = 0;
  end Ground;

  block RealConstant
    parameter Real k = 1;
    Connectors.RealOutput y;
  equation
    y = k;
  end RealConstant;

  block RealGain
    parameter Real k = 1;
    Connectors.RealInput u;
    Connectors.RealOutput y;
  equation
    y = k * u;
  end RealGain;

  block BooleanStep "startTime 以降に true"
    parameter Real startTime = 0.5;
    Connectors.BooleanOutput y;
  equation
    y = time >= startTime;
  end BooleanStep;

  block BooleanNot
    Connectors.BooleanInput u;
    Connectors.BooleanOutput y;
  equation
    y = not u;
  end BooleanNot;

  block Switch "on なら u、そうでなければ 0"
    Connectors.BooleanInput on;
    Connectors.RealInput u;
    Connectors.RealOutput y;
  equation
    y = if on then u else 0;
  end Switch;

  model FluidBoundary "圧力と比エンタルピーを固定した境界"
    replaceable package Medium = Modelica.Media.Interfaces.PartialMedium;
    parameter Medium.AbsolutePressure p = 1e5;
    parameter Medium.SpecificEnthalpy h = 1e5;
    Connectors.FluidPort port(redeclare package Medium = Medium);
  equation
    port.p = p;
    port.h_outflow = h;
  end FluidBoundary;

  model FluidResistance "圧力差に比例する流量"
    replaceable package Medium = Modelica.Media.Interfaces.PartialMedium;
    parameter Real k = 1e-3 "流量係数 [kg/(s.Pa)]";
    Connectors.FluidPort a(redeclare package Medium = Medium);
    Connectors.FluidPort b(redeclare package Medium = Medium);
  equation
    a.m_flow = k * (a.p - b.p);
    a.m_flow + b.m_flow = 0;
    a.h_outflow = inStream(b.h_outflow);
    b.h_outflow = inStream(a.h_outflow);
  end FluidResistance;
end Components;

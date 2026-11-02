within ;
model MslPartsImport "MslParts の import 版（S089。qualified・renamed・unqualified の 3 形）"
  import Modelica.Electrical.Analog.Basic.Resistor;
  import Sources = Modelica.Electrical.Analog.Sources;
  import Modelica.Electrical.Analog.Basic.*;
  Sources.ConstantVoltage V(V = 1);
  Resistor R1(R = 1);
  Ground G;
equation
  connect(V.p, R1.p);
  connect(R1.n, V.n);
  connect(V.n, G.p);
end MslPartsImport;

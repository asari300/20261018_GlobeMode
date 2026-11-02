within ;
model MslParts "MSL の部品を使うワークスペースのモデル（S081。connect グラフがライブラリの部品を引く）"
  Modelica.Electrical.Analog.Sources.ConstantVoltage V(V = 1);
  Modelica.Electrical.Analog.Basic.Resistor R1(R = 1);
  Modelica.Electrical.Analog.Basic.Ground G;
equation
  connect(V.p, R1.p);
  connect(R1.n, V.n);
  connect(V.n, G.p);
end MslParts;

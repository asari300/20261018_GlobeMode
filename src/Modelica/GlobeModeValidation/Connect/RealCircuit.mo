within GlobeModeValidation.Connect;
model RealCircuit "Real の potential / flow の connect（RC 回路）"
  Components.ConstantVoltage source(V = 10);
  Components.Resistor resistor(R = 100);
  Components.Capacitor capacitor(C = 1e-3);
  Components.Ground ground;
equation
  connect(source.p, resistor.p);
  connect(resistor.n, capacitor.p);
  connect(capacitor.n, source.n);
  connect(source.n, ground.p);
  annotation (experiment(StopTime = 1));
end RealCircuit;

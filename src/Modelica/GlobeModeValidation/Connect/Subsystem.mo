within GlobeModeValidation.Connect;
model Subsystem "外に 2 つのピンを出す RC の部分回路（階層のテスト用）"
  Connectors.Pin p;
  Connectors.Pin n;
  Components.Resistor resistor(R = 10);
  Components.Capacitor capacitor(C = 1e-3);
equation
  connect(p, resistor.p);
  connect(resistor.n, capacitor.p);
  connect(capacitor.n, n);
end Subsystem;

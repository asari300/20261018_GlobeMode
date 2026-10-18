within GlobeModeValidation.Connect;
model MediumFlow "Medium を持つ流体ポートの connect（flow と stream）。MSL 4.1.0 が要る"
  package Medium = Modelica.Media.Water.ConstantPropertyLiquidWater "水（物性一定）";
  Components.FluidBoundary source(redeclare package Medium = Medium, p = 2e5, h = 1e5);
  Components.FluidResistance pipe(redeclare package Medium = Medium, k = 1e-3);
  Components.FluidBoundary sink(redeclare package Medium = Medium, p = 1e5, h = 2e5);
equation
  connect(source.port, pipe.a);
  connect(pipe.b, sink.port);
  annotation (experiment(StopTime = 1));
end MediumFlow;

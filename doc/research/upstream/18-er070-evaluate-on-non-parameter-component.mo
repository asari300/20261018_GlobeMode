// Repro for upstream draft 18. Needs only the Modelica Standard Library.
// Shape taken from IBPSA.Controls.Continuous.LimPID (Dzero).
model Rep18

  parameter Boolean with_D = false;
  Modelica.Blocks.Sources.Constant Dzero(final k = 0) if not with_D "Zero input signal" annotation(
    Evaluate = true
  );
  Real y(start = 0);

equation

  der(y) = -y + 1;

end Rep18;

model Rep18Plain

  // Control: the same component without the Evaluate annotation.
  parameter Boolean with_D = false;
  Modelica.Blocks.Sources.Constant Dzero(final k = 0) if not with_D "Zero input signal";
  Real y(start = 0);

equation

  der(y) = -y + 1;

end Rep18Plain;

// Reduction of draft 16. MSL 4.0.0 writes Modelica.Math.sin, cos, exp, log, ... as `external "builtin" y = sin(u);`.
// Expected: x(1) = 1 - cos(1) = 0.459697... Actual: the solve-IR lowering stops with
// "[EL005] ... external builtin function `mySin` calls `sin`, which this runtime cannot execute".
model ExternalBuiltin
  function mySin
    input Real u;
    output Real y;
    external "builtin" y = sin(u);
  end mySin;

  Real x(start = 0, fixed = true);
equation
  der(x) = mySin(time);
  annotation (experiment(StopTime = 1));
end ExternalBuiltin;

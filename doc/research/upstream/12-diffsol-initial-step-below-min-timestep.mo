// Reduction of diffsol issue draft 12 (GlobeMode GlobeModeValidation.Simulation.TinyStateRestart).
// At the time event t = 1 the derivative jumps from 0 to 1e5 while x is about 3e-11.
// The restart's initial step estimate 0.01*|x|/|der(x)| is about 3e-18, below min_timestep (about 1e-13 near t = 1),
// so the first step() fails with "Step size is too small" without trying a step. Expected x(2) = 1e5.
model TinyStateRestart
  Real x(start = 3e-11, fixed = true) "state";
equation
  der(x) = if time < 1 then 0 else 1e5;
  annotation (experiment(StopTime = 2));
end TinyStateRestart;

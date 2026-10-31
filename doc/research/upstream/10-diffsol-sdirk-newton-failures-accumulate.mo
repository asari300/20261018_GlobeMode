// Reduction of diffsol issue draft 10 (GlobeMode GlobeModeValidation.Simulation.DiodeSwitching).
// A current of +-1 mA flips every 1 ms into a diode with a sharp knee (thermal voltage 2.6 mV)
// and a leakage resistor. Each flip makes the Newton iteration fail a few times; after about 50
// failures in total, SDIRK stops although every step recovers. BDF counts failures per step and runs on.
model DiodeSwitching
  parameter Real C = 1e-8 "capacitance";
  parameter Real R = 1e3 "leakage resistance";
  parameter Real Ids = 1e-12 "saturation current";
  parameter Real Vt = 0.0026 "thermal voltage (1/10 of the real one: sharper knee)";
  parameter Real I0 = 1e-3 "source current";
  parameter Real Maxexp = 40 "continue the exponential linearly beyond this (as the MSL Diode)";
  Real v(start = 0, fixed = true) "diode voltage";
  Real i "source current (forward positive)";
  Integer k(start = 0, fixed = true) "number of flips";
equation
  when sample(0.001, 0.001) then
    k = pre(k) + 1;
  end when;
  i = if mod(k, 2) == 0 then I0 else -I0;
  der(v) = (i - Ids * (smooth(1, if v / Vt > Maxexp then exp(Maxexp) * (1 + v / Vt - Maxexp) - 1 else exp(v / Vt) - 1)) - v / R) / C;
  annotation (experiment(StopTime = 0.0985, Interval = 1e-4, Tolerance = 1e-6));
end DiodeSwitching;

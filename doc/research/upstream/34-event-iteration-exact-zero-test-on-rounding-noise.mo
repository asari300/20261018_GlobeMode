// Draft 34: the MSL example whose event iteration cycles with GlobeMode's sdirk34hw at t = 13.04.
// Only the experiment settings are fixed here (the example's own StopTime is 15; Tolerance 1e-7 is the 0.1 scale of its 1e-6).
// Needs MSL 4.1.0: globe simulate -L data/msl 34-event-iteration-exact-zero-test-on-rounding-noise.mo -m Repro34 --solver sdirk34hw
model Repro34

  extends Modelica.Electrical.PowerConverters.Examples.ACDC.RectifierBridge2Pulse.ThyristorBridge2Pulse_DC_Drive;
annotation(experiment(StopTime = 15, Interval = 0.0002, Tolerance = 1e-7));

end Repro34;

// Reduction of Rumoca issue draft 05. Same pattern as MSL SineVoltage / StepVoltage:
// the inner block receives startTime through a same-name modifier (final startTime = startTime).
// A runtime override of src.startTime = 0.5 should move the ramp start; with Rumoca v0.10.2
// only src.startTime changes and the inner r.startTime stays 1 (y(1) = 0 instead of 0.5).
model InlinedParameters
  block Ramp
    parameter Real startTime = 1 "start of the ramp";
    Real y = if time < startTime then 0 else time - startTime;
  end Ramp;

  block Source
    parameter Real startTime = 1 "start of the ramp";
    Ramp r(final startTime = startTime);
    Real y = r.y;
  end Source;

  Source src(startTime = 1);
  annotation (experiment(StopTime = 1));
end InlinedParameters;

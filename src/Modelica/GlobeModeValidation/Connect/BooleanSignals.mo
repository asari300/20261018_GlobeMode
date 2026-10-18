within GlobeModeValidation.Connect;
model BooleanSignals "Boolean の信号の connect と、Real との混在"
  Components.BooleanStep step(startTime = 0.5);
  Components.BooleanNot not1;
  Components.RealConstant constant1(k = 1);
  Components.Switch switch1;
equation
  connect(step.y, not1.u);
  connect(not1.y, switch1.on);
  connect(constant1.y, switch1.u);
  annotation (experiment(StopTime = 1));
end BooleanSignals;

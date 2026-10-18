within GlobeModeValidation.Connect;
model RealSignals "Real の信号の connect（output → input）"
  Components.RealConstant constant1(k = 2);
  Components.RealGain gain1(k = 3);
  Components.RealGain gain2(k = 0.5);
equation
  connect(constant1.y, gain1.u);
  connect(gain1.y, gain2.u);
  annotation (experiment(StopTime = 1));
end RealSignals;

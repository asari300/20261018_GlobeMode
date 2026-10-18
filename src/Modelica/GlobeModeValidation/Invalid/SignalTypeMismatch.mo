within GlobeModeValidation.Invalid;
model SignalTypeMismatch "Real の出力を Boolean の入力につないでいる（型の誤り）"
  Components.RealConstant constant1;
  Components.BooleanNot not1;
equation
  connect(constant1.y, not1.u);
end SignalTypeMismatch;

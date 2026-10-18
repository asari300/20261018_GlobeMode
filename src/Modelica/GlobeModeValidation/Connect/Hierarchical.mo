within GlobeModeValidation.Connect;
model Hierarchical "部分回路を 2 つ直列につないだ階層モデル（depth のテスト）"
  Components.ConstantVoltage source(V = 5);
  Subsystem stage1;
  Subsystem stage2;
  Components.Ground ground;
equation
  connect(source.p, stage1.p);
  connect(stage1.n, stage2.p);
  connect(stage2.n, source.n);
  connect(source.n, ground.p);
  annotation (experiment(StopTime = 1));
end Hierarchical;

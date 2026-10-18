within GlobeModeValidation;
package Connectors "テスト用のコネクタ"
  connector Pin "Real の potential（v）と flow（i）"
    Real v "電位";
    flow Real i "流れ込む電流";
  end Pin;

  connector RealInput = input Real "Real の入力信号";
  connector RealOutput = output Real "Real の出力信号";
  connector BooleanInput = input Boolean "Boolean の入力信号";
  connector BooleanOutput = output Boolean "Boolean の出力信号";

  connector FluidPort "Medium を持つ流体ポート（flow と stream）"
    replaceable package Medium = Modelica.Media.Interfaces.PartialMedium "流体の物性";
    flow Medium.MassFlowRate m_flow "流れ込む質量流量";
    Medium.AbsolutePressure p "圧力";
    stream Medium.SpecificEnthalpy h_outflow "流れ出すときの比エンタルピー";
  end FluidPort;
end Connectors;

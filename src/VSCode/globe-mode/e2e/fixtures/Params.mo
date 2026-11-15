// パラメータ付きの部品（S306）。図の部品のダブルクリックで R・mode を修飾として書く
model Params
  type Mode = enumeration(low, high);

  model Res "抵抗"
    parameter Real R = 1 "抵抗値";
    parameter Mode mode = Mode.low;
    annotation(Icon(graphics = {Rectangle(extent = {{-100, -100}, {100, 100}})}));
  end Res;

  Res r1 annotation(Placement(transformation(origin = {0, 0}, extent = {{-25, -25}, {25, 25}})));
  annotation(Diagram(coordinateSystem(extent = {{-100, -100}, {100, 100}})));
end Params;

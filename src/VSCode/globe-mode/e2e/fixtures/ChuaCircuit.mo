// MSL の ChuaCircuit と同じ配置（部品 7 つ）を、ライブラリ無しで読める形にしたもの（S247）
model ChuaCircuit
  model Part "部品"
    annotation(Icon(graphics = {Rectangle(extent = {{-100, -100}, {100, 100}}, fillColor = {200, 220, 255}, fillPattern = FillPattern.Solid)}));
  end Part;

  Part L annotation(Placement(transformation(origin = {-75, 38}, extent = {{-25, -25}, {25, 25}}, rotation = 270)));
  Part Ro annotation(Placement(transformation(origin = {-75, -17}, extent = {{-25, -25}, {25, 25}}, rotation = 270)));
  Part G annotation(Placement(transformation(extent = {{-25, 38}, {25, 88}})));
  Part C1 annotation(Placement(transformation(origin = {25, 3}, extent = {{-25, -25}, {25, 25}}, rotation = 270)));
  Part C2 annotation(Placement(transformation(origin = {-25, 3}, extent = {{-25, -25}, {25, 25}}, rotation = 270)));
  Part Nr annotation(Placement(transformation(origin = {75, 3}, extent = {{-25, -25}, {25, 25}}, rotation = 270)));
  Part Gnd annotation(Placement(transformation(extent = {{-25, -112}, {25, -62}})));
  annotation(Diagram(coordinateSystem(extent = {{-100, -100}, {100, 100}})));
end ChuaCircuit;

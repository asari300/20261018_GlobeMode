within Alfheim;

model Objects_Before

annotation(
  experiment(StartTime = 0, StopTime = 1),
  Diagram(
    graphics = {
      Rectangle(
        origin = {0, 0}, extent = {{-100, 100}, {100, -100}}, radius = 25,
        fillColor = {7, 84, 112}, fillPattern = FillPattern.Solid,
        borderPattern = BorderPattern.Raised
      ),
      Polygon(
        origin = {0, 40},
        points = {{0, -40}, {-80, 0}, {0, 40}, {80, 0}, {0, -40}},
        fillColor = {207, 241, 244}, fillPattern = FillPattern.Solid
      )
    }
  ),
  Icon(
    graphics = {
      Rectangle(
        origin = {0, 0}, extent = {{-100, 100}, {100, -100}}, radius = 25,
        fillColor = {7, 84, 112}, fillPattern = FillPattern.Solid,
        borderPattern = BorderPattern.Raised
      )
    }
  ),
  Documentation(
    __OpenModelica_infoHeader = "
      <html><head></head><body>
        the Header
      </body></html>
    ",
    info = "
      <html><head></head><body>
        The Infomation
      </body></html>
    ",
    revisions = "
      <html><head></head><body>
        The Revision
      </body></html>
    "
  ),
  uses(Modelica(version = "4.1.0"))
);

end Objects_Before;

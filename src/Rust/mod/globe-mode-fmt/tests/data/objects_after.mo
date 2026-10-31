within Alfheim;

model Objects_Before

annotation(
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
      ),
      Polygon(
        origin = {40, -20},
        points = {{-40, 20}, {40, 60}, {40, -20}, {-40, -60}, {-40, 20}},
        fillColor = {192, 226, 229}, fillPattern = FillPattern.Solid
      ),
      Polygon(
        origin = {-40, -20},
        points = {{40, 20}, {40, -60}, {-40, -20}, {-40, 60}, {40, 20}},
        fillColor = {177, 211, 214}, fillPattern = FillPattern.Solid
      )
    }
  ),
  Icon(
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
      ),
      Polygon(
        origin = {40, -20},
        points = {{-40, 20}, {40, 60}, {40, -20}, {-40, -60}, {-40, 20}},
        fillColor = {192, 226, 229}, fillPattern = FillPattern.Solid
      ),
      Polygon(
        origin = {-40, -20},
        points = {{40, 20}, {40, -60}, {-40, -20}, {-40, 60}, {40, 20}},
        fillColor = {177, 211, 214}, fillPattern = FillPattern.Solid
      )
    }
  )
);

end Objects_Before;

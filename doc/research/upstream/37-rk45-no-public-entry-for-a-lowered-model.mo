package Rep37

  model A "a small non-stiff model; any model that simulates shows the same call pattern"

    Real x(start = 1, fixed = true);
    Real v(start = 0, fixed = true);
    parameter Real k = 4;

  equation

    der(x) = v;
    der(v) = -k * x;

  annotation(experiment(StopTime = 10));

  end A;

end Rep37;

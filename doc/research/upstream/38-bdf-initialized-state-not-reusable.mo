package Rep38

  model A "a stiff state with an initial equation and an algebraic loop"

    parameter Real k = 100;
    Real x;
    Real y;
    Real z;

  equation

    der(x) = -k * (x - y);
    y = sin(z) + 0.5;
    z = x - 0.3 * y;

  initial equation

    x = 1;

  annotation(experiment(StopTime = 1));

  end A;

end Rep38;

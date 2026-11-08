package Rep35

  model A "control: fixed start values (exports)"

    parameter Real C1 = 15;
    parameter Real C2 = 15;
    parameter Real G = 10;
    Real T1(start = 100, fixed = true);
    Real T2(start = 0, fixed = true);
    Real Q;

  equation

    Q = G * (T1 - T2);
    C1 * der(T1) = -Q;
    C2 * der(T2) = Q;

  end A;

  model E "initial equation on a state, from a parameter"

    parameter Real T_start = 100;
    parameter Real C = 15;
    Real T;

  equation

    C * der(T) = -T;

  initial equation

    T = T_start;

  end E;

  model F "the same on two connected capacitors"

    parameter Real C1 = 15;
    parameter Real C2 = 15;
    parameter Real G = 10;
    parameter Real T1s = 100;
    parameter Real T2s = 0;
    Real T1;
    Real T2;
    Real Q;

  equation

    Q = G * (T1 - T2);
    C1 * der(T1) = -Q;
    C2 * der(T2) = Q;

  initial equation

    T1 = T1s;
    T2 = T2s;

  end F;

end Rep35;

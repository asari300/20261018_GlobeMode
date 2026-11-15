package Rep39

  record R "a plain record with the fields of the MSL's Complex"

    Real re;
    Real im;

  end R;

  model A "array equation, array constructor on the right, size 3"

    R aw[3];

  equation

    aw = {R(1, 0), R(2, 0), R(3, 0)};

  end A;

  model B "array equation, comprehension on the right, size from a parameter array"

    parameter Real a[:] = {1, 2, 3};
    R aw[size(a, 1)];

  equation

    aw = {R(a[i], 0) for i in 1:size(a, 1)};

  end B;

  model C "array = array, the other array set by an array constructor"

    R aw[3];
    R bw[3];

  equation

    aw = bw;
    bw = {R(1, 0), R(2, 0), R(3, 0)};

  end C;

  model D "control: one element at a time, in a for loop"

    R aw[3];

  equation

    for i in 1:3 loop
      aw[i] = R(i, 0);
    end for;

  end D;

  model E "control: array = array, the other array set in a for loop"

    R aw[3];
    R bw[3];

  equation

    aw = bw;
    for i in 1:3 loop
      bw[i] = R(i, 0);
    end for;

  end E;

  model F "control: arrays of Real"

    Real re[3];
    Real im[3];

  equation

    re = {1, 2, 3};
    im = {0, 0, 0};

  end F;

end Rep39;

package Rep41

  record R "a plain record with the fields of the MSL's Complex"

    Real re;
    Real im;

  end R;

  model A "size from size(a, 1), the field of all elements read as a slice"

    parameter Real a[:] = {1, 2, 3};
    R aw[size(a, 1)];
    Real s;

  equation

    for i in 1:size(a, 1) loop
      aw[i] = R(a[i], 0);
    end for;
    s = sum(aw.re);

  end A;

  model B "size from size(a, 1), the field of one element"

    parameter Real a[:] = {1, 2, 3};
    R aw[size(a, 1)];
    Real s;

  equation

    for i in 1:size(a, 1) loop
      aw[i] = R(a[i], 0);
    end for;
    s = aw[1].re;

  end B;

  model C "A as a component"

    A m;

  end C;

  model D "size from size(a, 1), the field of an element set in a for loop"

    parameter Real a[:] = {1, 2, 3};
    R aw[size(a, 1)];

  equation

    for i in 1:size(a, 1) loop
      aw[i].re = a[i];
      aw[i].im = 0;
    end for;

  end D;

  model E "size from an Integer parameter n = size(a, 1)"

    parameter Real a[:] = {1, 2, 3};
    parameter Integer n = size(a, 1);
    R aw[n];
    Real s;

  equation

    for i in 1:n loop
      aw[i] = R(a[i], 0);
    end for;
    s = sum(aw.re);

  end E;

  model F "control: literal size 3"

    parameter Real a[3] = {1, 2, 3};
    R aw[3];
    Real s;

  equation

    for i in 1:3 loop
      aw[i] = R(a[i], 0);
    end for;
    s = sum(aw.re);

  end F;

  model G "control: size from an Integer parameter n = 3"

    parameter Real a[:] = {1, 2, 3};
    parameter Integer n = 3;
    R aw[n];
    Real s;

  equation

    for i in 1:n loop
      aw[i] = R(a[i], 0);
    end for;
    s = sum(aw.re);

  end G;

  model H "control: arrays of Real sized by size(a, 1)"

    parameter Real a[:] = {1, 2, 3};
    Real re[size(a, 1)];
    Real im[size(a, 1)];
    Real s;

  equation

    re = a;
    im = zeros(size(a, 1));
    s = sum(re);

  end H;

end Rep41;

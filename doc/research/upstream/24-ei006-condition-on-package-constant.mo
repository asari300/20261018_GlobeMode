// Repro for upstream draft 24. Self-contained.
// Shapes taken from IBPSA.Fluid.FMI.Interfaces.FluidProperties (X_w if
// Medium.nXi > 0) and IBPSA.Fluid.Sources.MassFlowSource_WeatherData
// (x_pTphi if not singleSubstance, singleSubstance = (Medium.nX == 1)).
package Rep24

  package Medium

    constant Integer nX = 2;
    constant Integer nXi = nX - 1;

  end Medium;

  connector Props

    replaceable package M = Medium;
    Real T;
    Real X_w if M.nXi > 0;

  end Props;

  model A "condition on a constant of a replaceable package (connector)"

    Props p;

  equation

    p.T = 1;
    p.X_w = 0.01;

  end A;

  model B "condition on a final parameter bound to a package constant"

    final parameter Boolean single = (Medium.nX == 1);
    Real x = time if not single;

  end B;

  model C "condition directly on a package constant"

    Real x = time if Medium.nX > 1;

  end C;

  model C0 "control: the same condition on a local constant"

    constant Integer nX = 2;
    Real x = time if nX > 1;

  end C0;

  model C1 "control: the same condition on a local parameter"

    parameter Integer nX = 2;
    Real x = time if nX > 1;

  end C1;

end Rep24;

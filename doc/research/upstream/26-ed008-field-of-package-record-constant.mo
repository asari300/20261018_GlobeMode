// Repro for upstream draft 26. Self-contained.
// Shape taken from IBPSA.Media.Air (constant GasProperties dryair(...), read
// as dryair.cp in enthalpyOfDryAir; copies in Buildings and AixLib).
package Rep26

  record GasProperties

    Real MM;
    Real cp;

  end GasProperties;

  package Air

    constant GasProperties dryair(MM = 0.029, cp = 1006);
    constant Real cpAir = 1006;
    function h_T

      input Real T;
      output Real h;

    algorithm

      h := (T - 273.15) * dryair.cp;

    end h_T;

  end Air;

  model A "a function of the package reads a field of the record constant"

    Real h = Air.h_T(300 + time);

  end A;

  model B "the model reads the field directly"

    Real h = (300 + time - 273.15) * Air.dryair.cp;

  end B;

  model C "a parameter bound to the field"

    parameter Real cp = Air.dryair.cp;
    Real h = (300 + time - 273.15) * cp;

  end C;

  model B0 "control: a scalar constant of the same package"

    Real h = (300 + time - 273.15) * Air.cpAir;

  end B0;

  model B1 "control: a record constant declared in the model itself"

    constant GasProperties dryair(MM = 0.029, cp = 1006);
    Real h = (300 + time - 273.15) * dryair.cp;

  end B1;

end Rep26;

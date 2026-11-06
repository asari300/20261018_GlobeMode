// Repro for upstream draft 20 (replaceable Medium shape, S080). Self-contained.
// Shape taken from ThermofluidStream.Media.myMedia.Examples.MixtureGases and
// Buildings.DHC.Plants.Steam.BaseClasses.Examples.ControlVolumeEvaporation:
// two Medium packages in one model, both extending one base package with
// modifiers, and a model of the base package calling its function unqualified.
package Rep28

  partial package PM

    constant Real k = 1;
    replaceable function f

      input Real x;
      output Real y;

    algorithm

      y := k * x;

    end f;
    model BaseProperties

      Real x = time;
      Real y = f(x) "unqualified call of the package's function";

    end BaseProperties;

  end PM;

  package A

    extends PM(k = 2);

  end A;

  package B

    extends PM(k = 3);

  end B;

  package A0 "control: extends without a modifier"

    extends PM;

  end A0;

  package B0 "control: extends without a modifier"

    extends PM;

  end B0;

  model M

    replaceable package M1 = A;
    replaceable package M2 = B;
    M1.BaseProperties b1;
    M2.BaseProperties b2;

  end M;

  model M0 "control: one Medium"

    replaceable package M1 = A;
    M1.BaseProperties b1;

  end M0;

  model M1 "control: the base packages have no modifiers"

    replaceable package M1 = A0;
    replaceable package M2 = B0;
    M1.BaseProperties b1;
    M2.BaseProperties b2;

  end M1;

  model M2 "control: the same package twice"

    replaceable package M1 = A;
    replaceable package M2 = A;
    M1.BaseProperties b1;
    M2.BaseProperties b2;

  end M2;

end Rep28;

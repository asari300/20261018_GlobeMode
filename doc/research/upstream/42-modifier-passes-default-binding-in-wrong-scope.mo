package Rep42

  type Dyn = enumeration(Steady, Dynamic);
  partial model Decl

    parameter Dyn energyDynamics = Dyn.Dynamic;
    parameter Dyn massDynamics = energyDynamics;

  end Decl;
  model Vol

    extends Decl;
    Real m(start = 1);
    Real f = time;

  equation

    if massDynamics == Dyn.Steady then
      0 = f;
    else
      der(m) = f;
    end if;

  end Vol;
  model Boiler

    extends Decl;
    Vol vol(massDynamics = massDynamics);

  end Boiler;
  model A "massDynamics is passed down; its default is energyDynamics of the enclosing model"

    Boiler b(energyDynamics = Dyn.Steady);

  end A;
  model Vol5

    parameter Dyn energyDynamics = Dyn.Dynamic;
    parameter Dyn massDynamics = energyDynamics;
    Real m(start = 1);
    Real f = time;

  equation

    if massDynamics == Dyn.Steady then
      0 = f;
    else
      der(m) = f;
    end if;

  end Vol5;
  model Boiler5

    extends Decl;
    Vol5 vol(massDynamics = massDynamics);

  end Boiler5;
  model B "control: Vol5 declares the two parameters itself instead of extending Decl"

    Boiler5 b(energyDynamics = Dyn.Steady);

  end B;
  model Boiler6

    extends Decl;
    Vol vol(energyDynamics = energyDynamics);

  end Boiler6;
  model C "control: only energyDynamics is passed down (massDynamics of vol defaults to its own energyDynamics)"

    Boiler6 b(energyDynamics = Dyn.Steady);

  end C;
  model D "control: massDynamics is set on the enclosing model"

    Boiler b(massDynamics = Dyn.Steady);

  end D;

end Rep42;

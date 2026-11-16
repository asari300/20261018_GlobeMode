package Rep43

  model A "parameter of the predefined enumeration AssertionLevel"

    parameter AssertionLevel level = AssertionLevel.warning;
    Real x;

  equation

    x = time;

  end A;
  model B "parameter of the predefined enumeration StateSelect"

    parameter StateSelect sel = StateSelect.prefer;
    Real x;

  equation

    x = time;

  end B;
  model C "control: parameter of an enumeration declared by the user"

    type Level = enumeration(low, high);
    parameter Level level = Level.high;
    Real x;

  equation

    x = time;

  end C;

end Rep43;

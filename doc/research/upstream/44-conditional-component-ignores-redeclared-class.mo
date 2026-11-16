package Rep44

  model PartArea

    parameter Boolean use_in = true;

  end PartArea;
  model AreaFinal

    extends PartArea(final use_in = false);

  end AreaFinal;
  model Holder

    replaceable model Area = PartArea;
    Area area;
    Real u = 1 if area.use_in;

  end Holder;
  model A "redeclare model Area = AreaFinal: the redeclared class sets use_in = false"

    Holder h(redeclare model Area = AreaFinal);

  end A;
  model B "control: the component has the class AreaFinal, not replaceable"

    AreaFinal area;
    Real u = 1 if area.use_in;

  end B;
  model C "control: use_in is set by a modification on the component"

    Holder h(area(use_in = false));

  end C;
  model D "control: a flag of the model itself"

    final parameter Boolean flag = false;
    Real u = 1 if flag;

  end D;

end Rep44;

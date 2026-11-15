package Rep40

  partial model Base

    parameter Boolean show = false;
    Real x = 1;
    Real y = 2 if show;

  end Base;

  model Impl

    extends Base;

  end Impl;

  model Holder

    replaceable model M = Impl;

    M m;

  end Holder;

  model A "redeclare model M = Impl(show = true): the modification of the redeclared class"

    Holder h(redeclare model M = Impl(show = true));

  end A;

  model B "control: the modification on the component"

    Holder h(m(show = true));

  end B;

  model C "control: no replaceable class"

    Impl m(show = true);

  end C;

end Rep40;

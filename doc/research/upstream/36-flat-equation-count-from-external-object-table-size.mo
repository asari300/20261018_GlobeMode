package Rep36

  class Table

    extends ExternalObject;

    pure function constructor

      input Real table[:,:];
      output Table t;
      external "C" t = tableInit(table);

    end constructor;

    pure function destructor

      input Table t;
      external "C" tableFree(t);

    end destructor;

  end Table;

  pure function lookup

    input Table t;
    input Real u;
    output Real y;
    external "C" y = tableLookup(t, u);

  end lookup;

  model A "table of 2 rows: the equation tau0 = mu0 * fn is counted as 4"

    parameter Real tab[:, 2] = [0, 0.5; 1, 1];
    parameter Table t = Table(tab);
    parameter Real mu0 = lookup(t, 0);
    Real fn;
    Real tau0;

  equation

    fn = time;
    tau0 = mu0 * fn;

  end A;

  model B "table of 3 rows: the same equation is counted as 6"

    parameter Real tab[:, 2] = [0, 0.5; 1, 1; 2, 3];
    parameter Table t = Table(tab);
    parameter Real mu0 = lookup(t, 0);
    Real fn;
    Real tau0;

  equation

    fn = time;
    tau0 = mu0 * fn;

  end B;

  model C "control: the call written inline in the equation"

    parameter Real tab[:, 2] = [0, 0.5; 1, 1; 2, 3];
    parameter Table t = Table(tab);
    Real fn;
    Real tau0;

  equation

    fn = time;
    tau0 = lookup(t, 0) * fn;

  end C;

  model D "control: mu0 is a plain parameter"

    parameter Real mu0 = 2;
    Real fn;
    Real tau0;

  equation

    fn = time;
    tau0 = mu0 * fn;

  end D;

end Rep36;

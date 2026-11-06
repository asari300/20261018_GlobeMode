// Repro for upstream draft 27 (S083). Self-contained.
// A model inherited by `extends` into another package cannot be used through
// that package's name (ER002), unless it is reached through a replaceable alias.
package Rep35

  package P

    model B

      Real x = time;

    end B;

  end P;

  package Q

    extends P;

  end Q;

  model M "inherited class through the package name"

    Q.B b;

  end M;

  model M1 "the same through a non-replaceable alias"

    package A = Q;
    A.B b;

  end M1;

  model M0 "control: the class where it is declared"

    P.B b;

  end M0;

  model M2 "control: through a replaceable alias"

    replaceable package A = Q;
    A.B b;

  end M2;

end Rep35;

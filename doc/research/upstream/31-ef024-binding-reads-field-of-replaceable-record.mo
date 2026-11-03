// Repro for upstream draft 31 (S108). Self-contained.
// A binding that reads a field through something declared `replaceable`
// fails with EF024: (1) a record component declared `replaceable` (M, M2),
// (2) a record class declared `replaceable`, read in a function's protected
// binding (F, F1). Without `replaceable` they compile.
package Rep39
  record NV
    parameter Real m = 1;
  end NV;

  record Stage
    replaceable parameter NV nomVal;
  end Stage;

  record Coil
    parameter Stage sta;
    parameter Real mSmall = sta.nomVal.m;
  end Coil;

  record CoilR
    replaceable parameter Stage sta;
    parameter Real mSmall = sta.nomVal.m;
  end CoilR;

  model M "binding reads sta.nomVal.m, nomVal is replaceable"
    parameter Coil c(sta = Stage(nomVal = NV(m = 2)));
    Real y = c.mSmall*time;
  end M;

  model M2 "the same with sta also replaceable, as in Buildings"
    parameter CoilR c(sta = Stage(nomVal = NV(m = 2)));
    Real y = c.mSmall*time;
  end M2;

  record StageF
    parameter NV nomVal;
  end StageF;

  record CoilF
    parameter StageF sta;
    parameter Real mSmall = sta.nomVal.m;
  end CoilF;

  model M0 "control: nomVal not replaceable"
    parameter CoilF c(sta = StageF(nomVal = NV(m = 2)));
    Real y = c.mSmall*time;
  end M0;

  model M1 "control: the same field read in the model"
    parameter Stage s(nomVal = NV(m = 2));
    Real y = s.nomVal.m*time;
  end M1;

  package Q
    replaceable record ST
      Real a = 2.5;
    end ST;

    function d "protected record variable of the replaceable record"
      input Real T;
      output Real y;
    protected
      ST st;
      Real da = st.a;
    algorithm
      y := da*T;
    end d;

    function k "input of the replaceable record"
      input ST s;
      output Real y;
    protected
      Real t = s.a;
    algorithm
      y := 2*t;
    end k;
  end Q;

  model F "binding reads st.a, ST is a replaceable record class"
    Real y = Q.d(time);
  end F;

  model F1 "binding reads s.a of an input, ST is a replaceable record class"
    Real y = Q.k(Q.ST(a = time));
  end F1;

  package QF
    record ST
      Real a = 2.5;
    end ST;

    function d
      input Real T;
      output Real y;
    protected
      ST st;
      Real da = st.a;
    algorithm
      y := da*T;
    end d;
  end QF;

  model F0 "control: the same function with a record that is not replaceable"
    Real y = QF.d(time);
  end F0;
end Rep39;

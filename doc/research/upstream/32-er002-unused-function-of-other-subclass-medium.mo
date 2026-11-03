// Repro for upstream draft 32 (S108). Self-contained.
// M redeclares Medium1 = Good and calls Medium1.f. Another subclass of the
// same base (Engine) redeclares Medium1 = Bad. Bad has a function T_psX with
// an unresolvable call (as in AixLib's CHPCombustionMixtureGasNasa), reached
// only through Bad.g. M never uses Bad, Engine, g or T_psX, yet M fails with
// ER002 on the call inside T_psX.
package Rep40
  partial package PM
    replaceable partial function f
      input Real u;
      output Real y;
    end f;

    replaceable partial function g
      input Real u;
      output Real y;
    end g;
  end PM;

  package Good
    extends PM;
    redeclare function extends f
    algorithm
      y := u;
    end f;
  end Good;

  partial package Bad "a medium with a broken function that M never uses"
    extends PM;
    redeclare function extends f
    algorithm
      y := 2*u;
    end f;

    redeclare function extends g
    algorithm
      y := T_psX(u);
    end g;

    function T_psX
      input Real s;
      output Real T;
    protected
      function f_nonlinear
        input Real u;
        output Real y;
      algorithm
        y := specificEntropyOfpTX(u) - 1;  // not declared anywhere
      end f_nonlinear;
    algorithm
      T := f_nonlinear(s);
    end T_psX;
  end Bad;

  partial model PFP
    replaceable package Medium1 = PM;
  end PFP;

  model Engine "another subclass of PFP; redeclares Medium1 = Bad"
    extends PFP(redeclare package Medium1 = Bad);
  end Engine;

  model M "uses Good only"
    extends PFP(redeclare package Medium1 = Good);
    Real y = Medium1.f(time);
  end M;

  model M0 "control: no call through Medium1"
    extends PFP(redeclare package Medium1 = Good);
    Real y = time;
  end M0;

  partial model PFP2 "the same base without a subclass that uses Bad"
    replaceable package Medium1 = PM;
  end PFP2;

  model M1 "control: the same call on PFP2"
    extends PFP2(redeclare package Medium1 = Good);
    Real y = Medium1.f(time);
  end M1;
end Rep40;

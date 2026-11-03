// Repro for upstream draft 33 (S113). Self-contained, for AixLib.
// AixLib's CHPCombustionMixtureGasNasa copies MSL's MixtureGasNasa.T_psX,
// whose nested f_nonlinear calls the protected function specificEntropyOfpTX,
// but does not copy specificEntropyOfpTX. Rep41.Copy has the same shape.
// Any tool rejects a model that calls Copy.T_psX (or setState_psX).
package Rep41
  partial package Interface "stands for Modelica.Media.Interfaces.PartialMixtureMedium"
    constant Integer nX = 2;
  end Interface;

  package Original "stands for Modelica.Media.IdealGases.Common.MixtureGasNasa"
    extends Interface;

    function T_psX
      input Real p;
      input Real s;
      input Real X[nX];
      output Real T;
    protected
      function f_nonlinear
        input Real p;
        input Real s;
        input Real X[nX];
        input Real u;
        output Real y;
      algorithm
        y := specificEntropyOfpTX(p=p, T=u, X=X) - s;
      end f_nonlinear;
    algorithm
      T := s + f_nonlinear(p, s, X, 300);
    end T_psX;

  protected
    function specificEntropyOfpTX
      input Real p;
      input Real T;
      input Real X[nX];
      output Real s;
    algorithm
      s := T/300 - sum(X)*log(p/1e5);
    end specificEntropyOfpTX;
  end Original;

  package Copy "stands for AixLib CHPCombustionMixtureGasNasa: T_psX copied, specificEntropyOfpTX not"
    extends Interface;

    function T_psX
      input Real p;
      input Real s;
      input Real X[nX];
      output Real T;
    protected
      function f_nonlinear
        input Real p;
        input Real s;
        input Real X[nX];
        input Real u;
        output Real y;
      algorithm
        y := specificEntropyOfpTX(p=p, T=u, X=X) - s;  // not declared in Copy or Interface
      end f_nonlinear;
    algorithm
      T := s + f_nonlinear(p, s, X, 300);
    end T_psX;
  end Copy;

  model M "calls the copied T_psX: rejected by any tool"
    Real T = Copy.T_psX(1e5, time, {0.5, 0.5});
  end M;

  model M0 "control: the original T_psX"
    Real T = Original.T_psX(1e5, time, {0.5, 0.5});
  end M0;
end Rep41;

//! 自前の数値積分のプラグイン（S012）。Rumoca の FMI ME 風の host（[`super::session`]）に差す
//! `MeIntegratorBackend` の実装。
//!
//! - 刻み幅の制御のあるもの: `esdirk34`・`trbdf2`（diffsol の SDIRK）、`tsit45`（diffsol の陽的 RK）。[`rk`]
//! - 固定刻み: `euler`・`heun`・`rk4`（係数と補間は `globe_mode_sim::integrators`。Rumoca に依存しない）。[`fixed`]
//!
//! どれも積分器の歴史（刻み・段の値・補間）だけを持つ。初期化・イベント・根探索・出力の格子は host が受け持つ。

mod fixed;
mod rk;

use globe_mode_ir::{ResolvedSettings, SolverId};
use globe_mode_sim::integrators::fixed_step_tableau;
use rumoca_solver::fmi_me::{MeIntegratorBackend, MeNumericalSetup};

/// 自前の積分器を作る。Rumoca の積分器（bdf・dopri45・auto）は渡さない（[`super::session::Integrator`]）。
pub(super) fn plugin(
    id: SolverId,
    setup: MeNumericalSetup,
    settings: &ResolvedSettings,
) -> Result<Box<dyn MeIntegratorBackend + 'static>, String> {
    let method = match id {
        SolverId::Tsit45 => rk::Method::Tsit45,
        SolverId::Esdirk34 => rk::Method::Esdirk34,
        SolverId::Trbdf2 => rk::Method::TrBdf2,
        SolverId::Euler | SolverId::Heun | SolverId::Rk4 => {
            let tableau = fixed_step_tableau(id).expect("固定刻みのソルバには係数がある");
            // 刻み幅は settings.step（resolve が固定刻みのときに必ず入れる）。無ければ出力の間隔
            let h = settings.step.unwrap_or(settings.interval);
            if !(h.is_finite() && h > 0.0) {
                return Err(format!(
                    "ソルバ {} の刻み幅 {h} は正の有限の値でなければならない",
                    id.name()
                ));
            }
            return Ok(Box::new(fixed::FixedStep::new(
                tableau,
                h,
                settings.start_time,
            )));
        }
        SolverId::Bdf | SolverId::Dopri45 | SolverId::Auto => {
            return Err(format!(
                "ソルバ {} は Rumoca の積分器で回す（自前のプラグインには無い）",
                id.name()
            ));
        }
    };
    Ok(Box::new(rk::DiffsolRk::new(method, setup)))
}

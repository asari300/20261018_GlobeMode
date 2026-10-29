//! 自前の数値積分のプラグイン（S012）。Rumoca の FMI ME 風の host（[`super::session`]）に差す
//! `MeIntegratorBackend` の実装。
//!
//! - 刻み幅の制御のあるもの: `esdirk34`・`trbdf2`（diffsol の SDIRK）、`esdirk23`・`esdirk45`・`sdirk34hw`（自前の係数表を diffsol の SDIRK に差す。S027）、`tsit45`（diffsol の陽的 RK）、`dopri853`（自前の係数表を diffsol の陽的 RK に差す。S028）、`cerk23`・`cerk34`・`cerk45`（同じ道で、連続の補間の係数も差す。S029）。[`rk`]
//! - `radau5`（自前の Radau IIA 3 段 5 次。刻み幅の制御つき。簡略 Newton 法と HW の埋め込みの誤差の見積もり。S030）。
//!   係数・1 歩・刻み幅の制御は `globe_mode_sim::radau`。[`radau`]
//! - `qss`（自前の QSS2。刻みでなく量子化の事象で進む。S031）。事象の探索・状態の更新は `globe_mode_sim::qss`。[`qss`]
//! - 固定刻み: `euler`・`heun`・`rk4`（陽的）と `impeuler`・`trapezoid`（陰的、1 歩ごとに Newton 法。S026）。
//!   係数と補間は `globe_mode_sim::integrators`（Rumoca に依存しない）。[`fixed`]
//!
//! どれも積分器の歴史（刻み・段の値・補間）だけを持つ。初期化・イベント・根探索・出力の格子は host が受け持つ。

mod fixed;
mod qss;
mod radau;
mod rk;

use globe_mode_ir::{ResolvedSettings, SolverId};
use globe_mode_sim::integrators::{fixed_step_tableau, theta_method};
use rumoca_solver::fmi_me::{MeIntegratorBackend, MeNumericalSetup};

/// 自前の積分器を作る。Rumoca の積分器（bdf・dopri45・auto）は渡さない（[`super::session::Integrator`]）。
pub(super) fn plugin(
    id: SolverId,
    setup: MeNumericalSetup,
    settings: &ResolvedSettings,
) -> Result<Box<dyn MeIntegratorBackend + 'static>, String> {
    let method = match id {
        SolverId::Tsit45 => rk::Method::Tsit45,
        SolverId::Dopri853 | SolverId::Cerk23 | SolverId::Cerk34 | SolverId::Cerk45 => {
            rk::Method::CustomExplicit(id)
        }
        SolverId::Esdirk34 => rk::Method::Esdirk34,
        SolverId::Trbdf2 => rk::Method::TrBdf2,
        SolverId::Esdirk23 | SolverId::Esdirk45 | SolverId::Sdirk34hw => rk::Method::Custom(id),
        SolverId::Radau5 => return Ok(Box::new(radau::Radau5::new(setup))),
        SolverId::Qss => return Ok(Box::new(qss::Qss::new(setup))),
        SolverId::Euler
        | SolverId::Heun
        | SolverId::Rk4
        | SolverId::Impeuler
        | SolverId::Trapezoid => {
            let method = match (fixed_step_tableau(id), theta_method(id)) {
                (Some(tableau), _) => fixed::Method::Explicit(tableau),
                (None, Some(theta)) => fixed::Method::Theta(theta),
                (None, None) => unreachable!("固定刻みのソルバには係数がある"),
            };
            // 刻み幅は settings.step（resolve が固定刻みのときに必ず入れる）。無ければ出力の間隔
            let h = settings.step.unwrap_or(settings.interval);
            if !(h.is_finite() && h > 0.0) {
                return Err(format!(
                    "ソルバ {} の刻み幅 {h} は正の有限の値でなければならない",
                    id.name()
                ));
            }
            return Ok(Box::new(fixed::FixedStep::new(
                method,
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

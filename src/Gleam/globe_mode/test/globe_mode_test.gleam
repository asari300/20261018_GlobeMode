import gleam/string
import gleeunit
import globe_mode

pub fn main() -> Nil {
  gleeunit.main()
}

const model = "GlobeModeValidation.Simulation.Decay"

pub fn decay_check_simulate_test() {
  let assert Ok(ws) = globe_mode.new(["../../Modelica/GlobeModeValidation"])
  let assert Ok(report) = globe_mode.check(ws, model)
  assert string.contains(report, "\"passed\":true")
  let assert Ok(res) = globe_mode.simulate(ws, model, "{}")
  assert string.contains(res, "\"ok\":true")
  // JSON は解析しない。CSV の最後の行が時刻 1 で始まる
  assert string.contains(res, "\\n1,") || string.contains(res, "\\n1.0,")
  let assert Error(_) = globe_mode.graph(ws, "NoSuchModel", 1)
}

import gleam/string
import gleeunit
import globe_mode

pub fn main() -> Nil {
  gleeunit.main()
}

/// 試験でモデルの本文を読む（stdlib にファイルの読み書きが無いので Erlang の file を呼ぶ）。
@external(erlang, "file", "read_file")
fn read_file(path: String) -> Result(String, Nil)

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
  // S111: 参照シナリオの x(1)（e⁻¹ = 0.36787944…）と format
  assert string.contains(res, "\"x\":0.36787")
  let assert Ok(src) =
    read_file("../../Modelica/GlobeModeValidation/Simulation/Decay.mo")
  let assert Ok(formatted) = globe_mode.format(src, 2, True)
  assert string.contains(formatted, "der(x) = -k * x;")
  let assert Error(_) = globe_mode.format("model Decay", 2, True)
  assert string.starts_with(
    globe_mode.lint("model Decay Real x; end Decay;"),
    "[",
  )
}

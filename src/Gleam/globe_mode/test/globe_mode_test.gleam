import gleam/bit_array
import gleam/int
import gleam/list
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
  // S116: 列の道。x(1) が e⁻¹、点の数が CSV の行数 - 1（JSON の中の \n の数 - 1）と同じ
  let assert Ok(#(layout, [_time, xs])) =
    globe_mode.simulate_columns(ws, model, "{}")
  let points = list.length(string.split(res, "\\n")) - 2
  assert string.contains(layout, "\"points\":" <> int.to_string(points))
  assert bit_array.byte_size(xs) == 8 * points
  let assert Ok(<<x1:float-little>>) = bit_array.slice(xs, 8 * points - 8, 8)
  assert x1 >. 0.367879 && x1 <. 0.36788
}

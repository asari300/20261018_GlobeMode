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
  // S118: FMU（2.0、build なし）。展開したディレクトリに modelDescription.xml がある
  let assert Ok(fmu) =
    globe_mode.export_fmu(ws, model, "build/gm-gleam-fmu", "{}")
  assert string.ends_with(fmu, "GlobeModeValidation_Simulation_Decay-fmi2.fmu")
  let assert Ok(xml) =
    read_file(
      "build/gm-gleam-fmu/GlobeModeValidation_Simulation_Decay/modelDescription.xml",
    )
  assert string.contains(xml, "fmiModelDescription")
  let assert Error(_) =
    globe_mode.export_fmu(
      ws,
      model,
      "build/gm-gleam-fmu",
      "{\"fmi_version\": 4}",
    )
}

/// 別のプロセスで `simulate_observed` を回し、最初の t の進捗で `cancel` して結果を返す（受信と spawn は
/// gleam_erlang を足さずに済むよう test/globe_mode_test_ffi.erl に置く）。
@external(erlang, "globe_mode_test_ffi", "run_and_cancel")
fn run_and_cancel(
  ws: globe_mode.Workspace,
  model: String,
  settings_json: String,
  token: globe_mode.CancelToken,
) -> Result(String, String)

/// S131（ADR-0033）: S129 と同じ、積分に 100 ms より長くかかるモデル
pub fn cancel_from_another_process_test() {
  let assert Ok(ws) = globe_mode.new([])
  let assert Ok(_) =
    globe_mode.set_file(
      ws,
      "Wiggle.mo",
      "model Wiggle Real x(start = 0, fixed = true); equation der(x) = sin(100 * time); end Wiggle;",
    )
  let token = globe_mode.cancel_token()
  let assert Ok(res) =
    run_and_cancel(
      ws,
      "Wiggle",
      "{\"solver\": \"esdirk34\", \"stop_time\": 1000, \"interval\": 1}",
      token,
    )
  assert string.contains(res, "\"kind\":\"cancelled\"")
  assert string.contains(res, "GM-I11004")
}

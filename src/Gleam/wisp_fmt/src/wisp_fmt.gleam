//// 起動: GLOBE_BIN（既定 globe）と PORT（既定 8080）を読んで mist で待ち受ける。

import gleam/erlang/process
import gleam/int
import gleam/result
import mist
import wisp
import wisp/wisp_mist
import wisp_fmt/web

@external(erlang, "wisp_fmt_ffi", "getenv")
fn getenv(name: String, default: String) -> String

pub fn main() -> Nil {
  wisp.configure_logger()
  let ctx = web.Context(globe_bin: getenv("GLOBE_BIN", "globe"))
  let port =
    getenv("PORT", "8080")
    |> int.parse
    |> result.unwrap(8080)
  let secret_key_base = wisp.random_string(64)
  let assert Ok(_) =
    fn(req) { web.handle(req, ctx) }
    |> wisp_mist.handler(secret_key_base)
    |> mist.new
    |> mist.port(port)
    |> mist.start
  process.sleep_forever()
}

//// globe CLI（GlobeMode のエンジン）を子プロセスで呼ぶ。整形と lint の計算は Rust 側だけ。
//// 終了コード 0 / 1 は正常な結果（整形できない・指摘あり）として読む。それ以外は異常。

import gleam/dynamic/decode
import gleam/int
import gleam/json
import gleam/list
import gleam/option.{Some}
import gleam/result

pub type Options {
  Options(indent: Int, width: Int, insert_origin: Bool)
}

pub type Message {
  Message(rule: String, line: Int, column: Int, message: String)
}

@external(erlang, "wisp_fmt_ffi", "run")
fn run(
  bin: String,
  args: List(String),
  input: String,
) -> Result(#(Int, String), String)

/// 整形後の本文。整形できないときは CLI の理由（文法の誤り等）。
pub fn format(
  bin: String,
  text: String,
  options: Options,
) -> Result(String, String) {
  let origin = case options.insert_origin {
    True -> []
    False -> ["--no-insert-origin"]
  }
  let args =
    list.flatten([
      ["fmt", "--json", "-"],
      ["--indent", int.to_string(options.indent)],
      ["--width", int.to_string(options.width)],
      origin,
    ])
  let decoder = {
    use ok <- decode.field("ok", decode.bool)
    use text <- decode.field("text", decode.optional(decode.string))
    use error <- decode.field("error", decode.optional(decode.string))
    decode.success(#(ok, text, error))
  }
  use #(status, out) <- result.try(run(bin, args, text))
  case status {
    0 | 1 ->
      case json.parse(out, decoder) {
        Ok(#(True, Some(formatted), _)) -> Ok(formatted)
        Ok(#(False, _, Some(reason))) -> Error(reason)
        _ -> Error("globe の出力を読めない")
      }
    _ -> Error(exit_error(status))
  }
}

/// lint の指摘（0 件なら空のリスト）。disable は止める規則の名前。
pub fn lint(
  bin: String,
  text: String,
  disable: List(String),
) -> Result(List(Message), String) {
  let args =
    list.flatten([
      ["lint", "--json", "-"],
      list.flat_map(disable, fn(rule) { ["--disable", rule] }),
    ])
  let decoder = {
    use messages <- decode.field("messages", decode.list(message_decoder()))
    decode.success(messages)
  }
  use #(status, out) <- result.try(run(bin, args, text))
  case status {
    0 | 1 ->
      json.parse(out, decoder)
      |> result.replace_error("globe の出力を読めない")
    _ -> Error(exit_error(status))
  }
}

fn message_decoder() -> decode.Decoder(Message) {
  use rule <- decode.field("rule", decode.string)
  use line <- decode.field("line", decode.int)
  use column <- decode.field("column", decode.int)
  use message <- decode.field("message", decode.string)
  decode.success(Message(rule:, line:, column:, message:))
}

fn exit_error(status: Int) -> String {
  "globe が異常終了した（終了コード " <> int.to_string(status) <> "）"
}

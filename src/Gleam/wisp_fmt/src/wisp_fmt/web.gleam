//// ルーティングと API（doc/web-apps.md §3 の形）。整形・lint は engine に任せる。

import gleam/dynamic/decode
import gleam/http.{Get, Post}
import gleam/http/request
import gleam/json
import wisp.{type Request, type Response}
import wisp_fmt/engine
import wisp_fmt/page

/// globe のバイナリの場所（GLOBE_BIN）。
pub type Context {
  Context(globe_bin: String)
}

pub fn handle(req: Request, ctx: Context) -> Response {
  use <- wisp.log_request(req)
  use <- wisp.rescue_crashes
  use req <- wisp.handle_head(req)
  case req.method, request.path_segments(req) {
    Get, [] -> wisp.html_response(page.html(), 200)
    Get, ["healthz"] -> json_out(json.object([#("ok", json.bool(True))]), 200)
    Post, ["api", "format"] -> format_api(req, ctx)
    Post, ["api", "lint"] -> lint_api(req, ctx)
    _, _ -> wisp.not_found()
  }
}

type FormatRequest {
  FormatRequest(text: String, options: engine.Options)
}

// HTTP は常に 200（doc/web-apps.md §3）。不正な要求も ok: false で返す。
fn format_api(req: Request, ctx: Context) -> Response {
  use body <- wisp.require_string_body(req)
  let result = case json.parse(body, format_request_decoder()) {
    Error(_) -> Error("リクエストの JSON が不正")
    Ok(FormatRequest(text:, options:)) ->
      case valid_options(options) {
        False -> Error("indent は 1〜16、width は 1〜1000 にする")
        True -> engine.format(ctx.globe_bin, text, options)
      }
  }
  let out = case result {
    Ok(text) ->
      json.object([
        #("ok", json.bool(True)),
        #("text", json.string(text)),
        #("error", json.null()),
      ])
    Error(reason) ->
      json.object([
        #("ok", json.bool(False)),
        #("text", json.null()),
        #("error", json.string(reason)),
      ])
  }
  json_out(out, 200)
}

fn format_request_decoder() -> decode.Decoder(FormatRequest) {
  use text <- decode.field("text", decode.string)
  use indent <- decode.optional_field("indent", 2, decode.int)
  use width <- decode.optional_field("width", 80, decode.int)
  use insert_origin <- decode.optional_field("insert_origin", True, decode.bool)
  decode.success(FormatRequest(
    text:,
    options: engine.Options(indent:, width:, insert_origin:),
  ))
}

fn valid_options(options: engine.Options) -> Bool {
  options.indent >= 1
  && options.indent <= 16
  && options.width >= 1
  && options.width <= 1000
}

type LintRequest {
  LintRequest(text: String, disable: List(String))
}

fn lint_api(req: Request, ctx: Context) -> Response {
  use body <- wisp.require_string_body(req)
  case json.parse(body, lint_request_decoder()) {
    Error(_) -> error_out("リクエストの JSON が不正", 400)
    Ok(LintRequest(text:, disable:)) ->
      case engine.lint(ctx.globe_bin, text, disable) {
        Error(reason) -> error_out(reason, 500)
        Ok(messages) ->
          json_out(
            json.object([
              #("messages", json.array(messages, of: message_json)),
            ]),
            200,
          )
      }
  }
}

fn lint_request_decoder() -> decode.Decoder(LintRequest) {
  use text <- decode.field("text", decode.string)
  use disable <- decode.optional_field(
    "disable",
    [],
    decode.list(decode.string),
  )
  decode.success(LintRequest(text:, disable:))
}

fn message_json(m: engine.Message) -> json.Json {
  json.object([
    #("rule", json.string(m.rule)),
    #("line", json.int(m.line)),
    #("column", json.int(m.column)),
    #("message", json.string(m.message)),
  ])
}

fn json_out(value: json.Json, status: Int) -> Response {
  wisp.json_response(json.to_string(value), status)
}

fn error_out(reason: String, status: Int) -> Response {
  json_out(json.object([#("error", json.string(reason))]), status)
}

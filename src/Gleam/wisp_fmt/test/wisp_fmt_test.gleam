import gleam/dynamic/decode
import gleam/http
import gleam/json
import gleam/option.{None, Some}
import gleam/string
import gleeunit
import simplifile
import wisp/simulate
import wisp_fmt/web

@external(erlang, "wisp_fmt_ffi", "getenv")
fn getenv(name: String, default: String) -> String

pub fn main() -> Nil {
  gleeunit.main()
}

// テストはリポジトリの worktree のルートで回す想定（cargo build -p globe-mode-cli の target/debug/globe）。
const sample_before =
  "../../../src/Rust/mod/globe-mode-fmt/tests/data/objects_before.mo"

const sample_after =
  "../../../src/Rust/mod/globe-mode-fmt/tests/data/objects_after.mo"

fn ctx() -> web.Context {
  web.Context(globe_bin: getenv("GLOBE_BIN", "../../../target/debug/globe"))
}

fn read_sample(path: String) -> String {
  let assert Ok(text) = simplifile.read(path)
  text
}

fn post(path: String, body: json.Json) -> String {
  let res =
    simulate.request(http.Post, path)
    |> simulate.json_body(body)
    |> web.handle(ctx())
  assert res.status == 200
  simulate.read_body(res)
}

fn format_reply(body: String) {
  let decoder = {
    use ok <- decode.field("ok", decode.bool)
    use text <- decode.field("text", decode.optional(decode.string))
    use error <- decode.field("error", decode.optional(decode.string))
    decode.success(#(ok, text, error))
  }
  let assert Ok(reply) = json.parse(body, decoder)
  reply
}

fn lint_rules(body: String) -> List(String) {
  let rule = {
    use r <- decode.field("rule", decode.string)
    decode.success(r)
  }
  let decoder = {
    use rules <- decode.field("messages", decode.list(rule))
    decode.success(rules)
  }
  let assert Ok(rules) = json.parse(body, decoder)
  rules
}

pub fn format_sample_test() {
  let body =
    post(
      "/api/format",
      json.object([#("text", json.string(read_sample(sample_before)))]),
    )
  let assert #(True, Some(formatted), None) = format_reply(body)
  assert formatted == read_sample(sample_after)
}

pub fn format_broken_input_test() {
  let body =
    post("/api/format", json.object([#("text", json.string("model Broken"))]))
  let assert #(False, None, Some(_)) = format_reply(body)
}

pub fn lint_sample_test() {
  let body =
    post(
      "/api/lint",
      json.object([#("text", json.string(read_sample(sample_before)))]),
    )
  assert lint_rules(body) == ["missing-description"]
}

pub fn lint_disable_test() {
  let body =
    post(
      "/api/lint",
      json.object([
        #("text", json.string(read_sample(sample_before))),
        #("disable", json.array(["missing-description"], json.string)),
      ]),
    )
  assert lint_rules(body) == []
}

@external(erlang, "wisp_fmt_ffi", "putenv")
fn putenv(name: String, value: String) -> Nil

pub fn oversize_body_is_413_test() {
  let res =
    simulate.request(http.Post, "/api/format")
    |> simulate.json_body(
      json.object([#("text", json.string(string.repeat("x", 1_048_576)))]),
    )
    |> web.handle(ctx())
  assert res.status == 413
}

pub fn indent_zero_is_valid_test() {
  let body =
    post(
      "/api/format",
      json.object([
        #("text", json.string("model A end A;")),
        #("indent", json.int(0)),
      ]),
    )
  let assert #(True, Some(_), None) = format_reply(body)
}

pub fn disable_must_be_a_rule_name_test() {
  let res =
    simulate.request(http.Post, "/api/lint")
    |> simulate.json_body(
      json.object([
        #("text", json.string("model A end A;")),
        #("disable", json.array(["--json"], json.string)),
      ]),
    )
    |> web.handle(ctx())
  assert res.status == 400
}

pub fn slow_globe_times_out_test() {
  let slow = "/tmp/wisp_fmt_slow_" <> string.inspect(erlang_unique()) <> ".sh"
  let assert Ok(_) = simplifile.write(slow, "#!/bin/sh\nexec sleep 30\n")
  let assert Ok(_) = simplifile.set_permissions_octal(slow, 0o755)
  putenv("GLOBE_TIMEOUT_MS", "200")
  let res =
    simulate.request(http.Post, "/api/format")
    |> simulate.json_body(
      json.object([#("text", json.string("model A end A;"))]),
    )
    |> web.handle(web.Context(globe_bin: slow))
  putenv("GLOBE_TIMEOUT_MS", "10000")
  let _ = simplifile.delete(slow)
  let body = simulate.read_body(res)
  let assert #(False, None, Some(reason)) = format_reply(body)
  assert string.contains(reason, "終わらなかった")
}

@external(erlang, "erlang", "unique_integer")
fn erlang_unique() -> Int

pub fn healthz_test() {
  let res = simulate.request(http.Get, "/healthz") |> web.handle(ctx())
  assert res.status == 200
  assert simulate.read_body(res) == "{\"ok\":true}"
}

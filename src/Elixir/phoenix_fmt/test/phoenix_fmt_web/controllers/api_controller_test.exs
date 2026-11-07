defmodule PhoenixFmtWeb.ApiControllerTest do
  use PhoenixFmtWeb.ConnCase, async: true

  # 例題は Rust 側の tests/data（整形後は objects_after.mo）
  @sample Path.expand("../../../../../Rust/mod/globe-mode-fmt/tests/data", __DIR__)
  @before Path.join(@sample, "objects_before.mo")
  @after_file Path.join(@sample, "objects_after.mo")

  describe "POST /api/format" do
    @tag :engine
    test "例題を整形すると objects_after.mo と同じになる", %{conn: conn} do
      conn = post(conn, "/api/format", %{text: File.read!(@before)})

      assert %{"ok" => true, "text" => text, "error" => nil} = json_response(conn, 200)
      assert text == File.read!(@after_file)
    end

    @tag :engine
    test "文法が壊れていれば 200 で ok false（入力は触らない）", %{conn: conn} do
      conn = post(conn, "/api/format", %{text: "model A Real x end A;"})

      assert %{"ok" => false, "text" => nil, "error" => error} = json_response(conn, 200)
      assert is_binary(error)
    end

    test "indent が 0〜16 の整数でなければ 400", %{conn: conn} do
      conn = post(conn, "/api/format", %{text: "model A end A;", indent: 17})

      assert %{"error" => _} = json_response(conn, 400)
    end

    test "text が無ければ 400", %{conn: conn} do
      conn = post(conn, "/api/format", %{})

      assert %{"error" => _} = json_response(conn, 400)
    end

    test "本文が 1 MiB を超えれば 413", %{conn: conn} do
      body = Jason.encode!(%{text: String.duplicate("x", 1_048_576)})
      conn = put_req_header(conn, "content-type", "application/json")

      assert_error_sent 413, fn -> post(conn, "/api/format", body) end
    end
  end

  describe "POST /api/lint" do
    @tag :engine
    test "例題の指摘（missing-description）が返る", %{conn: conn} do
      conn = post(conn, "/api/lint", %{text: File.read!(@before)})

      assert [%{"rule" => "missing-description", "line" => 2, "column" => 7, "message" => _}] =
               json_response(conn, 200)["messages"]
    end

    @tag :engine
    test "disable で規則を外すと指摘が消える", %{conn: conn} do
      conn =
        post(conn, "/api/lint", %{text: File.read!(@before), disable: ["missing-description"]})

      assert %{"messages" => []} = json_response(conn, 200)
    end

    test "disable が文字列の配列でなければ 400", %{conn: conn} do
      conn = post(conn, "/api/lint", %{text: "model A end A;", disable: "x"})

      assert %{"error" => _} = json_response(conn, 400)
    end

    test "disable が規則名の形でなければ 400", %{conn: conn} do
      conn = post(conn, "/api/lint", %{text: "model A end A;", disable: ["--json"]})

      assert %{"error" => _} = json_response(conn, 400)
    end
  end
end

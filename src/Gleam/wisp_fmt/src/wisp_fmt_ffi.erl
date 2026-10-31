-module(wisp_fmt_ffi).
-export([getenv/2, run/3, read_file/1]).

%% 環境変数。無ければ Default。
getenv(Name, Default) ->
    case os:getenv(binary_to_list(Name)) of
        false -> Default;
        Value -> unicode:characters_to_binary(Value)
    end.

%% Bin を Args で起動し、Input を標準入力に渡して、(終了コード, 標準出力) を返す。
%% ポートは入力だけを閉じられないので、入力は一時ファイルに書いて sh の `<` で渡す。
%% 標準エラーは親のものをそのまま使う（JSON を汚さない）。
run(Bin, Args, Input) ->
    Tmp = filename:join(tmp_dir(), "wisp_fmt_" ++ os:getpid() ++ "_"
                        ++ integer_to_list(erlang:unique_integer([positive])) ++ ".in"),
    try
        ok = file:write_file(Tmp, Input),
        Script = "bin=\"$1\"; in=\"$2\"; shift 2; exec \"$bin\" \"$@\" < \"$in\"",
        Port = open_port({spawn_executable, "/bin/sh"},
                         [{args, ["-c", Script, "sh", Bin, Tmp | Args]},
                          binary, exit_status, use_stdio]),
        collect(Port, [])
    catch
        Class:Reason ->
            {error, iolist_to_binary(io_lib:format("~p:~p", [Class, Reason]))}
    after
        file:delete(Tmp)
    end.

collect(Port, Acc) ->
    receive
        {Port, {data, Data}} -> collect(Port, [Acc, Data]);
        {Port, {exit_status, Status}} -> {ok, {Status, iolist_to_binary(Acc)}}
    after 60000 ->
        port_close(Port),
        {error, <<"globe が 60 秒以内に終わらなかった">>}
    end.

tmp_dir() ->
    case os:getenv("TMPDIR") of
        false -> "/tmp";
        Dir -> Dir
    end.

%% テスト用。読めなければ {error, エラー名}。
read_file(Path) ->
    case file:read_file(Path) of
        {ok, Bin} -> {ok, Bin};
        {error, Reason} -> {error, atom_to_binary(Reason, utf8)}
    end.

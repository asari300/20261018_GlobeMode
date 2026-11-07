-module(wisp_fmt_ffi).
-export([getenv/2, putenv/2, run/3, read_file/1]).

%% 環境変数。無ければ Default。
getenv(Name, Default) ->
    case os:getenv(binary_to_list(Name)) of
        false -> Default;
        Value -> unicode:characters_to_binary(Value)
    end.

%% テスト用。時間切れの値を差し替える。
putenv(Name, Value) ->
    os:putenv(binary_to_list(Name), binary_to_list(Value)),
    nil.

%% Bin を Args で起動し、Input を標準入力に渡して、(終了コード, 標準出力) を返す。
%% ポートは入力だけを閉じられないので、入力は一時ファイルに書いて sh の `<` で渡す。
%% 標準エラーは親のものをそのまま使う（JSON を汚さない）。
run(Bin, Args, Input) ->
    Tmp = filename:join(tmp_dir(), "wisp_fmt_" ++ os:getpid() ++ "_"
                        ++ integer_to_list(erlang:unique_integer([positive])) ++ ".in"),
    %% exclusive: 既にあるファイル（symlink を含む）には書かない。書けたときだけ消す
    case file:write_file(Tmp, Input, [exclusive]) of
        {error, Reason} ->
            {error, unicode:characters_to_binary(io_lib:format("一時ファイルを作れない: ~p", [Reason]))};
        ok ->
            try
                Script = "bin=\"$1\"; in=\"$2\"; shift 2; exec \"$bin\" \"$@\" < \"$in\"",
                Port = open_port({spawn_executable, "/bin/sh"},
                                 [{args, ["-c", Script, "sh", Bin, Tmp | Args]},
                                  binary, exit_status, use_stdio]),
                Timeout = timeout_ms(),
                collect(Port, [], erlang:monotonic_time(millisecond) + Timeout, Timeout)
            catch
                Class:Reason2 ->
                    {error, iolist_to_binary(io_lib:format("~p:~p", [Class, Reason2]))}
            after
                file:delete(Tmp)
            end
    end.

%% 時間は出力の塊ごとではなく全体で数える。切るときは globe（sh が exec で置き換わっている）を kill する
collect(Port, Acc, Deadline, Timeout) ->
    Left = max(Deadline - erlang:monotonic_time(millisecond), 0),
    receive
        {Port, {data, Data}} -> collect(Port, [Acc, Data], Deadline, Timeout);
        {Port, {exit_status, Status}} -> {ok, {Status, iolist_to_binary(Acc)}}
    after Left ->
        case erlang:port_info(Port, os_pid) of
            {os_pid, Pid} -> os:cmd("kill -9 " ++ integer_to_list(Pid));
            _ -> ok
        end,
        catch port_close(Port),
        {error, unicode:characters_to_binary(io_lib:format("globe が ~p ミリ秒以内に終わらなかった", [Timeout]))}
    end.

timeout_ms() ->
    case os:getenv("GLOBE_TIMEOUT_MS") of
        false -> 10000;
        Value -> list_to_integer(Value)
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

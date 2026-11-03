%% S131: globe_mode_test.gleam の cancel の試験の受信と spawn（gleam_erlang を足さないため Erlang で書く）。
-module(globe_mode_test_ffi).
-export([run_and_cancel/4]).

%% 別のプロセスで回し、最初の t（積分の時刻）の進捗で cancel して、その結果を返す。
run_and_cancel(Ws, Model, Settings, Token) ->
    Me = self(),
    spawn(fun() -> Me ! {result, globe_mode_nif:simulate_observed(Ws, Model, Settings, Me, Token)} end),
    wait(Token).

wait(Token) ->
    receive
        {globe_progress, #{time := T}} when is_float(T) ->
            ok = globe_mode_nif:cancel(Token),
            receive {result, R} -> R after 30000 -> {error, <<"timeout">>} end;
        {globe_progress, _} -> wait(Token);
        {result, R} -> R
    after 30000 -> {error, <<"timeout">>}
    end.

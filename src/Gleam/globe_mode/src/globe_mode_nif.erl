%% S087: Rustler の NIF（globe-mode-nif の libglobe_mode_nif.so）を読む。
%% rustler::init!("globe_mode_nif") なので、この名前のモジュールで load_nif する（Elixir 側の :globe_mode_nif と同じ形）。
-module(globe_mode_nif).
-export([new/1, set_file/3, load_dir/2, check/2, graph/3, simulate/3, simulate_columns/3, cancel_token/0, cancel/1, simulate_observed/5, export_fmu/4, solvers/0, format/3, lint/1]).
-on_load(load/0).

load() ->
    erlang:load_nif(os:getenv("GLOBE_MODE_NIF", "libglobe_mode_nif"), 0).

new(_Libraries) -> erlang:nif_error(not_loaded).
set_file(_Ws, _Path, _Text) -> erlang:nif_error(not_loaded).
load_dir(_Ws, _Dir) -> erlang:nif_error(not_loaded).
check(_Ws, _Model) -> erlang:nif_error(not_loaded).
graph(_Ws, _Model, _Depth) -> erlang:nif_error(not_loaded).
simulate(_Ws, _Model, _SettingsJson) -> erlang:nif_error(not_loaded).
simulate_columns(_Ws, _Model, _SettingsJson) -> erlang:nif_error(not_loaded).
cancel_token() -> erlang:nif_error(not_loaded).
cancel(_Token) -> erlang:nif_error(not_loaded).
simulate_observed(_Ws, _Model, _SettingsJson, _Pid, _Token) -> erlang:nif_error(not_loaded).
export_fmu(_Ws, _Model, _OutDir, _OptionsJson) -> erlang:nif_error(not_loaded).
solvers() -> erlang:nif_error(not_loaded).
format(_Text, _Indent, _InsertOrigin) -> erlang:nif_error(not_loaded).
lint(_Text) -> erlang:nif_error(not_loaded).

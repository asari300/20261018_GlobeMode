defmodule :globe_mode_nif do
  @moduledoc false
  # S082: Rustler の NIF（globe-mode-nif の libglobe_mode_nif.so）。Gleam からも @external でこの名前で呼べる
  @on_load :load

  def load do
    path = System.get_env("GLOBE_MODE_NIF", "libglobe_mode_nif")
    :erlang.load_nif(String.to_charlist(path), 0)
  end

  def new(_libraries), do: :erlang.nif_error(:not_loaded)
  def set_file(_ws, _path, _text), do: :erlang.nif_error(:not_loaded)
  def load_dir(_ws, _dir), do: :erlang.nif_error(:not_loaded)
  def check(_ws, _model), do: :erlang.nif_error(:not_loaded)
  def graph(_ws, _model, _depth), do: :erlang.nif_error(:not_loaded)
  def simulate(_ws, _model, _settings_json), do: :erlang.nif_error(:not_loaded)
  def simulate_columns(_ws, _model, _settings_json), do: :erlang.nif_error(:not_loaded)
  def export_fmu(_ws, _model, _out_dir, _options_json), do: :erlang.nif_error(:not_loaded)
  def solvers, do: :erlang.nif_error(:not_loaded)
  def format(_text, _indent, _insert_origin), do: :erlang.nif_error(:not_loaded)
  def lint(_text), do: :erlang.nif_error(:not_loaded)
end

defmodule GlobeMode do
  @moduledoc """
  GlobeMode の Elixir の束ね（S082）。面は Python の `globe_mode.Workspace` と同じ。

  NIF の場所は環境変数 `GLOBE_MODE_NIF`（拡張子 `.so` を除いたパス）。結果は `{:ok, map}` か `{:error, 理由}`。
  """

  @doc "`libraries` はライブラリのディレクトリ（MSL など）。"
  def new(libraries \\ []), do: :globe_mode_nif.new(libraries)

  def set_file(ws, path, text), do: :globe_mode_nif.set_file(ws, path, text)

  @doc "ディレクトリの下の `.mo` をすべて読み、`{:ok, 読んだ数}`。"
  def load_dir(ws, dir), do: :globe_mode_nif.load_dir(ws, dir)

  def check(ws, model), do: decode(:globe_mode_nif.check(ws, model))

  def graph(ws, model, depth \\ 1), do: decode(:globe_mode_nif.graph(ws, model, depth))

  @doc "`settings` は `%{stop_time: 2, solver: \"bdf\"}` など。"
  def simulate(ws, model, settings \\ %{}),
    do: decode(:globe_mode_nif.simulate(ws, model, JSON.encode!(settings)))

  @doc """
  列の道（S116）。`{:ok, layout, columns}` で、`layout` は `%{"response", "names", "points"}`（`response` に `csv` は無い）、
  `columns` は `names` と同じ順の列ごとの f64（little-endian）の binary。値は `for <<x::float-little <- col>>, do: x` で読める。
  """
  def simulate_columns(ws, model, settings \\ %{}) do
    with {:ok, {layout, columns}} <-
           :globe_mode_nif.simulate_columns(ws, model, JSON.encode!(settings)),
         do: {:ok, JSON.decode!(layout), columns}
  end

  @doc """
  FMU を書き出して `{:ok, .fmu のパス}`（S118。`globe fmu export` と同じ）。`options` は
  `%{fmi_version: 2, build: false, targets: [], fmi_headers: nil}`（省略した鍵は既定）。
  """
  def export_fmu(ws, model, out_dir, options \\ %{}),
    do: :globe_mode_nif.export_fmu(ws, model, out_dir, JSON.encode!(options))

  def solvers, do: decode(:globe_mode_nif.solvers())

  @doc "本文を整形して `{:ok, 整形した本文}`。文法が壊れていれば `{:error, 理由}`（S111）。"
  def format(text, opts \\ []),
    do: :globe_mode_nif.format(text, Keyword.get(opts, :indent, 2), Keyword.get(opts, :insert_origin, true))

  @doc "lint の結果（`%{\"rule\", \"line\", \"column\", \"message\"}` のリスト。S111）。"
  def lint(text), do: JSON.decode!(:globe_mode_nif.lint(text))

  defp decode({:ok, json}), do: {:ok, JSON.decode!(json)}
  defp decode(error), do: error
end

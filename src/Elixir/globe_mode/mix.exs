defmodule GlobeMode.MixProject do
  use Mix.Project

  # S082: 依存は無い（JSON は Elixir 1.18 の標準の JSON）。NIF は cargo が作る（mise run elixir:test）
  def project do
    [app: :globe_mode, version: "0.0.1", elixir: "~> 1.18", deps: []]
  end

  def application, do: []
end

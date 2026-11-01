defmodule PhoenixFmt.Application do
  # See https://elixir.hexdocs.pm/Application.html
  # for more information on OTP Applications
  @moduledoc false

  use Application

  @impl true
  def start(_type, _args) do
    children = [
      PhoenixFmtWeb.Telemetry,
      {DNSCluster, query: Application.get_env(:phoenix_fmt, :dns_cluster_query) || :ignore},
      {Phoenix.PubSub, name: PhoenixFmt.PubSub},
      # Start a worker by calling: PhoenixFmt.Worker.start_link(arg)
      # {PhoenixFmt.Worker, arg},
      # Start to serve requests, typically the last entry
      PhoenixFmtWeb.Endpoint
    ]

    # See https://elixir.hexdocs.pm/Supervisor.html
    # for other strategies and supported options
    opts = [strategy: :one_for_one, name: PhoenixFmt.Supervisor]
    Supervisor.start_link(children, opts)
  end

  # Tell Phoenix to update the endpoint configuration
  # whenever the application is updated.
  @impl true
  def config_change(changed, _new, removed) do
    PhoenixFmtWeb.Endpoint.config_change(changed, removed)
    :ok
  end
end

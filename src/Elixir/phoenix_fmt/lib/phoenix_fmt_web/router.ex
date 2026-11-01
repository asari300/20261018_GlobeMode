defmodule PhoenixFmtWeb.Router do
  use PhoenixFmtWeb, :router

  pipeline :api do
    plug :accepts, ["json"]
  end

  scope "/", PhoenixFmtWeb do
    get "/", PageController, :index
    get "/healthz", PageController, :healthz
  end

  scope "/api", PhoenixFmtWeb do
    pipe_through :api

    post "/format", ApiController, :fmt
    post "/lint", ApiController, :lint
  end
end

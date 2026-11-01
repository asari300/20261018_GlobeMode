ExUnit.start()

# globe（GLOBE_BIN）が無いときは :engine の試験を飛ばす（警告を出す）
bin = System.get_env("GLOBE_BIN", "globe")

if System.find_executable(bin) == nil do
  IO.warn("GLOBE_BIN=#{bin} が見つからない。:engine の試験を飛ばす")
  ExUnit.configure(exclude: [engine: true])
end

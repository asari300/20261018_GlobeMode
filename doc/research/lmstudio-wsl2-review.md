# LM Studio（Windows）を WSL2 の Claude Code から副レビュアーとして使う調査

- 確認日: 2026-10-18（すべての項目）。確認できなかった項目には【未確認】と書いた。
- 「実機」は、この PC で測った・読んだ値（Ryzen 7 7730U、RAM 31GB、AMD Radeon 内蔵 GPU、Windows 11 + WSL2 Ubuntu 26.04.1、LM Studio の llama.cpp ランタイムは Vulkan 版 2.60.0）。
- 結論: Claude Code が主に実装とレビューを行う。副レビューは、スキル `/local-review` が差分を LM Studio の OpenAI 互換 API に送る形にした（[ADR-0008](../adr/0008-local-llm-reviewer.md)）。手順は [S006 の解説書](../sessions/S006-local-llm-review/解説書.md) にある。

## 1. WSL2 から Windows のサービスへつなぐ

| 項目 | 内容 | 出典 |
| --- | --- | --- |
| NAT モード（既定） | WSL から見て Windows は別ホスト。ホストの IP は `ip route show \| grep -i default \| awk '{ print $3}'` で取る。サーバは `0.0.0.0` で待ち受け、Windows のファイアウォールで受信を許す必要がある | <https://learn.microsoft.com/en-us/windows/wsl/networking> |
| DNS トンネリングとの関係 | `dnsTunneling` が有効だと `/etc/resolv.conf` の nameserver は `10.255.255.254` になる。この値は Windows の IP ではないので、ホストの IP を取るのに使えない | 同上 |
| mirrored モード | `%UserProfile%\.wslconfig` の `[wsl2]` に `networkingMode=mirrored` と書く（Windows 11 22H2 以降）。WSL から `127.0.0.1` で Windows のサーバに届く。IPv6 の `::1` は使えない | 同上、<https://learn.microsoft.com/en-us/windows/wsl/wsl-config> |
| `hostAddressLoopback` | `[experimental]` に `hostAddressLoopback=true`。127.0.0.1 以外のホストの IPv4 アドレスでも、ホストとコンテナが互いに届く | 同上 |
| Hyper-V ファイアウォール | mirrored では、受信を Hyper-V のファイアウォールで制御する。全体を許すなら管理者の PowerShell で `Set-NetFirewallHyperVVMSetting -Name '{40E0AC32-46A5-438A-A0B2-2B479E8F2E90}' -DefaultInboundAction Allow`、ポートを絞るなら `New-NetFirewallHyperVRule` | <https://learn.microsoft.com/en-us/windows/wsl/networking> |
| 反映 | `.wslconfig` を変えたら `wsl --shutdown`。MS は手での編集より、スタートメニューの「WSL Settings」アプリを勧めている | <https://learn.microsoft.com/en-us/windows/wsl/wsl-config> |
| 実機 | mirrored、`dnsTunneling=true`、`hostAddressLoopback=true`。WSL から `http://localhost:1234/v1/models` が返った。既定ゲートウェイはルータ（mirrored ではホストではない）なので、NAT 向けの「ゲートウェイの IP を使う」手順は**この構成では使えない** | 実機 |

## 2. LM Studio のサーバ

| 項目 | 内容 | 出典 |
| --- | --- | --- |
| `lms server start` | `--port`、`--cors`、`--bind`。`--bind` の既定は `127.0.0.1`。127.0.0.1 以外で待ち受けるときや CORS を有効にするときは、認証を有効にするよう docs が勧めている | <https://lmstudio.ai/docs/cli/server-start> |
| GUI の設定 | Developer → Server Settings: Server Port、Serve on Local Network、Enable CORS、Just in Time Model Loading、Auto Unload Unused JIT Models、Only Keep Last JIT Loaded Model、Require Authentication | <https://lmstudio.ai/docs/developer/core/server/settings> |
| 認証 | 0.4.0 以降。Require Authentication を有効にし、Manage Tokens で発行する。`Authorization: Bearer <token>` で送る。`/v1/messages` は `x-api-key` も受ける | <https://lmstudio.ai/docs/developer/core/authentication>、<https://lmstudio.ai/docs/developer/anthropic-compat> |
| JIT と TTL | JIT で読み込んだモデルは既定で 60 分使われないと解放される。リクエストの `"ttl": 秒` で個別に指定できる。`lms load` で読み込んだものは既定で TTL なし（`--ttl` で付ける） | <https://lmstudio.ai/docs/developer/core/ttl-and-auto-evict> |
| OpenAI 互換 | `http://<host>:1234/v1` の下に `/models`、`/chat/completions`、`/responses`、`/completions`、`/embeddings` | <https://lmstudio.ai/docs/developer/openai-compat> |
| REST API | `/api/v0/*`（`/api/v0/models` は state と `loaded_context_length` を返す。`/api/v0/chat/completions` は `stats` に tok/s と最初のトークンまでの時間を返す）。0.4.0 で `/api/v1/*` が入った | <https://lmstudio.ai/docs/developer/rest>、実機 |
| `POST /api/v1/models/load` | `context_length`、`eval_batch_size`、`flash_attention`、`num_experts`、`offload_kv_cache_to_gpu`、`echo_load_config`。**KV キャッシュの量子化、parallel、GPU オフロード率は無い** | <https://lmstudio.ai/docs/developer/rest/load> |
| Anthropic 互換 | 0.4.1 で `/v1/messages` が入った。公式ブログ「Claude Code with LM Studio」（2026-01-30）と連携の docs がある。ブログはコンテキスト 25K 以上を勧めている | <https://lmstudio.ai/changelog/lmstudio-v0.4.1>、<https://lmstudio.ai/blog/claudecode>、<https://lmstudio.ai/docs/integrations/claude-code> |
| 実機: 一覧の遅さ | `/api/v0/models` と `/api/v1/models` は、14〜63 秒かかることがあった（眠っていたサービスを起こすとき、`lms` は "Waking up LM Studio service..." と表示した）。`/v1/models` は 0.1 秒以内 | 実機 |
| 実機: 読み込み中の設定 | `/api/v1/models` の `loaded_instances[].config` に、`context_length`、`parallel`、`flash_attention`（既定で true）、`eval_batch_size`（2048）、`offload_kv_cache_to_gpu` が出る。推論の選択肢は `off` / `on`（既定 on） | 実機 |
| 実機: `/v1/messages` | `thinking: {"type":"disabled"}` を付けて "pong" が 6 秒で返った | 実機 |
| 実機: サーバの設定 | `networkInterface: "0.0.0.0"`（LAN と Tailscale に公開）、CORS 無効、JIT 有効、認証なし。ログは `logSensitiveData` と `logIncomingTokens` が true で、**プロンプトの全文がログに残る** | 実機（`%USERPROFILE%\.lmstudio\.internal\http-server-config.json`） |

### `lms load` のフラグ（実機の `lms load --help`）

公式 docs の一覧（`--ttl`、`--gpu`、`--context-length`、`--identifier`、`--estimate-only`）より多い。

| フラグ | 意味 |
| --- | --- |
| `-c, --context-length <n>` | コンテキスト長 |
| `--gpu <off\|max\|0〜1>` | GPU へのオフロード率 |
| `--parallel <n>` | 同時に処理する予測の数。既定は 4（実機の `lms ps` で確認） |
| `--ttl <秒>` | 使われなくなってから解放するまで |
| `--estimate-only` | メモリの見積もりだけを出す |
| `--speculative-draft-mtp` ほか | 投機的デコード（MTP、ドラフトモデル） |
| `--engine-config-file <path>` | エンジンの設定ファイルを読み込む。書式は docs に無い【未確認】 |
| `-y` | 確認をすべて承認する（スクリプト向け） |

- Flash Attention、K/V キャッシュの量子化、CPU のスレッド数、mmap、Keep Model in Memory は、`lms load` にも REST の load にも無い。**GUI のモデルごとの設定（My Models → 歯車）で変える。**
- その設定は `%USERPROFILE%\.lmstudio\.internal\user-concrete-model-default-config\<publisher>\<model>.json` に保存される（実機。Bonsai には `llm.load.contextLength` だけが入っていた）。
- アプリの中には `llm.load.llama.kCacheQuantizationType`、`llm.load.llama.vCacheQuantizationType`、`llm.load.llama.flashAttention`、`llm.load.llama.cpuThreadPoolSize` などのキーがある（実機でアプリのバンドルを検索して確認）。値の書式は【未確認】なので、ファイルを手で編集せず GUI で変える。

## 3. Bonsai

PrismML の「Bonsai」には系列が複数ある。インストール済みなのは **Bonsai-27B**。

| モデル | 公開 | ベース | 形式 | 最大 ctx | 出典 |
| --- | --- | --- | --- | --- | --- |
| 1-bit Bonsai 8B / 4B / 1.7B | 2026-03-31 | Qwen3（dense） | GGUF Q1_0（g128、1.125 bpw）、Apache-2.0 | 65,536 / 32,768 / 32,768 | <https://huggingface.co/prism-ml/Bonsai-8B-gguf>、<https://www.theregister.com/2026/04/04/prismml_1bit_llm/> |
| Ternary Bonsai 8B / 4B / 1.7B | 2026-04-16 | 同上 | 1.58 bit | — | <https://prismml.com/news/prismml-introduces-ternary-bonsai-model-family> |
| **Bonsai 27B** | — | Qwen3.6 27B（LM Studio の説明） | GGUF Q1_0、3.80 GB。ツール呼び出し、推論（既定で有効）、画像入力 | 262,144 | <https://lmstudio.ai/models/bonsai-27b> |
| Ternary-Bonsai-2（27B / 9B / 4B / 2B） | 2026-09-17〜10-15 | Qwen3.5 | PTQ1_0 / PQ2_0。PrismML のビルドか、upstream の b11555 以降が要る | — | <https://docs.prismml.com/run/llamacpp> |

### Bonsai-27B の構成（実機の GGUF のメタデータ）

`Bonsai-27B-Q1_0.gguf` の先頭を読んだ値。

| キー | 値 | 意味 |
| --- | --- | --- |
| `general.architecture` | `qwen35` | Qwen3.5 系のハイブリッド構成 |
| `block_count` | 64 | 層の数 |
| `full_attention_interval` | 4 | **4 層に 1 層だけが full attention**。残りの 48 層は線形注意（SSM、Gated DeltaNet）で、KV キャッシュを持たない |
| `attention.head_count` / `head_count_kv` | 24 / 4 | GQA |
| `attention.key_length` / `value_length` | 256 / 256 | head_dim |
| `ssm.state_size` / `inner_size` / `time_step_rank` | 128 / 6144 / 48 | 線形注意の状態（ctx に依らず一定） |
| `context_length` | 262,144 | 最大コンテキスト |
| `general.sampling.*` | temp 1.0、top_p 0.95、top_k 20 | 同梱された推奨値（推論を有効にしたとき向け） |
| チャットテンプレート | `enable_thinking` が false のときだけ空の `<think></think>` を入れる | 推論の切り替え |

### KV キャッシュの大きさ

式は `2（K と V）× KV を持つ層の数 × KV ヘッド数 × head_dim × ctx × 1 要素のバイト数`。

- Bonsai-27B は KV を持つ層が 64 / 4 = 16 層なので、fp16 で 1 トークンあたり 2 × 16 × 4 × 256 × 2 B = **64 KiB**。
- 同じ大きさの dense モデル（64 層すべてが full attention）の 4 分の 1 になる。
- q8_0 は 32 要素で 34 B（約 0.53 倍）、q4_0 は 32 要素で 18 B（約 0.28 倍）。これは ggml のブロック形式からの計算。

| ctx | fp16 | q8_0 | q4_0 | 参考: `lms load --estimate-only`（全体、実機） |
| --- | --- | --- | --- | --- |
| 8,192 | 0.50 GiB | 0.27 GiB | 0.14 GiB | 5.27 GiB |
| 16,384 | 1.00 GiB | 0.53 GiB | 0.28 GiB | 5.66 GiB |
| 32,768 | 2.00 GiB | 1.06 GiB | 0.56 GiB | 6.44 GiB |
| 65,536 | 4.00 GiB | 2.13 GiB | 1.13 GiB | 7.99 GiB |
| 262,144 | 16.0 GiB | 8.50 GiB | 4.50 GiB | — |

- 見積もりの増え方は 8K あたり約 0.39 GiB。fp16 の計算値（0.5 GiB）より小さいが、理由は【未確認】。
- 線形注意の状態は、48 層 × 48 ヘッド × 128 × 128 × 4 B ≈ 144 MiB（f32 と仮定した計算）。ctx に依らず一定で、`--parallel` の数だけ要ると考えられる【未確認】。

### 実機での速さ（2026-10-18）

`/api/v0/chat/completions` の `stats` から取った値。入力は Rust のソース。

| 構成 | 入力 | 最初のトークンまで | prompt 処理 | 生成 |
| --- | --- | --- | --- | --- |
| GPU max、ctx 32K、parallel 4、推論あり（短い質問） | 25 | 25.7 秒（初回） | — | 1.90 tok/s（推論 186 トークンで 126 秒） |
| 同上、`reasoning_effort: "none"` | 27 | 2.0 秒 | — | 1.95 tok/s（4 トークン、3.8 秒） |
| GPU max、ctx 32K、parallel 4 | 2,124 | 262.8 秒 | 8.1 tok/s | 1.81 tok/s |
| GPU off、ctx 16K、parallel 1 | 2,124 | 267.1 秒 | 8.0 tok/s | 2.24 tok/s |
| GPU max、ctx 12K、parallel 1 | 1,152 | 117.9 秒 | 9.8 tok/s | 2.24 tok/s |
| GPU off、ctx 12K、parallel 1 | 1,152 | 121.7 秒 | 9.5 tok/s | 2.31 tok/s |

- **内蔵 GPU にオフロードしても速くならない。** `lms runtime survey` では内蔵 GPU の VRAM は 3.00 GiB（専用の割り当て）で、モデルの見積もり（5〜6 GiB）が収まらない。Vulkan 版のランタイムが Q1_0 の計算を GPU でしていない可能性もある【未確認】。
- 読み込みは GPU off で 8.5 秒、GPU max で 53 秒。
- 推論を切る方法:

  | 指定 | 効いたか |
  | --- | --- |
  | `reasoning_effort: "none"`（OpenAI 互換） | 効いた |
  | `thinking: {"type":"disabled"}`（`/v1/messages`） | 効いた |
  | `chat_template_kwargs: {"enable_thinking": false}` | **無視された**（推論 269 トークン） |

- Rust のソースでは 1 トークンが約 3.0〜3.3 文字だった。

## 4. Claude Code 側の選択肢

| 項目 | 内容 | 出典 |
| --- | --- | --- |
| サポートの範囲 | ゲートウェイ経由で Claude 以外のモデルにつなぐ使い方は、Anthropic のサポート外 | <https://code.claude.com/docs/en/llm-gateway> |
| `--bare` | hooks、スキル、MCP、CLAUDE.md の自動読み込みを省き、システムプロンプトを最小にする（`CLAUDE_CODE_SIMPLE=1` と同じ） | <https://code.claude.com/docs/en/headless>、<https://code.claude.com/docs/en/env-vars> |
| サブエージェントの `model` | `sonnet` / `opus` / `haiku` / `fable` / 完全なモデル ID / `inherit`。接続先（base URL）をサブエージェントごとに変える欄は無い | <https://code.claude.com/docs/en/sub-agents> |
| スキル | `.claude/skills/<name>/SKILL.md`。frontmatter に `allowed-tools`、`disable-model-invocation`、`context: fork`、`model` など | <https://code.claude.com/docs/en/skills> |
| MCP | `claude mcp add [--scope local\|project\|user] -e K=V <name> -- <cmd>` | <https://code.claude.com/docs/en/mcp> |
| hooks | Stop フックで exit 2 にすると会話が続き、出力が Claude に渡る。command のタイムアウトの既定は 600 秒 | <https://code.claude.com/docs/en/hooks> |

### 実機: `claude --bare -p` を LM Studio に向けた（D-3）

```bash
ANTHROPIC_BASE_URL=http://localhost:1234 ANTHROPIC_AUTH_TOKEN=lmstudio ANTHROPIC_API_KEY= \
CLAUDE_CODE_ATTRIBUTION_HEADER=0 CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC=1 \
ANTHROPIC_DEFAULT_HAIKU_MODEL=prism-ml/bonsai-27b \
  claude --bare -p "Reply with the single word: pong" --model prism-ml/bonsai-27b --output-format json --max-turns 1
```

- 結果は "pong"。入力 1,197 トークン、出力 99 トークン、最初のトークンまで 178 秒、合計 179 秒。
- Claude Code はこのモデルのコンテキストを 200,000 と仮定していた（`contextWindow`）。実際の読み込み（12〜16K）を超えるとエラーになる。
- 標準エラーには `[claude-code:unrecognized_model]` が出た。
- レビューのようにファイルを読ませると、入力はすぐ数千トークンになる。1 回で 10 分を超える見込み。

### MCP サーバ

| サーバ | ライセンス | 内容 | 出典 |
| --- | --- | --- | --- |
| houtini-ai/lm（`@houtini/lm`） | Apache-2.0 | OpenAI 互換なら何でも使える。ツールは `chat`、`custom_prompt`、`code_task`、`code_task_files`（ファイルを MCP サーバが読む）、`discover` など 8 個。Node 22.5 以上 | <https://github.com/houtini-ai/lm> |
| raine/consult-llm-mcp | MIT | 複数の provider に相談する。`CONSULT_LLM_<PROVIDER>_BASE_URL` で接続先を変える | <https://github.com/raine/consult-llm-mcp> |
| infinitimeless/LMStudio-MCP | MIT | LM Studio 専用 | <https://github.com/infinitimeless/LMStudio-MCP> |

- houtini-lm の環境変数は `HOUTINI_LM_ENDPOINT_URL`（`/v1` なし）、`HOUTINI_LM_API_KEY`、`HOUTINI_LM_MODEL`、`HOUTINI_LM_THINKING`（auto / off / on）、`HOUTINI_LM_SERIALISE`（既定 1）、`HOUTINI_LM_MIN_TOKENS`（**既定 4096**）。
- 生成が 2 tok/s のこの PC では、`MIN_TOKENS` の既定のままだと 1 回の出力に最大 34 分かかる。
- 実機で試した結果は [S006 の解説書 §2](../sessions/S006-local-llm-review/解説書.md#2-決定と理由) にある。

# Tong Wen (同文) v0.3

零 ASR、純規則 + OpenCC 的 OpenAI-compatible 台灣繁體後處理服務。Rust 實作，API 相容舊版：`/health`、`/v1/models`、`/v1/chat/completions`（`stream: true` 會被接受但不串流，一律回傳完整 JSON 回應）。

內部後處理鏈移植自 [SpeakSlow (聲聲慢)](https://github.com/Jeffrey0117/SpeakSlow) 的 `text_processing.py`，順序 1:1 對應，簡轉繁改用 `opencc-rust` S2TW。

## 後處理管線

只取 **最後一條 user 訊息** 為輸入；若 `model` 以 `-voiceink` 結尾，先剝 `<TRANSCRIPT>` 標籤。

按序執行（對應 `src/processing.rs` / `src/convert.rs`）：

| #   | 函式                          | 內容                                                                         |
| --- | ----------------------------- | ---------------------------------------------------------------------------- |
| 1   | `collapse_repeats`            | 口吃疊字收斂（白名單保留 慢慢/謝謝；AABB 疊詞保留）                          |
| 2   | `collapse_phrase_repeats`     | 詞組口吃（其實其實其實→其實）+ 發語詞重複                                    |
| 3   | `normalize_interjections`     | 哎/誒→欸、去呃                                                               |
| 4   | `fix_tw_pronunciation`        | 樂色/勒色→垃圾（負向環視避開 音樂色彩 等）                                   |
| 5   | `apply_punct_rules`           | 句尾語助詞標點（吗→？/啦→！）、片語規則（真的假的→！）、哈→吼、好了好→好了吼 |
| 6   | `format_lists`                | 第一…第二…→1. 2. 3.（預設關閉，`config.toml` `lists = true` 開啟）           |
| 7   | `localize_english_punct`      | 英文為主的行→半形標點+句首大寫+i→I；中英混雜行不動                           |
| 8   | `to_traditional`              | OpenCC `S2TW` + 賬→帳                                                        |
| 9   | `strip_short_trailing_period` | ≤5 字短句的句尾。拿掉                                                        |

不包含：ct-punc 神經標點、emoji 觸發詞、Hybrid LLM、使用者自訂 emoji DB。

### 詞彙來源

`opencc-rust` `DefaultConfig::S2TW`（簡→台灣字形繁體，無詞彙層）：网络→網絡、内存→內存、视频→視頻、软件→軟件。另有字形補丁 `賬→帳`。

### 轉換行為

無簡繁偵測閘門，一律走 `S2TW`（OpenCC 不可用或轉換失敗時原樣返回）。偏好 `S2TW`，初始化失敗自動退回 `S2T`。

## 建置需求

系統需安裝 OpenCC C++ 函式庫：

```bash
# macOS
brew install opencc
# Debian/Ubuntu
apt install libopencc-dev
```

`opencc-rust` 透過 `pkg-config` / `OPENCC_*` 環境變數尋找函式庫；必要時：

```bash
OPENCC_LIB_DIRS=/opt/homebrew/lib OPENCC_INCLUDE_DIRS=/opt/homebrew/include cargo run --release
```

可啟用 `static-dictionaries` feature 將字典內嵌（仍需動態連結 libopencc）。

## 設定

以檔案為設定（TOML），僅當指定 CLI 參數時讀取：`--config <path>` / `-c <path>`。

```toml
# config.toml
host = "127.0.0.1"
port = 1180
lists = false  # true 開啟規則式列點排版（第一…第二…→1. 2. 3.）
```

```bash
cargo run --release                       # 無參數：使用預設值
cargo run --release -- --config config.toml
cargo run --release -- --config /path/to/config.toml
cargo run --release -- -c my.toml
```

若指定的檔案不存在，印出警告並使用預設值（同上）；欄位缺漏亦補預設。範例檔見 `example.toml`。

## API

- `GET /health` → `ok`
- `GET /v1/models` → `tongwen`、`tongwen-voiceink`
- `POST /v1/chat/completions`
  - `model` 缺漏/空 → 補 `tongwen`
  - `model` 以 `-voiceink` 結尾 → 剝 `<TRANSCRIPT>` 標籤
  - 回應為 OpenAI 格式 JSON（含 `usage`）

### 只轉最後一條 user 訊息

`messages` 中多條訊息時，逆序取第一條 `role=user`；若無則取最後一條。`content` 支援字串與 `[{type:"text",text:...}]` 陣列。

## 快速開始

```bash
cargo run --release
# http://127.0.0.1:1180
```

### 查詢模型

```bash
curl http://localhost:1180/v1/models
```

### 轉換

```bash
curl -X POST http://localhost:1180/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{"model":"tongwen","messages":[{"role":"user","content":"汉字转换：软件、电脑、网络"}]}'
# → 漢字轉換：軟體、電腦、網路
```

## 接入 VoiceInk / Superwhisper

將 App 的 OpenAI Base URL 設為 `http://127.0.0.1:1180/v1`，模型選 `tongwen-voiceink`（會自動剝 `<TRANSCRIPT>`）。一般文字接 `tongwen`。

## Crate 結構

```
src/
  main.rs        # bin 薄殼：config → bind → graceful shutdown
  lib.rs         # re-export
  config.rs      # CLI --config 載入（host/port/lists），無參數用預設
  pipeline.rs    # post_process() 管線編排
  processing.rs  # 純文字步驟（1:1 對應 text_processing.py）
  convert.rs     # OpenCC S2TW + 賬→帳
  server.rs      # axum 路由
example.toml         # 範例設定檔
```

## 測試

```bash
cargo test
```

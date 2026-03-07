---
description: 後端網路連線與 Authoritative 同步 MVP
---

# 任務藍圖：Task 02 - Network Sync MVP

**指派對象**：Backend Engineer Agent  
**目標**：在 Bevy 0.18 中引入連線函式庫（例如 `lightyear` 或 `bevy_replicon`），建立 Client-Server 的連線，並實作 Authoritative Server 的基礎驗證邏輯（Client 上報意圖，Server 推送狀態）。

## 1. 涉及檔案清單
- `Cargo.toml` (Workspace / shared / client / server)（修改：加入連線相關 Crate，例如 `bevy_replicon` 或自製 UDP wrapper）。
- `shared/src/lib.rs` & `shared/src/protocol.rs`（新建/修改：定義網路封包 Event 或共用的 Replica Component）。
- `server/src/network.rs` & `server/src/main.rs`（新建/修改：開啟 Server Port，接收並驗證 Client 傳來的 `MoveIntent`）。
- `client/src/network.rs` & `client/src/main.rs`（新建/修改：連線至 Server，並在點擊地圖時送出 `MoveIntent` UDP Packet，而非直接改自己的 `TargetPosition`）。

## 2. Step-by-Step 實作步驟
1. **依賴注入與 Protocol 定義**：在 `shared` 裡加入 `bevy_replicon` (或雙向 UDP Socket 選擇)，定義 `MoveCommand { target_x, target_y }` 訊息。標記 `Position` Component 為伺服器覆寫（Replicated）。
2. **啟動 Server 監聽**：在 `server` 端綁定 UDP 埠口（如 5000），接受連線並為每個 Client 生成對應的 Entity 與初始化 `Position`。
3. **客戶端連線與封包發送**：在 `client` 端啟動時連線本機 Server，並將原本直接寫入 `TargetPosition` 的 Input System，改為對 Server 發送 `MoveCommand` RPC/Event。
4. **Server 權威計算**：`server` 端接收 `MoveCommand` 後，將該 Client Entity 的 `TargetPosition` 設為新座標，並在 Server 端執行 `movement_system` 更新 `Position`。
5. **Client 狀態映射**：`client` 透過網路同步直接獲得 `Position` 的更新（Server 廣播），平滑渲染。

## 3. 資料結構定義
例如在 `shared/src/protocol.rs` 加入（視選用的網路庫而定）：
```rust
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Event, Serialize, Deserialize, Debug, Clone)]
pub struct MoveIntent {
    pub target_x: f32,
    pub target_y: f32,
}
```

## 4. 邊界提醒（Backend Engineer 絕對不能做的事）
- 🚫 **不要實作 Client Side Prediction 或是 Rollback（延遲補償）**。目前是 MVP，先讓連線跑通（純 Server-Authoritative 即可，哪怕 Client 點下去要等 50ms 才會開始動）。
- 🚫 **不要動到任何渲染邏輯**。
- 🚫 **嚴禁建立資料庫（DB）或登入驗證**。現在只需要任意開 Client 就能拿到一個隨機 Player Entity 並移動即可。

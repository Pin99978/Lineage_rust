---
description: 伺服器持久化與帳號登入 MVP
---

# 任務藍圖：Phase 1 - Task 01 - DB Persistence & Login MVP

**指派對象**：Backend Engineer Agent / Technical Artist Agent  
**目標**：讓這個世界變成持久的 MMORPG！引入輕量級資料庫（如 `SQLite` 或 `RocksDB` / `Sled`），讓玩家在連線時能夠「輸入帳號名稱」登入，並從資料庫讀取/寫入玩家的屬性、裝備、座標位置。這能確立跨連線 Session 的資料保留能力。

## 1. 涉及檔案清單
- `Cargo.toml` (Workspace / server)（修改：加入資料庫 ORM 或 Driver，例如 `sqlx` 搭配 `sqlite`，或是 `rusqlite`）。
- `shared/src/protocol.rs`（修改：加入 `LoginRequest { username }` 與 `LoginResponse` 或 `SpawnPlayerEvent`）。
- `server/src/db.rs`（新建：處理資料庫連接池初始化，建立 `users` 資料表，封裝讀寫玩家庫存與座標的函數）。
- `server/src/network.rs` 或 `server/src/systems/login.rs`（修改/新建：攔截剛連線的客戶端，拒絕未登入者的移動請求，直到他們成功發送 `LoginRequest` 並驗證透過）。
- `client/src/systems/ui.rs`（修改：在初始畫面建立一個超極簡的文字輸入框，讓玩家輸入名稱並點選「登入 / Login」按鈕）。

## 2. Step-by-Step 實作步驟
1. **(Backend) 資料庫初始化**：在 `server` 啟動時建立/連接到本地 `data.db`。執行資料表 Migration（確保 `users` 表格存在，含欄位 `id`, `username`, `x`, `y`, `inventory_json`, `health`）。
2. **(Technical Artist) 客戶端登入 UI**：在 Client 開啟時，不要直接生成場景跟相機，而是先進入一個 `AppState::LoginMenu` 階段。顯示一個輸入框（或簡單從終端機 `stdin` 讀取參數，作為 MVP 可接受這兩種作法），取得字串後發送 `LoginRequest` 給 Server。
3. **(Backend) 登入與恢復狀態**：Server 接收 `LoginRequest` 後，若於 DB 查無此人 => 建立新帳號並賦予預設值；若有 => 從 DB 還原其 `Position`, `Health` 與 `Inventory`。然後正式 `spawn` 這個玩家的 Entity，回傳成功並將狀態同步給該 Client。
4. **(Technical Artist) 進入遊戲**：Client 收到成功登入的封包（或看到代表自己的 Entity 出現），轉換狀態到 `AppState::InGame`，移除登入 UI 並開啟 HUD 與場景渲染。
5. **(Backend) 持久化存檔 (Save)**：在 Server 上設定一個定時系統 (e.g. `system_save_all_players`)，每 5~10 秒或玩家斷線時，將所有連在線上的 Player 資訊非同步地 `UPDATE` 寫回資料庫。

## 3. 資料結構定義
例如在 `shared/src/protocol.rs`：
```rust
use serde::{Deserialize, Serialize};
use bevy::prelude::*;

#[derive(Event, Serialize, Deserialize, Debug, Clone)]
pub struct LoginRequest {
    pub username: String,
    // MVP 暫不處理 password 加密驗證，單純靠名稱識別
}

#[derive(Event, Serialize, Deserialize, Debug, Clone)]
pub struct LoginResponse {
    pub success: bool,
    pub message: String,
}

// 供 Bevy 狀態機使用
#[derive(States, Default, Debug, Clone, PartialEq, Eq, Hash)]
pub enum AppState {
    #[default]
    LoginMenu,
    InGame,
}
```

## 4. 邊界提醒（Agent 絕對不能做的事）
- 🚫 **不要實作複雜的加密、信箱認證或密碼找回功能**。這是 MVP，玩家只要輸入任何字串，伺服器就會把它當成唯一帳號 ID。
- 🚫 **不要阻塞 (Block) Main Game Loop**。資料庫的讀寫都應該以 `Async` (如 `sqlx`) 或丟到後台緒池執行，禁止因為存檔而導致所有人畫面卡頓。
- 🚫 **不要設計太華麗的登入畫面**。用 Bevy 原生 UI 組件拉一個灰色方塊配上文字按鈕即可。

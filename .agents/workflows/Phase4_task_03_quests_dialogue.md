---
description: 任務與對話樹系統 (Quests & Dialogue Trees MVP)
---

# 任務藍圖：Phase 4 - Task 03 - Quests & Dialogue Trees MVP

**指派對象**：Backend Engineer Agent / Game Designer Agent / Narrative Designer Agent  
**目標**：為遊戲加上任務推動機制與 NPC 對話選項。讓玩家可以接取任務、回報任務進度，並且可以把狀態儲存到 SQLite 資料庫中維持持久化。

## 1. 涉及檔案清單
- `shared/src/components/npc.rs` (修改：加上對話節點(Dialogue Node) 的結構與任務種類標籤)。
- `shared/src/components/quest.rs` (新建：定義任務目標 (Objective)、任務狀態 (NotStarted, InProgress, Completed))。
- `shared/src/protocol.rs` (修改：新增 `InteractNpcIntent(NpcId, Option<DialogueChoice>)` 以及 `DialogueResponse`, `QuestUpdateEvent`)。
- `server/src/systems/npc.rs` (新建：處理 NPC 對話的狀態機，當收到 `InteractNpcIntent` 後給出不同的回應文本與選項)。
- `server/src/systems/quest.rs` (新建：當玩家殺死怪物，或者拿到特定數量的道俱時，更新任務進度，如果達標就推播 `QuestUpdateEvent`)。
- `server/src/db.rs` (修改：新增 `quests` 資料表，紀錄 `user_id`, `quest_id`, `status`, `progress`)。
- `client/src/systems/ui/dialog.rs` (修改：在對話框下方新增按鈕，顯示對話選項 `1. Yes 2. No`，點擊後發送 Intent)。
- `client/src/systems/ui/quest_log.rs` (新建：在畫面上方或右側顯示目前的任務追蹤清單，例如 `[1/5] Kill Goblins`)。

## 2. Step-by-Step 實作步驟
1. **(Database & Shared) 定義任務資料模型**：
   - 定義一個簡單的任務：`Quest { id: "kill_slimes", objective: KillEnemies("Slime", 3), reward: Gold(100) }`。
   - `server/src/db.rs` 新增 `create_table("quests")` 與載入玩家紀錄的方法。
2. **(Server) 處理對話分支邏輯 (`npc.rs`)**：
   - 將目前的單向 `DialogEvent(text)` 改為 `DialogueEvent { text, choices: Vec<String> }`。
   - 當跟商人講話時，出現選項 `[Buy Items]` 或 `[Accept Quest: Kill Slimes]`。
   - 客戶端選擇對應按鈕，送出 `InteractNpcIntent { choice_index: 1 }`，伺服器發給他接取任務。
3. **(Server) 任務進度監聽 (`quest.rs`)**：
   - 玩家接到任務後，身上加上一個 `ActiveQuests(Vec<QuestState>)` 元件。
   - 在原本的 `systems/combat.rs` (死亡邏輯) 或 `systems/loot.rs` 裡，加入掛鉤 (Hook)：如果怪物死了，丟給 `quest.rs` 檢查這隻怪是不是任務目標。是的話就進度 +1。
   - 數量滿了自動變更為 `Completed`，並更新資料庫。
4. **(Client) 對話按鈕與任務追蹤 UI**：
   - `DialogState` 現在要能顯示按鈕陣列。玩家可以用滑鼠點擊，或是按 `1`, `2` 快速鍵選取。
   - 在 UI 右上角開一個 `QuestLog` 區塊，列出目前正在解的任務進度。

## 3. 資料結構定義
例如客端 `shared/src/components/quest.rs`：
```rust
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum QuestId {
    KillSlimes,
    FindSword,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum QuestStatus {
    NotStarted,
    InProgress { count: u32 },
    ReadyToTurnIn,
    Completed,
}

#[derive(Component, Default, Debug, Clone)]
pub struct QuestTracker {
    pub active_quests: Vec<(QuestId, QuestStatus)>,
}
```

## 4. 邊界提醒（Agent 絕對不能做的事）
- 🚫 **嚴禁將對話與任務邏輯硬寫在 Client 端**。Client 無權知道對話內容或任務條件，一切選項與進度一律由伺服器派發 `DialogueEvent` 與 `QuestUpdateEvent` 決定。
- 🚫 **不要為了任務系統寫一套龐大的腳本解析器 (Lua/RON)**。為了 MVP 的穩固性，把第一個殺怪任務的邏輯用 Rust 原生程式碼寫死 (Hardcode) 在 Server 端即可，之後有工具工程師 (Tools Engineer) 再來將其資料驅動化。
- 🚫 **記得處理重新連線的狀態**。登入時必須把玩家資料庫裡的 `quests` 正確讀取出來並塞入 `QuestTracker`。

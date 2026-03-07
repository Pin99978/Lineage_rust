---
description: 怪物生成站與靜態 NPC (Spawners & Static Encounters)
---

# 任務藍圖：Phase 2 - Task 01 - World Spawners & NPCs

**指派對象**：Backend Engineer Agent / Game Designer Agent  
**目標**：讓遊戲世界從無意義的「隨機灑怪」進化為具備區域概念的持久化世界。實作 ECS `Spawner` 元件，讓伺服器能定時、定點重生特定數量的怪物。同時引進第一隻無陣營的不死 NPC (例如：雜貨商人)，為未來的商店系統打底。

## 1. 涉及檔案清單
- `shared/src/components/world.rs`（新建：定義 `Spawner`，包含生成哪一種怪物、範圍多大、最高重生數量、重生冷卻時間）。
- `shared/src/components/npc.rs`（新建：包含 `NpcType`、`Dialog` 等靜態 NPC 資訊）。
- `server/src/systems/spawner.rs`（新建：一個 `Update` 系統，負責掃描所有的 `Spawner`，計算該範圍內現存的怪物數量，若不足則經過冷卻時間後自動 `spawn` 新怪物）。
- `server/src/map_data.rs`（新建：將原本在 `main.rs` 裡寫死的怪，改為在此處初始化幾座 `Spawner` 與一位 `NPC`）。
- `client/src/systems/interaction.rs`（修改：當玩家滑鼠點擊 (Click) NPC 時，不再送出 `AttackIntent`，而是送出 `InteractIntent`）。
- `client/src/systems/ui.rs`（修改：實作極簡對話框，當收到 Server 傳來的對話文字時顯示在畫面上）。

## 2. Step-by-Step 實作步驟
1. **(Game Designer) 定義 Spawner 結構**：在 `shared` 中建立 `Spawner { entity_type: EntityType, radius: f32, max_count: u32, timer: Timer }`。此物不具體現身形，只是一個位於座標上的「重生點」。
2. **(Backend) 重生邏輯**：撰寫 `spawner_system`。對於每一個 `Spawner`，查詢該重生點目前追蹤生成的存活 Entity 數量。假如小於 `max_count`，且 `timer` 滴答完畢，則在給定的 `radius` 半徑內隨機座標生成一隻該類型的怪物。
3. **(Backend) NPC 與防護區**：在城鎮中心（如座標 `[0, 0]`）生成一個不會被怪物攻擊、也不會扣血的 `Merchant` NPC。
4. **(Backend & TA) 互動協議**：擴充 `shared/src/protocol.rs` 新增 `InteractIntent(target_entity)`。當 Client 點擊到 NPC，改發送此意圖。Server 收到後驗證距離，並回傳 `DialogEvent { text: "歡迎來到說話之島！" }` 給該 Client。
5. **(Technical Artist) 對話框**：在 Client 捕獲 `DialogEvent`，並在頂層 UI 生成一個帶有文字的視窗，過幾秒鐘後自動消失或點擊關閉。

## 3. 資料結構定義
例如在 `shared/src/components/world.rs`：
```rust
use bevy::prelude::*;

#[derive(Component, Debug, Clone, Reflect)]
pub struct Spawner {
    // 這裡可以使用某個 Enum，例如 EntityType::Goblin
    pub spawn_type: String, 
    pub max_count: u32,
    pub radius: f32,
    // 追蹤這個重生點目前已經活著的怪物 Entity 列表
    pub active_entities: Vec<Entity>, 
    pub cooldown: Timer,
}

#[derive(Component, Debug, Clone, Reflect)]
pub struct NpcMarker;

#[derive(Event, Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct InteractIntent(pub Entity);
```

## 4. 邊界提醒（Agent 絕對不能做的事）
- 🚫 **嚴禁實作真正的商店交易 (Shop UI & Currency Exchange)**。這個 MVP 只要能點擊 NPC 然後跳出一句對話文字 (`DialogEvent`) 就好，金錢增減與商店面板留到之後的里程碑。
- 🚫 **不要把 Spawner 的數量開太大**。為了避免滿圖亂走導致 Server 跑不動，`max_count` 預設最多 `3 ~ 5` 隻即可。
- 🚫 **不要讓怪物離開防禦半徑太遠 (Tethering)**。這個在此階段可不嚴格要求，但記得 `spawner_system` 只需要管生成，不需要強迫控制怪物追到天涯海角。

---
description: 多地圖切換與傳送點 (World Expansion & Portals MVP)
---

# 任務藍圖：Phase 4 - Task 02 - World Expansion & Portals MVP

**指派對象**：Backend Engineer Agent / Game Designer Agent / Technical Artist Agent  
**目標**：讓遊戲世界脫離單一地圖的限制，伺服器能夠同時管理多張地圖 (Maps) 與其實例 (Instances)，並讓玩家可以透過傳送點 (Portals) 或是指令在不同地圖間切換。此功能是擴展 MMORPG 世界觀的地基。

## 1. 涉及檔案清單
- `shared/src/components/world.rs` (修改：新增 `MapId` 元件，標記實體 (Entity) 目前位在哪一張地圖內。定義 `Portal` 元件，包含要傳送的目標地圖與座標)。
- `server/src/map_data.rs` (修改：把原本單一個 `CollisionGrid` 升級成 `MapManager`，裡面用 `HashMap<MapId, CollisionGrid>` 管理多張地圖的碰撞資訊)。
- `server/src/network.rs` (修改：在計算視角、廣播怪物/玩家封包時，**務必過濾出相同 `MapId` 的實體**，避免你在城鎮卻看到地城裡的人的封包)。
- `server/src/systems/interaction.rs` (修改：當玩家碰到 `Portal`，或者是跟特定 NPC (Teleporter) 講話時，改變他的 `MapId` 與 `Position`，並廣播 `MapChangeEvent` 讓他的客戶端載入新地圖)。
- `client/src/systems/render.rs` / `client/src/main.rs` (修改：攔截 `MapChangeEvent`，把先前的背景圖 (`map_bg.png`) 換成新的地圖圖片 (例如 `dungeon_bg.png`))。
- `shared/src/protocol.rs` (修改：新增 `MapChangeEvent(MapId, f32, f32)`)。

## 2. Step-by-Step 實作步驟
1. **(Shared) 定義 `MapId` 與修改封包**：
   - 包含玩家與怪物在內的所有 `Position` 擁有者，都必須加上 `MapId(String)`。
   - 在 `protocol::EntityState` 或是登入封包，都要寫入目前的地圖 ID。
2. **(Server) 地圖隔離與區域廣播 (Spatial Partitioning)**：
   - 目前 `network::broadcast_state_system` 是一次發給全服所有人。
   - 加上過濾：`if player_map_id == entity_map_id { send(entity_state) }`。這非常關鍵！不然會在同座標但不同圖的人疊在一起！
3. **(Server) 實作切換邏輯 (Teleportation)**：
   - 實作傳送門 `Portal { target_map: String, target_x: f32, target_y: f32 }`。
   - 寫一個 `portal_system`，每幀檢查有沒有 `Player` 的坐標與 `Portal` 的坐標非常接近 (例如 `< 2.0_f32`)，有的話就將他傳送過去，並立刻發給他 `MapChangeEvent` 封包，同時將原本圖內的他的 `NetworkEntityVisual` 發出 `EntityDespawn` 事件。
4. **(Client) 場景切換渲染 (Scene Loading)**：
   - Client 不用維護所有的碰撞網格，但需要知道什麼地圖載入什麼背景圖。
   - 收到 `MapChangeEvent` 後，把畫面上所有的其他玩家、怪物、物品全部清空 (`despawn_all_network_entities`)，換成新的 `Sprite` 背景，然後等 Server 送來新地圖的實體。

## 3. 資料結構定義
例如在客端 `shared/src/components/world.rs` 裡：
```rust
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Component, Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MapId(pub String);

#[derive(Component, Debug, Clone)]
pub struct Portal {
    pub target_map: String,
    pub target_x: f32,
    pub target_y: f32,
}
```

## 4. 邊界提醒（Agent 絕對不能做的事）
- 🚫 **不要為了這個系統一次載入太多張地圖**。只要實作兩張地圖即可：`"Town"`(城鎮) 與 `"Dungeon1"`(地城)。這兩張圖的背景可以給客戶端放不同的暫代圖片 (例如 `town.png` 跟 `dungeon.png`)。
- 🚫 **嚴禁忘記清空舊地圖實體**。Client 切換地圖時如果沒有刪除畫面上的其他玩家與怪物，就會帶著舊地圖的怪物「穿越」到新地圖上。一定要有一個 `clear_map_state` 的系統。

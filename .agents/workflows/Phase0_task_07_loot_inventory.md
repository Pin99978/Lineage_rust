---
description: 基礎掉落物與背包統 MVP (Loot & Basic Inventory)
---

# 任務藍圖：Task 07 - Loot Drops & Basic Inventory MVP

**指派對象**：Game Designer Agent / Backend Engineer Agent / Technical Artist Agent  
**目標**：打怪不能沒有獎勵！我們要在目前的戰鬥循環中加入「怪物死亡時掉落物品」的機制，並且允許玩家撿起物品，將其存入角色的 `Inventory` (背包) 中。為了保持白模 MVP 階段的單純，撿起物品後只需更新 UI 或在日誌中顯示即可。

## 1. 涉及檔案清單
- `shared/src/components/item.rs`（新建：定義 `Item` 種類，如金幣、藥水，以及 `Inventory` 組件）。
- `shared/src/protocol.rs`（修改：加入 `LootIntent` 事件，與 Server 廣播的 `ItemSpawnEvent`, `ItemDespawnEvent`）。
- `server/src/systems/drop.rs`（新建：監聽 `DeathEvent`，並根據機率在怪物死亡位置生成掉落物實體）。
- `server/src/systems/loot.rs`（新建：處理玩家的 `LootIntent`，驗證玩家與掉落物的距離，成功則刪除掉落物實體並更新玩家的 `Inventory`）。
- `client/src/systems/render.rs`（修改：為掉落物生成有別於玩家與怪物的特定顏色 Sprite（如金色方塊代表金幣）。
- `client/src/systems/input.rs`（修改：若點擊到掉落物，發送 `LootIntent`）。

## 2. Step-by-Step 實作步驟
1. **(Game Designer) 定義物品與背包**：在 `shared` 裡新增 `Item` (Enum：例如 Gold, Potion) 與 `Inventory` (類似 `HashMap<Item, u32>` 或 `Vec<Item>`)。同時定義怪物的 `LootTable` (掉落機率表)。
2. **(Backend) 怪物掉落邏輯**：Server 端建立 `item_drop_system`，監聽 `DeathEvent`。當怪物死亡時，機率性（或必定）在該座標生成具有 `Position` 且帶有 `Item` 屬性的「掉落物實體」，並網路同步給所有 Client。
3. **(Technical Artist) 視覺與撿取意圖**：Client 接收到掉落物實體後，將其渲染為「黃色小方塊」。擴充 Client 端的滑鼠點擊系統，若點擊目標是 `Item`，則對 Server 發送 `LootIntent(item_entity)`，而不是攻擊它。
4. **(Backend) 撿拾驗證與狀態更新**：Server 端建立 `loot_system` 處理 `LootIntent`。檢查玩家與該物品的 `Position` 是否夠近；若是，將物品加入該玩家的 `Inventory`，發送 `inventory_update`，並 `despawn` 場上的物品。
5. **(Technical Artist) 背包狀態顯示**：實作簡單的文字 UI 或透過 `println!` 提醒玩家撿到了什麼，確認撿寶流程暢通。

## 3. 資料結構定義
例如在 `shared/src/components/item.rs`：
```rust
use bevy::prelude::*;
use std::collections::HashMap;

#[derive(Component, Debug, Clone, PartialEq, Eq, Hash, Reflect)]
pub enum ItemType {
    Gold,
    HealthPotion,
}

#[derive(Component, Debug, Clone, Reflect)]
pub struct GroundItem {
    pub item_type: ItemType,
    pub amount: u32,
}

#[derive(Component, Default, Debug, Clone, Reflect)]
pub struct Inventory {
    pub items: HashMap<ItemType, u32>,
}
```

## 4. 邊界提醒（Agent 絕對不能做的事）
- 🚫 **不要製作複雜的網格背包 UI (Grid Inventory)**。現在只要確保數據有確實寫入 Server 的 `Inventory` 並且能簡單輸出文字（或單行 UI 字串）通知玩家即可。
- 🚫 **不要載入掉落物 icon 或特效**。請使用基礎形狀的 Sprite（如金黃色方形代表金幣、紅色小圓/小方形代表藥水）。
- 🚫 **嚴打 Client 信任**。掉落物是否在撿拾範圍內、是否已被別人撿走，必須由 Server 透過 `loot_system` 來判斷，Client 無權自己把東西收進背包。

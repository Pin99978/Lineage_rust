---
description: 進階戰鬥與裝備屬性系統 MVP (Advanced Combat & Equipment)
---

# 任務藍圖：Phase 1 - Task 02 - Advanced Combat & Equipment System

**指派對象**：Game Designer Agent / Backend Engineer Agent / Technical Artist Agent  
**目標**：結合 Option A 與 Option B。實作魔法值 (`Mana`)、快捷鍵技能施放，並引入裝備系統 (`Equipment`) 與屬性加成 (Modifier/Buffs)。所有的狀態包含裝備欄都必須同步持久化至資料庫。

## 1. 涉及檔案清單
- `shared/src/components/combat.rs`（修改：新增 `Mana`、`Spell` 以及防禦相關如 `ArmorClass` 組件）。
- `shared/src/components/item.rs`（修改：新增 `EquipmentSlot` 與 `StatModifier`，區分一般物品與可裝備物品）。
- `shared/src/protocol.rs`（修改：新增 `CastSpellIntent`, `EquipIntent`, `UnequipIntent` 等客戶端意圖）。
- `server/src/systems/spell.rs`（新建：處理魔法施放、判定距離、扣除 MP 與冷卻時間）。
- `server/src/systems/equipment.rs`（新建：處理裝備對 `CombatStats` 的穿脫加成重新計算）。
- `server/src/db.rs`（修改：在玩家存檔欄位增加 `equipment_json` 與 `mana` 的儲存與讀取）。
- `client/src/systems/input.rs`（修改：攔截鍵盤 1~4 數字鍵發送對應技能意圖；攔截背包 UI 操作發送裝備意圖）。
- `client/src/systems/ui.rs`（修改：新增藍色 MP 條，並印出簡單的「裝備清單」或裝備成功文字）。

## 2. Step-by-Step 實作步驟
1. **(Game Designer) 屬性與裝備結構**：重新定義角色體質，增加 `Mana { current, max }` 和 `ArmorClass`。建立 `EquipmentMap { weapon: Option<Item>, armor: Option<Item> }`。設計一個能掛載到玩家身上的通用「屬性加成 (Modifier)」邏輯（例如拔下劍就扣掉攻擊力）。
2. **(Game Designer) 單體技能法術**：設計一招名為 `Fireball` (單體傷害) 或 `Heal` (恢復生命) 的技能。定義其 `mana_cost`、`cooldown` 與 `range`。
3. **(Backend) 裝備與屬性加成系統**：在 Server 建立一套 `CombatStats` 的計算中樞。當收到 `EquipIntent(item_id)`，如果包包有該物品且符合欄位，將其移入 `EquipmentMap`，並觸發 `RecalculateStatsEvent`，更新攻擊力與防禦力。
4. **(Backend) 技能施放系統**：當收到 `CastSpellIntent` 時，先驗證玩家有沒有足夠的 `Mana` 並且不在 `cooldown` 狀態。驗證通過則扣除 MP，並將效果（傷害或補血）觸發在給定的 `Target` 身上，廣播新的 `DamageEvent` 或 `HealEvent`。
5. **(Backend) 資料庫整合**：記得將玩家的 `EquipmentMap` 以及 `Mana` 數值同步到 `server/src/db.rs` 中的 `UPDATE` 語句，並在登入時還原裝備加成。
6. **(Technical Artist) 視覺與交互**：在螢幕左下角新增藍色的 MP Bar。利用鍵盤按下鍵盤對應 Server 發送施法事件，透過文字或控制台印出「施放 XXX，剩餘 MP: XX」。裝備成功時也做出對應的 UI 回饋。

## 3. 資料結構定義
例如在 `shared/src/components/combat.rs` 與 `item.rs`：
```rust
use bevy::prelude::*;
use std::collections::HashMap;

#[derive(Component, Debug, Clone, Reflect)]
pub struct Mana {
    pub current: i32,
    pub max: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Reflect)]
pub enum EquipmentSlot {
    Weapon,
    Armor,
}

#[derive(Component, Default, Debug, Clone, Reflect)]
pub struct EquipmentMap {
    pub slots: HashMap<EquipmentSlot, Entity>, // 記錄裝在哪個槽的實體，或是以 UUID 記錄
}

#[derive(Component, Debug, Clone, Reflect)]
pub struct StatModifier {
    pub attack_power_bonus: i32,
    pub armor_class_bonus: i32,
}
```

## 4. 邊界提醒（Agent 絕對不能做的事）
- 🚫 **嚴禁實作飛行軌跡或物理碰撞彈道**。目前的 `Fireball` 或 `Heal` 是立刻命中的「鎖定技能（Targeted Spells）」。魔法只要放出去，Server 算距離對了就直接扣血，不要寫碰撞箱。
- 🚫 **不要為了裝備系統寫複雜的紙娃娃 (Paper Doll) 換裝繪圖**。角色依然是那塊原始的顏色的方塊，只要數值計算正確、能存入 DB 就好。
- 🚫 **不要違反 Phase 1 的 SOLID 規則**。請使用 `Result` 與 `.clamp()`，不要在算錯加成時 `unwrap`，屬性更新請用專屬的 Event (例如 `StatsChangedEvent`)，而不是用一個肥大的系統掃全圖。

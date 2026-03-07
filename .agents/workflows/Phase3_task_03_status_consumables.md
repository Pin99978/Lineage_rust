---
description: 狀態效果與消耗品快捷鍵 (Buffs, Status Effects & Consumables)
---

# 任務藍圖：Phase 3 - Task 03 - Buffs, Status Effects & Consumables

**指派對象**：Backend Engineer Agent / Game Designer Agent  
**目標**：為戰鬥增加深度與操作空間，讓玩家可以喝藥水補血，或者受到怪物攻擊時獲得各式各樣的狀態效果 (Buff/Debuff)。

## 1. 涉及檔案清單
- `shared/src/components/combat.rs` (修改：新增 `StatusEffect` 結構與 `Buffs` 元件，用來儲存當前的所有增益/減益狀態)。
- `server/src/systems/combat.rs` (修改：在計算傷害前，套用 `Buffs` 影響；在受到特定攻擊時，為角色加上 `StatusEffect`)。
- `server/src/systems/item.rs` (新建/修改：實作消耗品 `Consumable` 使用邏輯，例如點擊/快捷鍵使用 `HealthPotion` 來補血)。
- `shared/src/protocol.rs` (修改：新增 `UseItemIntent` 來通知伺服器玩家使用了某個道具。新增 `StatusEffectUpdate` 給客戶端更新 UI)。
- `client/src/systems/ui.rs` (修改：左下方的快捷鍵 1~4 開始運作，綁定背包內的消耗品或技能，並且顯示 CD 冷卻時間；在血條旁邊顯示角色當前的狀態 Icon 或文字)。

## 2. Step-by-Step 實作步驟
1. **(Shared) 定義狀態效果結構 (`Buffs` / `StatusEffect`)**：
   - 建立 `StatusEffect` struct，包含：`effect_type` (如 Poison, SpeedUp, AttackUp), `duration` (剩餘時間), `value` (效果數值)。
   - 為角色 (Player & Enemy) 加上 `Buffs(Vec<StatusEffect>)` 元件。
2. **(Server) 處理狀態效果隨時間的變化 (Tick)**：
   - 新增一個 `update_status_effects` 系統，負責每幀扣除 `duration`。
   - 如果是 Poison (持續傷害)，每 1 秒讓 `Health` 減少。
   - 當 `duration` 歸零時，從清單移除該效果。
3. **(Shared & Server) 完善消耗品機制 (`UseItemIntent`)**：
   - `MoveIntent`, `AttackIntent` 之外，加入 `UseItemIntent`，讓玩家可以主動使用背包內的物品。
   - `server/src/systems/item.rs` 攔截 `UseItemIntent`，減少背包數量，並為玩家回復對應的血量 (如 `HealthPotion` 補 20 滴血)。
4. **(Client) 顯示狀態列與綁定快捷鍵**：
   - 把畫面上原本寫死的 `1 Fireball | 2 Heal | E/R Equip` 快捷鍵列，真的與鍵盤事件 (`systems/input.rs`) 綁定。
   - 讓玩家按下 `1` 或 `2` 可以發送 `UseItemIntent` 給伺服器。
   - 收到 `StatusEffectUpdate` 封包後，在 HUD 面板顯示正在中毒、或是有增益狀態。

## 3. 資料結構定義
例如在客端 `shared/src/components/combat.rs` 中：
```rust
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EffectType {
    Poison,
    Regen,
    SpeedUp,
    AttackUp,
    DefenseDown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusEffect {
    pub effect_type: EffectType,
    pub duration_remaining: f32, // 秒數
    pub tick_timer: f32,         // 用於觸發每秒傷害/回血的內部計時器
    pub strength: f32,
}

#[derive(Component, Default, Debug, Clone)]
pub struct Buffs {
    pub effects: Vec<StatusEffect>,
}
```

## 4. 邊界提醒（Agent 絕對不能做的事）
- 🚫 **嚴禁在 Client 端直接修改血量或增加 Buff**。狀態效果的所有邏輯與傷害計算必須交給 Server 負責，Client 只負責發送 `UseItemIntent` 並接收 `EntityState` 與 `StatusEffectUpdate` 來顯示 UI。
- 🚫 **不要為了每一個 Buff 而寫出龐大的 Switch-Case 到幾千行**。使用模塊化的設計：可以有一個 `apply_healing` 函數跟一個 `apply_damage` 函數供不同的 Effect 呼叫。
- 🚫 **不要一次實作太多狀態跟消耗品**。目前只要實作：`HealthPotion` (立即回血) 跟 `Poison` (持續扣血) 這兩個足以演示機制的 MVP 即可。

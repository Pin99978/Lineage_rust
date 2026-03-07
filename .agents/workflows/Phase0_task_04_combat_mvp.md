---
description: 基礎戰鬥與站樁假人 MVP
---

# 任務藍圖：Task 04 - Basic Combat & Identity MVP

**指派對象**：Game Designer Agent / Backend Engineer Agent / Technical Artist Agent  
**目標**：在 Bevy 0.18 實作基本戰鬥循環。允許玩家點擊目標進行攻擊判定，Server 計算扣血與死亡，Client 顯示攻擊狀態、傷害跳字及死亡表現。

## 1. 涉及檔案清單
- `shared/src/components/combat.rs`（新建：定義血量、攻擊力、冷卻時間等屬性）。
- `shared/src/protocol.rs`（修改：加入 `AttackIntent` 或 `ActionIntent` 事件，與伺服器廣播的 `DamageEvent`, `DeathEvent`）。
- `server/src/systems/combat.rs`（新建：處理攻擊距離驗證、扣除 HP、發布死亡事件）。
- `server/src/main.rs`（修改：生成一個或多個「站樁假人 (Target Dummy)」供玩家攻擊）。
- `client/src/systems/input.rs`（修改：若點擊到包含可攻擊元件的 Entity，則發送 `AttackIntent` 而非 `MoveIntent`）。
- `client/src/systems/combat_render.rs`（新建：接收 Server 的 `DamageEvent`，顯示傷害跳字，或將死亡的 Entity 改變顏色/消滅）。

## 2. Step-by-Step 實作步驟
1. **(Game Designer) 狀態與屬性設計**：在 `shared/` 建立 `Health { current, max }`、`CombatStats { attack_power, attack_range, attack_speed }` 等 Component。
2. **(Backend) 創建假人與攻擊邏輯**：Server 啟動時在場景中生成帶有 `Health` 屬性的假人。實作 Server 端的 `combat_system`：當收到 Client 的 `AttackIntent(target_entity)` 時，檢查兩者距離是否在 `attack_range` 內，若是則扣除目標 `Health`，並向該區域廣播 `DamageEvent { target, amount }`。若 `Health` <= 0，廣播 `DeathEvent { target }`。
3. **(Technical Artist) 目標選取與輸入判斷**：Client 在滑鼠點擊時，需透過 Bevy 的鼠標射線 (Raycast) 或是簡單的座標與 AABB 距離轉換，判斷是否點擊到「可被攻擊的 Entity」。若是，發送 `AttackIntent`。
4. **(Technical Artist) 視覺回饋**：實作 `combat_render_system`，監聽來自 Server 的 `DamageEvent` 與 `DeathEvent`。當扣血時，可在目標位置生成一個短暫往上飄的文字 (Text2dBundle) 顯示傷害；當死亡時，將白模 Sprite 染成灰色或直接 `despawn`。

## 3. 資料結構定義
例如在 `shared/src/components/combat.rs`：
```rust
use bevy::prelude::*;

#[derive(Component, Debug, Clone, Reflect)]
pub struct Health {
    pub current: i32,
    pub max: i32,
}

#[derive(Component, Debug, Clone, Reflect)]
pub struct CombatStats {
    pub attack_power: i32,
    pub attack_range: f32,
    pub attack_speed: f32, // 次/秒
}

#[derive(Component, Debug, Clone, Reflect)]
pub struct ActionState {
    pub is_attacking: bool,
    pub target: Option<Entity>,
}
```

## 4. 邊界提醒（Agent 絕對不能做的事）
- 🚫 **嚴禁實作複雜的範圍技能 (AOE) 或投射物 (Projectile)**。這是 MVP，只做「近戰鎖定 / 單體點擊扣血」。
- 🚫 **不要載入特效圖片或是 Sprite Sheet 動畫**。美術資源依舊使用純色白模（或純文字 `Text2d`）。
- 🚫 **不要將扣血與死亡邏輯寫在 Client**。Damage 和 Death 的權威永遠在 Server 端，Client 只負責依據 Server Event 做顯示。

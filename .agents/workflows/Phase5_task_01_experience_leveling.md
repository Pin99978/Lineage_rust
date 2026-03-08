---
description: 經驗值與升級系統 (Experience & Leveling MVP)
---

# 任務藍圖：Phase 5 - Task 01 - Experience & Leveling MVP

**指派對象**：Backend Engineer Agent / Game Designer Agent  
**目標**：為遊戲導入最核心的 RPG 進度驅動：**打怪 -> 獲取經驗值 (EXP) -> 升級 (Level Up) -> 屬性成長**。讓玩家的努力能即時反饋在角色強度上。

## 1. 涉及檔案清單
- `shared/src/components/world.rs` 或 `combat.rs` (修改：在玩家身上加入 `Level`, `Experience` 元件)。
- `server/src/db.rs` (修改：在 `users` table 裡加入 `level`, `exp` 欄位，確保玩家登出後不會被打回 1 級)。
- `server/src/systems/combat.rs` (修改：在怪物死亡判定時，根據怪物的強度配發 `EXP` 給擊殺者及周圍的隊友。加上偵測：如果 `EXP > max_exp` 則觸發升級)。
- `shared/src/protocol.rs` (修改：新增推播封包 `LevelUpEvent` 與 `ExpUpdateEvent`)。
- `client/src/systems/ui/paperdoll.rs` (修改：在角色屬性面板顯示目前的 Level 與 EXP 百分比)。
- `client/src/systems/render.rs` (修改：當收到 `LevelUpEvent` 時，在角色頭上播放一個黃金閃光或是「Level Up」的浮空文字，增加視覺爽感)。

## 2. Step-by-Step 實作步驟
1. **(Shared) 建立經驗值公式與結構**：
   - 定義 `Level { current: u32 }` 與 `Experience { current: u32, next_level_required: u32 }`。
   - 設計簡單的升級公式：`next_level = base_exp * (level ^ 1.5)`。
2. **(Server) 處理經驗獲取邏輯**：
   - 在 `combat.rs` 的 `apply_damage` 造成實體死亡時，如果死掉的是 `NpcType::Monster`，則呼叫 `grant_exp(player_entity, exp)`。
   - 檢查並觸發升級：升級時，把 `mana` 與 `health` 補滿，並給玩家增加基本屬性（例如血量上限 +10，攻擊力 +2）。
3. **(Client) UI 面板與特效反饋**：
   - HUD 下方新增一條細細的「經驗值進度條」。
   - 客戶端攔截 `LevelUpEvent`，並掛上短暫的特效組件播放動畫。

## 3. 資料結構定義
```rust
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Component, Debug, Clone, Serialize, Deserialize)]
pub struct Level {
    pub value: u32,
    pub stat_points: u32, // 可分配的屬性點
}

#[derive(Component, Debug, Clone, Serialize, Deserialize)]
pub struct Experience {
    pub current: u32,
    pub next_level_req: u32,
}
```

## 4. 邊界提醒（Agent 絕對不能做的事）
- 🚫 **嚴禁將經驗值計算邏輯放在 Client 端**。Client 只能收到「你現在 EXP 多少了」的結果，不能自己宣告「我要升級了」。
- 🚫 **為保持 MVP 單純，先不要實作太複雜的天賦樹 (Skill Tree)**。只要達到「升級 -> 血量變多、傷害變痛、跑出特效」的循環，就足以驗證進度系統。

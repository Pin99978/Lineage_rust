---
description: 經驗值與升級系統 (Experience & Leveling MVP) + 類天堂法術系統
---

# 任務藍圖：Phase 5 - Task 01 - Experience & Leveling (Lineage-Style)

**指派對象**：Backend Engineer Agent / Game Designer Agent  
**目標**：為遊戲導入「打怪 -> 獲取經驗值 (EXP) -> 升級 (Level Up) -> 屬性成長」機制。並且仿照《天堂》的法術學習系統：**只要等級到達門檻，搭配法術書或特定道具，即可施放該階級的魔法。** 不採用 Diablo 那種複雜的技能樹與點數分配。

## 1. 涉及檔案清單
- `shared/src/components/combat.rs` (修改：在玩家身上加入 `Level`, `Experience`, `BaseStats` 元件，並新增一個 `LearnedSpells` 清單)。
- `server/src/db.rs` (修改：在 `users` table 裡加入 `level`, `exp`, `str`, `dex`, `int`, `con` 等基礎屬性欄位)。
- `server/src/systems/combat.rs` (修改：殺怪配發 `EXP`。如果 `EXP > max_exp` 觸發升級，給予額外的屬性點數 `stat_points`)。
- `server/src/systems/spell.rs` (修改：在施放 `Fireball` 等法術前，檢查 `player.level >= spell.req_level` 以及 `player.learned_spells.contains(spell_id)`).
- `client/src/systems/ui/paperdoll.rs` (修改：在角色屬性面板顯示目前的 Level、EXP 百分比，以及 STR/DEX 等屬性，如果 `stat_points > 0`，提供 [+] 按鈕供玩家分配)。
- `shared/src/protocol.rs` (修改：新增 `LevelUpEvent`, `ExpUpdateEvent`, `AllocateStatIntent`)。

## 2. Step-by-Step 實作步驟
1. **(Database & Shared) 建立基礎屬性與經驗值公式**：
   - 定義 `Level { current: u32, stat_points: u32 }`。
   - 定義基礎六大屬性（可先實作 `STR`, `DEX`, `INT`, `CON` 四個即可）。
   - 設計升級公式：`next_level = base_exp * (level ^ 1.5)`。
2. **(Server) 處理經驗獲取與手動配點**：
   - 殺怪給 EXP。滿了就發 `LevelUpEvent`，`Level.current += 1` 並且 `Level.stat_points += 1`。
   - 接收 `AllocateStatIntent(StatType)`，檢查 `stat_points > 0`，然後將對應屬性 +1，立刻回滿血魔，重新計算最大 HP/MP (例如 CON 影響 HP，INT 影響 MP)。
3. **(Server) 類《天堂》法術門檻判定**：
   - 不搞技能樹。假設 `Fireball` 是一階魔法 (需要 Lv 4)，`Heal` 是一階魔法 (需要 Lv 4)。
   - 在 `spell.rs` 的施放邏輯加上：`if player.level < 4 { return Error("Level too low"); }`。
   - (Optional MVP) 先設定為：只要等級到了，自動習得該等級對應的所有法術（不強求必須吃法術書才能學，降低 MVP 難度）。
4. **(Client) UI 面板與特效**：
   - 畫面底端顯示經驗值進度條。
   - 角色面板顯示力量、敏捷等數值，升級時顯示配點按鈕 `[+]`。
   - 攔截 `LevelUpEvent`，並在角色頭上播放視覺特效。

## 3. 資料結構定義
```rust
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum StatType { Str, Dex, Int, Con }

#[derive(Component, Debug, Clone, Serialize, Deserialize)]
pub struct BaseStats {
    pub str: u32,
    pub dex: u32,
    pub int: u32,
    pub con: u32,
}

#[derive(Component, Debug, Clone, Serialize, Deserialize)]
pub struct Level {
    pub current: u32,
    pub stat_points: u32,
}

#[derive(Component, Debug, Clone, Serialize, Deserialize)]
pub struct Experience {
    pub current: u32,
    pub next_level_req: u32,
}
```

## 4. 邊界提醒（Agent 絕對不能做的事）
- 🚫 **不要實作暗黑破壞神 (Diablo) 那種相依技能樹**。法術沒有前置條件，只有「等級門檻」（以及未來可能需要的特定職業限制）。
- 🚫 **嚴禁讓 Client 決定升級或屬性計算**。所有能力值加成、最大 HP 增長公式都在 Server 端算好，再透過 `EntityState` 同步給 Client。

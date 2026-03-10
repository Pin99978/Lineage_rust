---
description: 經驗值與升級系統 (Experience & Auto-Leveling MVP) + 類天堂法術系統
---

# 任務藍圖：Phase 5 - Task 01 - Experience & Auto-Leveling (Lineage-Style)

**指派對象**：Backend Engineer Agent / Game Designer Agent  
**目標**：為遊戲導入「打怪 -> 獲取經驗值 (EXP) -> 升級 (Level Up) -> 自動屬性成長」機制。仿照《天堂》的法術學習與升級系統：升級時伺服器自動根據角色的基礎屬性（如 CON）給予額外的血量/魔力上限增長。只要等級到達門檻，即可放該階級的魔法。不採用手動配點。

## 1. 涉及檔案清單
- `shared/src/components/combat.rs` (修改：加上 `Level`, `Experience`, `BaseStats` 元件，包含 STR, DEX, INT, CON 等經典屬性)。
- `server/src/db.rs` (修改：`users` table 擴增 `level`, `exp`, 以及 `str`, `dex`, `int`, `con` 欄位)。
- `server/src/systems/combat.rs` (修改：怪物死亡發送 EXP，若滿經驗則發動升級。升級時根據 `CON` 給予額外 HP 上限，根據 `INT` 給予額外 MP 上限，滿血滿魔)。
- `server/src/systems/spell.rs` (修改：施法前檢查 `player.level >= spell.req_level`)。
- `client/src/systems/ui/paperdoll.rs` (修改：角色屬性面板單純顯示目前的 Level、EXP 進度，以及 STR/DEX 等屬性與 MaxHP/MaxMP，不需 `[+]` 配點按鈕)。
- `shared/src/protocol.rs` (修改：新增 `LevelUpEvent`, `ExpUpdateEvent`)。

## 2. Step-by-Step 實作步驟
1. **(Database & Shared) 定義基礎屬性與經驗值公式**：
   - 定義 `Level { current: u32 }`。
   - 定義 `BaseStats { str: u32, dex: u32, int: u32, con: u32 }`。
   - 設計簡單升級公式：`next_level = base_exp * (level ^ 1.5)`。
   - 在創角（或初次給予 Db 預設值時），寫死一組平均的基礎屬性（例如全部為 15）。
2. **(Server) 處理經驗獲取與自動成長 (Auto-growth)**：
   - 打死怪物給予 EXP。滿了就觸發 `LevelUpEvent`，`Level.current += 1`。
   - **自動成長邏輯**：升級時，查閱玩家的 `con`，例如 `hp_gain = random(con/2, con)`，將玩家的 `max_health += hp_gain`。同理 `max_mana` 受 `int` 影響。
3. **(Server) 類《天堂》法術門檻判定**：
   - 在 `spell.rs` 的施放邏輯內加上 `if player.level < spell.req_level { return Error; }`。
4. **(Client) UI 面板與特效**：
   - 畫面底端顯示經驗值進度條 (`ExpUpdateEvent`)。
   - 角色面板 (Paperdoll) 顯示能力參數 (從 `EntityState` 中刷新)。
   - 攔截 `LevelUpEvent`，並在角色上方播放一陣升級的光芒或浮動文字。

## 3. 資料結構定義
```rust
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

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
}

#[derive(Component, Debug, Clone, Serialize, Deserialize)]
pub struct Experience {
    pub current: u32,
    pub next_level_req: u32,
}
```

## 4. 邊界提醒（Agent 絕對不能做的事）
- 🚫 **嚴禁製作手動配點 UI**。升級就是自動根據屬性成長。
- 🚫 **先別管轉職或重置屬性**。MVP 先寫死一套通用的成長公式。

---
description: HUD 介面與血條 (Health Bar) 渲染 MVP
---

# 任務藍圖：Task 06 - HUD & Health Bars MVP

**指派對象**：Technical Artist Agent / Game Designer Agent  
**目標**：既然現在怪物會追殺並且攻擊玩家了，我們需要在畫面提供正確的視覺回饋。實作簡單的玩家本身血條 UI、以及怪物實體頭頂的血條，並讓它們會根據 Server 傳來的狀態動態更新。

## 1. 涉及檔案清單
- `client/src/systems/ui.rs`（新建：處理 Bevy UI 節點的生成與更新）。
- `client/src/main.rs`（修改：註冊 UI 的系統與載入預設字型）。
- `client/src/systems/combat_render.rs`（修改：加上在對手頭頂生成 2D 血條的邏輯）。
- `shared/src/protocol.rs`（可能不需要修改，依賴之前的 `DamageEvent`，或由 Server 定期 Sync 全部實體的 Health）。

## 2. Step-by-Step 實作步驟
1. **(Technical Artist) 玩家主 UI (HUD)**：在 `client/src/systems/ui.rs` 裡的 `setup_ui` 生成經典的底部/頂部狀態列。使用 `NodeBundle` 設定簡單的背景，搭配內部紅色的 `NodeBundle` 當作血條（Health Bar）。設定為絕對定位 (Absolute)。
2. **(Game Designer / Backend) 確保玩家資訊同步**：原本伺服器可能只有扣血時發 `DamageEvent`。若需要初始血量或最大血條同步，可以在連線時或 `Health` 改變時，Sync 自己的最新狀態。
3. **(Technical Artist) 動態更新 HUD**：加入一個 system，每當偵測到「屬於玩家自己」的 `Health` Component（或者相關網路屬性）發生變化時，動態調整紅色血條的 `Style.width` 比例（`current / max * 100.0%`）。
4. **(Technical Artist) 怪物頭頂血條 (World space bar)**：在渲染怪物的實體上，`spawn` 加入子實體 (Child entity)，包含簡單的 Sprite（細長條，背景黑、前景紅），或者使用 `billboard`，並撰寫一個 `update_world_health_bars` 系統，根據實體當前血量比例更動長度。當血量為滿時可以選擇隱藏。

## 3. 資料結構定義
Client 端可能新增一些輔助用的 Marker Component：
```rust
use bevy::prelude::*;

#[derive(Component)]
pub struct PlayerHealthBarUi; // 標記畫面上定死的玩家 UI 血條

#[derive(Component)]
pub struct WorldHealthBar {  // 標記在世界中 (World Space) 的實體血條
    pub parent_entity: Entity,
}
```

## 4. 邊界提醒（Agent 絕對不能做的事）
- 🚫 **不要載入精緻的 UI 圖片或九宮格邊框**。MVP 階段全部使用 Bevy 原生的 `NodeBundle` 和背景顏色設定即可。
- 🚫 **不要實作背包或技能樹介面**。我們專注於「顯示玩家血量」跟「顯示目標頭頂血量」。
- 🚫 **不要將 UI 重繪頻率寫得太高而拖慢效能**。只要在 `Health` 或 `DamageEvent` 觸發時再更新即可（使用 Changed Query）。

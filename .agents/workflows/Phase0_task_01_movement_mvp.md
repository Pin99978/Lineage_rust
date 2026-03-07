---
description: 實作基本的角色移動 (Whitebox MVP)
---

# 任務藍圖：Task 01 - Movement MVP

**指派對象**：Coding Agent (Codex) / Backend Engineer & Technical Artist  
**目標**：在 Bevy 0.18 環境下，實作無素材（白模）的基礎角色移動 MVP。

## 1. 涉及檔案清單
- `shared/src/components/movement.rs`（新建：定義共用元件）
- `shared/src/lib.rs`（修改：註冊元件與模組導出）
- `client/src/systems/input.rs`（新建：處理玩家輸入並發布移動意圖）
- `server/src/systems/movement.rs`（新建：處理移動邏輯與狀態同步 [在此次 MVP 若只需客戶端驗證，可只寫在 client]）
- `client/src/main.rs`（修改：註冊系統與生成測試用的白模實體）

## 2. Step-by-Step 實作步驟
1. **建立資料結構**：在 `shared/src/components/movement.rs` 建立 `Position` 與 `TargetPosition` component，並透過 App 註冊與 Reflect。
2. **初始化測試場景**：在 `client/src/main.rs` 裡的 `setup` 系統中，生成一個 2D `Camera2d` 與一個代表玩家的「實心色塊（白模）」（利用 `Sprite` 設定純色與大小，配合 `Transform`）。
3. **輸入捕捉系統**：在 `client/src/systems/input.rs` 以 `ButtonInput<MouseButton>` 讀取滑鼠左鍵點擊，透過 `Camera` 計算出正確的世界座標，並更新玩家實體的 `TargetPosition`。
4. **移動插值系統**：建立一個 `movement_system`，讓具備 `MoveSpeed` 的實體從當前位置逐漸朝 `TargetPosition` 直線移動，並套用 `Time::delta_secs()` 確保幀率獨立。
5. **視覺同步系統**：若將邏輯與視覺分離，最後需建立一個 system 將最新的邏輯座標更新至 `Transform.translation` 供 Bevy 渲染。

## 3. 資料結構定義
請在 `shared/src/components/movement.rs` 中實作以下 Struct：

```rust
use bevy::prelude::*;

#[derive(Component, Default, Debug, Clone, Copy, PartialEq, Reflect)]
#[reflect(Component, Default)]
pub struct Position {
    pub x: f32,
    pub y: f32,
}

#[derive(Component, Default, Debug, Clone, Copy, PartialEq, Reflect)]
#[reflect(Component, Default)]
pub struct TargetPosition {
    pub x: f32,
    pub y: f32,
}

#[derive(Component, Default, Debug, Clone, Copy, PartialEq, Reflect)]
#[reflect(Component, Default)]
pub struct MoveSpeed {
    pub value: f32,
}
```

## 4. 邊界提醒（Codex 絕對不能做的事）
- 🚫 **嚴禁載入任何外部圖片素材**（如 `asset_server.load("player.png")`）。目前為白模階段，請**唯一**使用 Bevy 的原生方形（如自訂 color 與 custom_size 的 `Sprite`）。
- 🚫 **禁止使用 Bevy 的過時 API**。專案規範為 Bevy **0.18**，請確保符合最新的 ECS 語法與寫法。
- 🚫 **不要實作複雜的 A* Pathfinding** 或地形阻擋。這純粹是個驗證基礎 ECS 的直線移動 MVP。
- 🚫 **禁止將所有功能混裝於單一檔案**。請嚴格遵守分層架構，元件定義必須於 `shared/`，客戶端行為必須於 `client/`。

---
description: 視覺革命與預渲染資產 MVP (Visual Polish & Pre-rendered Atmosphere)
---

# 任務藍圖：Phase 1 - Task 03 - Visual Polish & Pre-rendered Assets MVP

**指派對象**：Technical Artist Agent  
**目標**：隨著戰鬥與資料庫系統的完善，我們終於可以解除「白模限制 (Whitebox MVP)」！在此階段，我們將導入真正的《天堂》預渲染 2.5D 美術風格：實作角色 8 方向動畫、加入高解析度 2D 地圖背景圖片，並透過 Bevy 0.18 的 HDR、Bloom 來強化暗黑中世紀氛圍。

## 1. 涉及檔案清單
- `.agents/rules/game_rules`（修改：解除 `NO ART ASSETS` 限制，開放 `client/assets/` 使用）。
- `client/src/systems/render.rs` 或 `client/src/systems/animation.rs`（新建：實作 8 方向 SpriteSheet 動畫控制器與 TextureAtlas 載入）。
- `client/src/main.rs` & `client/src/systems/camera.rs`（修改：為相機加上 Bloom、HDR，並把純色背景換成真正載入的 Image 背景）。
- `client/assets/`（新增：放置測試用的角色 SpriteSheet (`player.png`) 以及測試地圖 (`map_bg.png`)。此工作由開發者或美術提供，TA 負責撰寫載入邏輯）。

## 2. Step-by-Step 實作步驟
1. **(Technical Artist) 解除白模限制**：修改或確認團隊規範，開放 Client 使用 `asset_server.load()` 讀取圖片素材。
2. **(Technical Artist) 環境氛圍 (Atmosphere)**：修改 `setup` 裡頭的 Camera，開啟 `hdr: true`，並掛上 `BloomSettings`。可以微微調整色調或加入 Vignette（暗角），營造暗黑奇幻感。將原本的「大面積純色白模背景」替換為 `asset_server.load("textures/map_bg.png")`。
3. **(Technical Artist) 角色動畫系統 (Animation Controller)**：建立 `AnimationTimer` 與 `AnimationState (Idle, Walk, Attack)` Component。在 Client 端實作 `animate_sprite_system`，讀取 `TextureAtlas` 進行逐幀播放。
4. **(Technical Artist) 8 方向邏輯**：撰寫一個輔助系統。當收到 Server 傳來的座標更新（代表角色正在移動）時，計算出 X 與 Y 的移動向量 (e.g., dx, dy)。根據這個向量算出角度，對應到 SpriteSheet 的 8 個不同方向的 Row（例如上、下、左、右及四個斜角）。
5. **(Technical Artist) 從系統事件觸發動畫**：當玩家處於移動狀態，切換為 Walk 動畫；當接收到來自 Server 的 `AttackIntent` 或 `DamageEvent`，讓攻擊者播放 Attack 動畫。

## 3. 資料結構定義
例如在客端 `client/src/systems/animation.rs` 可能會新增：
```rust
use bevy::prelude::*;

#[derive(Component)]
pub struct AnimationController {
    pub timer: Timer,
    pub current_frame: usize,
    pub start_frame: usize,
    pub end_frame: usize,
    pub direction_row: usize, // 用於 8 方向對應行數
}

#[derive(Component, Default, PartialEq, Eq)]
pub enum CharacterState {
    #[default]
    Idle,
    Walking,
    Attacking,
    Dead,
}
```

## 4. 邊界提醒（Agent 絕對不能做的事）
- 🚫 **不要把邏輯與動畫綁死**。即使動畫沒有播放完成，只要 Server 判定玩家死掉，玩家就是死掉。動畫只是對 Server 狀態的「視覺反饋」。
- 🚫 **嚴禁 Server 端碰觸圖片資源**。Server `Headless` 的設計絕對不能因為載入不到 `map_bg.png` 而崩潰。所有的 `TextureAtlas` 與 Bloom 都只能在 `client/` 中生存。
- 🚫 **處理缺圖的例外 (Fallback)**。如果 `client/assets/textures/player.png` 還沒放進去，不要 Panicked！請準備好 `Result` 或預設粉紅色方形，保證開發期間無論美術進度到哪，Codex 的程式都能跑。

---
description: 實作 2.5D NavMesh 移動與 Y-Sorting (Pre-rendered 視覺風格 MVP)
---

# 任務藍圖：Task 03 - 2.5D Movement & Y-Sorting MVP

**指派對象**：Technical Artist Agent / Coding Agent (Codex)  
**目標**：在 Bevy 0.18 環境下，套用《天堂》、《暗黑破壞神2》的 2.5D 預渲染美術風格邏輯。放棄傳統 Tilemap，改用 NavMesh (或單純多邊形邊界) 限制移動範圍，並實作基於 Y 軸的深度排序（Y-Sorting）。

## 1. 涉及檔案清單
- `Cargo.toml` (Workspace / client / server)（修改：加入 `bevy_pathmesh` 或自製簡單多邊形碰撞檢測機制）。
- `client/src/systems/render.rs` (新建/修改：實作 Y-Sorting，動態根據實體的 Y 座標調整 Z 軸）。
- `client/src/main.rs`（修改：建立一個作為地面背景的超大 Sprite，取代網格，並掛載 Y-Sorting 系統）。
- `server/src/systems/movement.rs` 或 `shared/src/movement.rs`（修改：在 Server 端依據 NavMesh 阻擋或修正玩家從 `A` 移動到 `B` 的路徑）。

## 2. Step-by-Step 實作步驟
1. **背景佈置 (Pre-rendered Background)**：在 `client` 的 `setup` 裡，生成一張極大尺寸的純色 (或黑白網格圖樣) Sprite（例如 `custom_size: Some(Vec2::new(2000.0, 2000.0))`），代表未來的整張高解析度 2D 背景圖片。將其 Z 軸設為極低 (如 -100.0)。
2. **Y-Sorting 系統**：在 `client` 實作一個 `y_sorting_system`。對於所有包含特定標籤（例如 `YYSortable` 或直接查閱 `Player`/`Entity` 的 Transform）的實體，在 `Update` 階段即時計算 `transform.translation.z = -transform.translation.y * 0.001`。
3. **加入 NavMesh 或邊界**：引入 `bevy_pathmesh` (或其他適合 Bevy 0.18 的簡易導航網格 crate)。在 Server / Shared 定義一塊「可行走區域」多邊形（例如避開場景中央的某個矩形障礙物）。
4. **路徑修正 (Pathfinding / Sliding)**：當 Client 送出 `MoveIntent` 時，Server 計算從當前 `Position` 到 `TargetPosition` 的路線。如果遇到 NavMesh 邊界，則沿著邊界滑動 (Sliding) 或是只走到邊界處，確保玩家無法穿越障礙物。Client 只需負責接收 Server 正確的座標。

## 3. 資料結構定義
例如在 `client/src/systems/render.rs`：
```rust
use bevy::prelude::*;

#[derive(Component)]
pub struct YSortable;

pub fn y_sorting_system(mut query: Query<&mut Transform, With<YSortable>>) {
    for mut transform in query.iter_mut() {
        // 設定 Z 軸以達成 Y 座標越小 (越下方) 則 Z 越大 (越上層) 的 2.5D 遮擋效果
        transform.translation.z = -transform.translation.y * 0.0001;
    }
}
```

## 4. 邊界提醒（Technical Artist 絕對不能做的事）
- 🚫 **不要使用任何 Tilemap crate**。我們明確捨棄了格狀地圖，使用純座標、連續空間與 NavMesh。
- 🚫 **不要載入真正的外部高解析圖**。目前還是「白模 MVP」階段，請繼續使用 `SpriteBundle` 搭配 `custom_size` 與純色。
- 🚫 **不要將碰撞邏輯唯獨寫在 Client**。這是一款 Authoritative Server 的遊戲，真正的阻擋判斷與路徑截斷必定要在 Server 端執行！

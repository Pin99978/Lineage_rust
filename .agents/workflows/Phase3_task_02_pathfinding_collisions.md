---
description: 進階 AI 尋路與地形阻擋 (A* Pathfinding & Map Collisions)
---

# 任務藍圖：Phase 3 - Task 02 - A* Pathfinding & Map Collisions

**指派對象**：Backend Engineer Agent / Game Designer Agent  
**目標**：讓遊戲世界具備物理阻擋感。伺服器必須實作網格碰撞系統 (Collision Grid) 或導航網格 (NavMesh)，並搭配 A* 演算法讓玩家與怪物能在不穿牆的前提下，聰明地繞過障礙物尋找路徑。

## 1. 涉及檔案清單
- `Cargo.toml` (Workspace / server)（修改：引入 `pathfinding` crate）。
- `server/src/map_data.rs` (修改：定義一張簡單的二維陣列或網格結構，標記哪裡是牆壁 `1`，哪裡是空地 `0`）。
- `shared/src/components/movement.rs` (新建/修改：把單純的 `TargetPosition` 升級為包含多個節點的路徑隊列（Path Queue），如 `Vec<Position>`）。
- `server/src/systems/movement.rs`（修改：實作 A* 路徑尋路。當收到 `MoveIntent` 時，先計算出從起點到終點的可行路徑節點。如果終點不可達，則走到最近的路徑點）。
- `server/src/systems/ai.rs`（修改：怪物的 `Chasing_system` 同樣改為利用 A* 計算下一個節點，而不是無視牆壁直衝玩家）。

## 2. Step-by-Step 實作步驟
1. **(Backend) 定義伺服器端碰撞網格 (`CollisionGrid`)**：
   - 建立一個 2D Array (例如 `100x100` 或 `200x200` 每格代表 1.0 的世界座標單位)。
   - 在 `map_data.rs` 裡隨便畫幾條「牆壁 (Wall)」阻擋玩家。例如：將 Y 座標在 `[5, 15]`，X 座標在 `[10, 15]` 的區域都標示為不可行走 (`blocked = true`)。
2. **(Backend) A* 尋路演算法整合 (`pathfinding` crate)**：
   - 使用 `pathfinding::directed::astar::astar` 實作函式 `find_path(start, end, grid)`。
   - 考量 8 方向移動，斜向走的 cost 是直線的 1.4 倍 (大約 `sqrt(2)`)。
   - 確保不會走到 `blocked == true` 的網格上。
3. **(Backend) 重新改寫玩家移動邏輯 (`Movement System`)**：
   - 當收到 `MoveIntent`，伺服器不再直接把 `TargetPosition` 設為玩家點選的終點。
   - 呼叫 `find_path` 取得一系列的 `Vec<Position>` (Waypoints)。
   - `movement_system` 每一幀更新時，慢慢從 `Waypoints` 隊列中將玩家朝著下一個最近的節點移動。到達之後彈出下一個節點。
4. **(Backend & Game Designer) AI 怪物尋路升級**：
   - 當怪物進入 `Chasing` 狀態，也套用這套 `find_path(monster_pos, player_pos)`。
   - 若計算路徑太長或不可達 (玩家躲在完全封閉的牆壁後)，怪物應該放棄追擊退回 `Idle` 狀態。

## 3. 資料結構定義
例如在客端 `shared/src/components/movement.rs` 或 `server/src/map_data.rs` 中：
```rust
use bevy::prelude::*;
use std::collections::VecDeque;

#[derive(Resource, Debug, Clone)]
pub struct CollisionGrid {
    pub width: usize,
    pub height: usize,
    pub cell_size: f32,
    pub obstacles: Vec<bool>, // true 代表被阻擋
}

impl CollisionGrid {
    pub fn is_blocked(&self, x: f32, y: f32) -> bool {
        // ... 將世界座標轉為 Grid 座標，回傳是否為牆壁 ...
        false
    }
}

#[derive(Component, Default, Debug, Clone)]
pub struct PathQueue {
    pub waypoints: VecDeque<Position>, // 儲存 A* 算出來的路徑點
}
```

## 4. 邊界提醒（Agent 絕對不能做的事）
- 🚫 **嚴禁在 Client 端計算 A* 尋路**。真正的地形資料掌管在 Server 手裡，Client 只管發送最終想抵達的 `MoveIntent` 的座標，讓 Server 回傳正確走出來的路。
- 🚫 **不要為每一個怪物每 Frame 都重新計算 A* 演算法**。非常耗效能。怪物可以在進入 `Chasing` 狀態的瞬間計算一次路徑，如果在途中玩家位移沒有超過太多，可以沿用舊路徑；或者設定一個 `Timer` 每 1.0 秒才重新尋路一次。
- 🚫 **先不用實作複雜的多邊形 NavMesh**。對於 ARPG MVP 來說，簡單的 2D 陣列碰撞網格（2D Collision Grid Array）已經足以滿足天堂風格的移動需求了，避免過早優化。

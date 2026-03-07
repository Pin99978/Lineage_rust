---
description: 基礎怪物 AI：索敵、追擊與自動反擊 MVP
---

# 任務藍圖：Task 05 - Basic Enemy AI & Aggro MVP

**指派對象**：Game Designer Agent / Backend Engineer Agent  
**目標**：在 Bevy 0.18 的 Server 端增加「具備主動思考」的敵人實體。這些敵人不再只是站樁，而會偵測玩家距離（Aggro / 索敵範圍），主動追蹤，並在進入攻擊距離時對玩家發動攻擊（扣除玩家 Health）。

## 1. 涉及檔案清單
- `shared/src/components/ai.rs`（新建：定義 AI 狀態，如 `AggroRange`, `ChasingTarget` 等）。
- `server/src/systems/ai.rs`（新建：處理怪物的思考邏輯，包含巡邏、索敵、追擊、攻擊）。
- `server/src/main.rs`（修改：將之前的「站樁假人」換成幾隻具有 `AiBehavior` 及 `MoveSpeed` 的哥布林/史萊姆白模）。
- `client/src/systems/combat_render.rs` 或 UI（暫不動/稍微修改：確認玩家扣血時也能看到傷害提示或血條變化）。

## 2. Step-by-Step 實作步驟
1. **(Game Designer) AI 狀態與組件定義**：在 `shared` 下建立簡單的狀態機 `AiState (Idle, Chasing, Attacking)`，以及定義仇恨範圍 `AggroRange` 組件。
2. **(Backend) 視野與索敵系統**：在 Server 端實作 `ai_aggro_system`，每隔一定頻率（或每幀）檢查 `Idle` 狀態的敵人周圍半徑 (`AggroRange`) 內是否有帶有 `Player` 標籤的實體。若有，切換為 `Chasing` 狀態並將 `TargetEntity` 設為該玩家。
3. **(Backend) 追擊與尋路機制**：實作 `ai_chase_system`，處於 `Chasing` 狀態的敵人會持續更新其 `TargetPosition` 為目標玩家的位置。由於 Task 03 已完成 NavMesh 移動，這裡只需讓它像玩家一樣更新 `TargetPosition` 即可自動避開障礙朝玩家移動。
4. **(Backend) 發動反擊**：當怪物與玩家的距離小於自身的 `attack_range`，狀態轉為 `Attacking`，發動跟玩家上一階段相同的 `combat_system` 邏輯，並向全服廣播 `DamageEvent`，讓 Client 也能看到玩家被扣血。
5. **(Backend) 玩家血量判斷**：稍微擴充 `combat_system`，若玩家受到攻擊且 `Health` 歸零，先簡單印出 Log 或讓玩家角色「暫時切換成灰色 / 死亡狀態」（真正的重生機制將在下一個 Phase 處理）。

## 3. 資料結構定義
例如在 `shared/src/components/ai.rs`：
```rust
use bevy::prelude::*;

#[derive(Component, Debug, Clone, Reflect)]
pub struct AggroRange(pub f32);

#[derive(Component, Default, Debug, Clone, Reflect)]
pub enum AiState {
    #[default]
    Idle,
    Chasing(Entity),
    Attacking(Entity),
}
```

## 4. 邊界提醒（Agent 絕對不能做的事）
- 🚫 **嚴禁實作複雜的 A* 多節點複雜巡邏 / 逃跑邏輯**。怪物只需要原地發呆 `Idle`，看到人就直直衝過去 `Chasing` 及原地揍人 `Attacking` 即可。MVP 越簡單越好。
- 🚫 **不要把 AI 邏輯放到 Client 算**。敵人索敵、扣血、移動都必須是 Server-Side Authoritative。
- 🚫 **暫時不需要實作掉寶（Looting）**。掉寶與背包系統是下一個大 Feature 的範圍。

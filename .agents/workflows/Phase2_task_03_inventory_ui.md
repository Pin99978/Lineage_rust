---
description: 進階背包與實體角色面板 UI (Advanced Inventory GUI & Interactive UI Windows)
---

# 任務藍圖：Phase 2 - Task 03 - Advanced Inventory GUI & Interactive Windows

**指派對象**：Technical Artist Agent / Backend Engineer Agent
**目標**：將 Phase 1 中純粹只印在日誌裡的裝備資訊 (`equipment changed: weapon=None armor=Some(LeatherArmor)`) 升級為真正的視覺化 RPG 介面。實作可開關的「背包 (Inventory, 預設鍵 'I')」與「角色裝備面板 (Character Paper-doll, 預設鍵 'C')」，並支援滑鼠點擊物品來「裝備/卸下」。

## 1. 涉及檔案清單
- `client/src/systems/ui/inventory.rs`（新建：處理背包 UI 畫面的生成、開啟關閉邏輯、物品 Slot 繪製）。
- `client/src/systems/ui/paperdoll.rs`（新建：處理角色面板 UI，顯示當前裝備與角色屬性）。
- `client/src/systems/input.rs`（修改：新增攔截熱鍵 'I' 與 'C'，切換 UI 的顯示與隱藏）。
- `client/src/systems/network.rs`（修改：當接收到 `InventoryUpdate` 或是 `EquipmentUpdate` 時，不只要 `info!()`，還要更新給 UI 系統知道）。
- `shared/src/protocol.rs`（確認/修改：確保 Client 發送的 `EquipIntent` / `UnequipIntent` 運作正常，並能由 UI 的 Click 事件觸發）。

## 2. Step-by-Step 實作步驟
1. **(Technical Artist) 定義 UI 資源與佈局 (Layout)**：
   - 使用 Bevy UI Node 系統，畫出一個固定尺寸、半透明背景的「背包面板」。
   - 面板內部使用 `Grid` 或 `Flex` 排列多個方格 (Slots)，代表背包裡面的物品。
   - 畫出一個「角色裝備面板」，裡面有特定位置的格子對應頭盔、盔甲、武器等 (`EquipmentSlot`)。
2. **(Technical Artist) 熱鍵切換與 Focus 攔截**：
   - 按下 `I` 鍵可以 Toggle (開/關) 背包介面；按下 `C` 鍵可以 Toggle 角色介面。
   - 如果 UI 在最上層開啟，確保滑鼠點擊不會意外點到後面的怪物而導致人物移動或攻擊 (Pointer Block)。
3. **(Technical Artist) 資料綁定 (Data Binding)**：
   - 建立一個 Client 端專屬的 Component 或是 Resource (`ClientInventoryState`, `ClientEquipmentState`) 來快取 Server 傳來的背包狀態。
   - UI 系統在每個 Frame 或在狀態變更事件觸發時，重新渲染格子內的文字 (例如 `LeatherArmor x1`) 或是圖示。
4. **(TA & Backend) 互動發送 (`EquipIntent`)**：
   - 替背包裡的每個「可裝備物品格子」綁定 `On<Pointer<Click>>` 的 Observer (Bevy 0.16+ 機制)。
   - 當玩家點擊背包裡的 `LeatherArmor`：產生並派送一個 `EquipIntent` Network Packet 給 Server。
   - 當玩家點擊角色面板上的已裝備 `LeatherArmor`：產生並派送一個 `UnequipIntent` 給 Server。
   - **注意：UI 直接反應操作是很危險的。請發送封包後，等待 Server 回傳 `EquipmentUpdate` 成功後，再更新 UI 畫面，這才是真正 Authoritative Server 的作法。**

## 3. 資料結構參考 (前端狀態快取)
在 `client/src/systems/ui/inventory.rs` 可能會有類似這樣的本地資料快取：
```rust
use bevy::prelude::*;
use shared::components::item::{ItemType, EquipmentSlot};

#[derive(Resource, Default, Debug, Clone)]
pub struct LocalInventoryState {
    pub gold: u32,
    pub items: Vec<InventorySlot>,
}

#[derive(Debug, Clone)]
pub struct InventorySlot {
    pub item_type: ItemType,
    pub quantity: u32,
}

#[derive(Resource, Default, Debug, Clone)]
pub struct LocalEquipmentState {
    pub weapon: Option<ItemType>,
    pub armor: Option<ItemType>,
}
```

## 4. 邊界提醒（Agent 絕對不能做的事）
- 🚫 **嚴禁 Client 先斬後奏 (Client Prediction on Equipment)**。玩家點選「穿備」後，在畫面上的 UI 絕對不能馬上畫上去。必須是發送封包 -> 等待 Server 驗證 -> 收到 Server 說「你穿上了」 (`EquipmentUpdate`) -> 才能畫。這能杜絕 99% 的裝備複製 Bug 與作弊行為。
- 🚫 **不要強求圖片 Icon (`asset_server`) 如果還沒有**。這個 MVP 重點在「版型 UI Layout」與「滑鼠點擊事件的綁定」。格子的內容可以直接用 Text (`字串`) 代表物品名稱，例如寫著「皮甲」兩個字的方塊，不需要等美術畫好完美的刀劍 Icon 才能開工。
- 🚫 **避開過度複雜的拖曳 (Drag and Drop) MVP**。雖然我們最終希望有完整的拖曳，但在此 MVP 中，只要做到「點擊 (Click)」背包物品就自動 `Equip`，點擊裝備欄物品就自動 `Unequip` 進入背包，這樣就足以驗證雙向通訊了。

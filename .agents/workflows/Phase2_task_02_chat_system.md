---
description: 社交聊天頻道系統 MVP (Social Chat System & UI)
---

# 任務藍圖：Phase 2 - Task 02 - Social Chat System & UI

**指派對象**：Backend Engineer Agent / Technical Artist Agent  
**目標**：實作 MMORPG 不可或缺的靈魂功能：「聊天系統」。包括前端的文字輸入框、聊天記錄區域，以及後端的距離廣播（分區喊話）。這將為後續的組隊與交易系統建立最重要的溝通橋樑。

## 1. 涉及檔案清單
- `shared/src/protocol.rs`（修改：新增 `ChatIntent` 及 `ChatEvent`，並定義頻道類型 Enum `ChatChannel`）。
- `server/src/systems/chat.rs`（新建：處理來自客戶端的聊天訊息，並根據頻道類型廣播給正確範圍內的玩家）。
- `client/src/systems/ui/chat.rs`（新建：使用 Bevy UI 建立左下角可捲動的對話框，以及攔截鍵盤輸入發送訊息）。
- `client/src/systems/input.rs`（修改：實作攔截模式。當聊天框在輸入時，阻止移動、施法等熱鍵觸發）。

## 2. Step-by-Step 實作步驟
1. **(Backend & TA) 通訊協議**：在 `shared` 定義 `ChatChannel` (包含 `Say` 一般說話, `Shout` 大喊, `Whisper` 密語)。新增 `ChatIntent(channel, target_name_opt, text)` 供 Client 送出，以及 `ChatEvent(sender_name, channel, text)` 供 Server 回傳。
2. **(Technical Artist) 聊天窗 UI**：使用 Bevy 的 NodeBundle 與 TextBundle，在畫面左下角做出一個黑色半透明的聊天視窗。分為「顯示區 (History)」與「輸入區 (Input field)」。
3. **(Technical Artist) 輸入攔截與焦點**：當玩家按下 `Enter` 鍵，啟動輸入框焦點 (Focus)。此時任何字母按鍵都只會加入輸入框文字，不會觸發走路或放技能。再次按下 `Enter` 則送出 `ChatIntent` 並清除焦點。
4. **(Backend) 廣播邏輯**：
   - 如果收到 `Say`：只發送 `ChatEvent` 給距離發送者座標 30.0 半徑內的所有玩家。
   - 如果收到 `Shout`：加上前綴（例如 `[大喊] 玩家: Hello`）並發送給全伺服器的所有玩家。
   - 如果收到 `Whisper`：查詢目標名稱是否在線，若在線則單獨傳送給該名玩家與發送者本身。

## 3. 資料結構定義
例如在 `shared/src/protocol.rs` 可能會新增：
```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ChatChannel {
    Say,
    Shout,
    Whisper,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatIntent {
    pub channel: ChatChannel,
    pub target: Option<String>, // 密語才需要帶名字
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatEvent {
    pub sender: String,
    pub channel: ChatChannel,
    pub message: String,
}
```

## 4. 邊界提醒（Agent 絕對不能做的事）
- 🚫 **嚴禁濫用 `unwrap()` 在輸入解析上**。如果玩家打了一堆亂碼或者試圖密語一個不在線上的玩家，伺服器只需靜默丟棄或者回傳「該玩家不在線上」的系統訊息，絕對不可以 Panic。
- 🚫 **不要嘗試實作完美的 IME 中文輸入法 MVP**。Rust/Bevy 處理 OS 級別的中文輸入法 (IME) 非常複雜。這個 MVP 只要保證**基礎的英文/數字輸入**與發送能動就好，不要陷入 `winit` IME 事件的泥沼中！如果能打出中文字是加分，不能的話先以純 ASCII 測試為主。
- 🚫 **注意 ECS 系統執行順序**。確保聊天焦點的檢查，優先於移動系統。不然玩家一邊打字聊天會一邊亂跑。

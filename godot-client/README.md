# Godot Client Scaffold (Phase 7 Migration)

This is the initial migration scaffold for replacing the temporary Bevy client.

## Current status

Implemented:
- Godot 4 project skeleton
- UDP connection to server (`127.0.0.1:5000`)
- JSON protocol encode/decode
- Login request send (`LoginRequest`)
- Core intent send path:
  - Move / Attack / Loot / Interact
  - Cast spell / Use item / Equip / Unequip
  - Chat and guild commands
- Server message handling for core gameplay/UI:
  - Login, entity/map, inventory/equipment, mana/exp/level, spell learned
  - chat/system notice, guild updates/invite/errors, quest/dialog/status
- In-game UI (Godot):
  - HUD
  - chat input/history
  - inventory panel
  - paperdoll panel
  - guild panel
  - dialog panel
- World marker rendering + click interaction routing

Not implemented yet:
- Sprite assets/animation
- Map backgrounds/tilemap
- Full NPC dialogue choice input loop
- Visual parity polish vs Bevy client

## How to run

1. Start server:

```bash
cargo run -p server
```

2. Open `godot-client/project.godot` in Godot 4.
3. Run scene `res://scenes/Main.tscn`.
4. Enter username/class and click Login.

## Folder layout

- `project.godot`: Godot project config
- `scenes/Main.tscn`: main scene
- `scripts/main.gd`: app bootstrap + world sync
- `scripts/net/protocol.gd`: JSON protocol helpers
- `scripts/net/client_network.gd`: UDP client transport
- `scripts/world/entity_registry.gd`: temporary in-memory entity state store
- `scripts/ui/login_ui.gd`: basic login panel

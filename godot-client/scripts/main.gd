extends Node2D

const ITEM_ORDER := [
    "BronzeSword",
    "LeatherArmor",
    "HealthPotion",
    "ScrollLightning",
    "ScrollPoisonArrow",
    "ScrollBless",
    "Gold",
]
const SPELL_ORDER := ["Fireball", "Heal", "Lightning", "PoisonArrow", "Bless"]
const MAX_CHAT_HISTORY := 12
const MAX_CHAT_INPUT := 160
const INTERACT_RADIUS := 24.0

@onready var _login_ui: LoginUI = $CanvasLayer/LoginUI
@onready var _world_layer: Node2D = $WorldLayer
@onready var _canvas_layer: CanvasLayer = $CanvasLayer

var _network: ClientNetwork
var _entity_registry := EntityRegistry.new()
var _local_player_id: int = -1
var _local_map_id: String = "town"
var _entity_nodes: Dictionary = {}

var _hud_state := {
    "class": "Knight",
    "guild_name": null,
    "guild_role": null,
    "guild_members": [],
    "known_spells": [],
    "mana_current": 60,
    "mana_max": 60,
    "level": 1,
    "exp_current": 0,
    "exp_next": 100,
    "str_stat": 15,
    "dex": 15,
    "int_stat": 15,
    "con": 15,
    "equipment": {"weapon": null, "armor": null},
    "status_effects": [],
    "quest_entries": {},
}
var _inventory_state: Dictionary = {}
var _chat_history: Array[String] = []
var _chat_focused := false
var _chat_input := ""
var _dialog_state := {
    "visible": false,
    "text": "",
    "choices": [],
    "npc_id": null,
}
var _windows_state := {
    "inventory_open": false,
    "paperdoll_open": false,
    "guild_open": false,
}

var _hud_label: Label
var _chat_history_label: RichTextLabel
var _chat_input_label: Label
var _inventory_panel: PanelContainer
var _inventory_list: VBoxContainer
var _inventory_buttons: Dictionary = {}
var _paperdoll_panel: PanelContainer
var _paperdoll_label: Label
var _guild_panel: PanelContainer
var _guild_label: Label
var _dialog_panel: PanelContainer
var _dialog_label: Label
var _dialog_choice_box: VBoxContainer
var _ui_dirty := true

func _ready() -> void:
    _world_layer.position = get_viewport_rect().size * 0.5

    _network = ClientNetwork.new()
    add_child(_network)
    _network.server_message_received.connect(_on_server_message_received)
    _network.network_error.connect(_on_network_error)

    if _network.connect_to_server():
        _login_ui.set_status("Connected to 127.0.0.1:5000")
    else:
        _login_ui.set_status("Connection failed")

    _login_ui.login_submitted.connect(_on_login_submitted)
    _build_ingame_ui()
    _mark_ui_dirty()

func _process(_delta: float) -> void:
    _network.poll_messages()
    if _ui_dirty:
        _refresh_ui()

func _unhandled_input(event: InputEvent) -> void:
    if event is InputEventKey and event.pressed and not event.echo:
        var key_event := event as InputEventKey
        _handle_key_input(key_event)
        return

    if event is InputEventMouseButton:
        var mouse_event := event as InputEventMouseButton
        if mouse_event.button_index == MOUSE_BUTTON_LEFT and mouse_event.pressed:
            _handle_world_click(mouse_event.position)

func _build_ingame_ui() -> void:
    _hud_label = Label.new()
    _hud_label.position = Vector2(16, 16)
    _canvas_layer.add_child(_hud_label)

    var chat_panel := PanelContainer.new()
    chat_panel.position = Vector2(16, 460)
    chat_panel.custom_minimum_size = Vector2(460, 240)
    _canvas_layer.add_child(chat_panel)

    var chat_vbox := VBoxContainer.new()
    chat_panel.add_child(chat_vbox)

    _chat_history_label = RichTextLabel.new()
    _chat_history_label.fit_content = true
    _chat_history_label.bbcode_enabled = false
    _chat_history_label.custom_minimum_size = Vector2(440, 190)
    _chat_history_label.scroll_active = false
    chat_vbox.add_child(_chat_history_label)

    _chat_input_label = Label.new()
    chat_vbox.add_child(_chat_input_label)

    _inventory_panel = PanelContainer.new()
    _inventory_panel.position = Vector2(980, 24)
    _inventory_panel.custom_minimum_size = Vector2(280, 340)
    _canvas_layer.add_child(_inventory_panel)

    var inventory_vbox := VBoxContainer.new()
    _inventory_panel.add_child(inventory_vbox)

    var inventory_title := Label.new()
    inventory_title.text = "Inventory [I]"
    inventory_vbox.add_child(inventory_title)

    _inventory_list = VBoxContainer.new()
    inventory_vbox.add_child(_inventory_list)

    for item_name in ITEM_ORDER:
        var btn := Button.new()
        btn.text = item_name
        btn.pressed.connect(_on_inventory_item_pressed.bind(item_name))
        _inventory_list.add_child(btn)
        _inventory_buttons[item_name] = btn

    _paperdoll_panel = PanelContainer.new()
    _paperdoll_panel.position = Vector2(980, 380)
    _paperdoll_panel.custom_minimum_size = Vector2(280, 220)
    _canvas_layer.add_child(_paperdoll_panel)

    _paperdoll_label = Label.new()
    _paperdoll_panel.add_child(_paperdoll_label)

    _guild_panel = PanelContainer.new()
    _guild_panel.position = Vector2(680, 380)
    _guild_panel.custom_minimum_size = Vector2(280, 220)
    _canvas_layer.add_child(_guild_panel)

    _guild_label = Label.new()
    _guild_panel.add_child(_guild_label)

    _dialog_panel = PanelContainer.new()
    _dialog_panel.position = Vector2(420, 24)
    _dialog_panel.custom_minimum_size = Vector2(420, 140)
    _canvas_layer.add_child(_dialog_panel)

    _dialog_label = Label.new()
    _dialog_panel.add_child(_dialog_label)

    _dialog_choice_box = VBoxContainer.new()
    _dialog_panel.add_child(_dialog_choice_box)

func _refresh_ui() -> void:
    if not _ui_dirty:
        return
    _ui_dirty = false

    _inventory_panel.visible = _windows_state["inventory_open"]
    _paperdoll_panel.visible = _windows_state["paperdoll_open"]
    _guild_panel.visible = _windows_state["guild_open"]
    _dialog_panel.visible = _dialog_state["visible"]

    _hud_label.text = "[%s] LV %d  MP %d/%d  EXP %d/%d\nSTR %d DEX %d INT %d CON %d" % [
        _hud_state["class"],
        int(_hud_state["level"]),
        int(_hud_state["mana_current"]),
        int(_hud_state["mana_max"]),
        int(_hud_state["exp_current"]),
        int(_hud_state["exp_next"]),
        int(_hud_state["str_stat"]),
        int(_hud_state["dex"]),
        int(_hud_state["int_stat"]),
        int(_hud_state["con"]),
    ]

    var chat_text := ""
    for line in _chat_history:
        chat_text += line + "\n"
    _chat_history_label.text = chat_text.strip_edges()
    _chat_input_label.text = "> %s" % _chat_input if _chat_focused else "> (Enter to chat)"

    for item_name in ITEM_ORDER:
        var amount := int(_inventory_state.get(item_name, 0))
        var hint := "Not Usable"
        if item_name == "BronzeSword" or item_name == "LeatherArmor":
            hint = "Click to Equip"
        elif item_name == "HealthPotion" or item_name.begins_with("Scroll"):
            hint = "Click to Use"
        var btn: Button = _inventory_buttons[item_name]
        btn.text = "%s x%d [%s]" % [item_name, amount, hint]

    var known_spells := _hud_state["known_spells"] as Array
    var spells_text := ""
    for spell_name in SPELL_ORDER:
        if known_spells.has(spell_name):
            spells_text += "- %s\n" % spell_name

    var equip := _hud_state["equipment"] as Dictionary
    _paperdoll_label.text = "Character [C]\nWeapon: %s\nArmor: %s\n\nKnown Spells:\n%s" % [
        str(equip.get("weapon", null)),
        str(equip.get("armor", null)),
        spells_text if not spells_text.is_empty() else "-",
    ]

    var guild_name = _hud_state["guild_name"]
    var guild_role = _hud_state["guild_role"]
    var members := _hud_state["guild_members"] as Array
    var guild_member_text := ""
    for member in members:
        guild_member_text += "- %s\n" % str(member)
    _guild_label.text = "Guild [G]\nName: %s\nRole: %s\n\nMembers:\n%s" % [
        str(guild_name),
        str(guild_role),
        guild_member_text if not guild_member_text.is_empty() else "-",
    ]

    _dialog_label.text = str(_dialog_state["text"])

func _handle_key_input(event: InputEventKey) -> void:
    if _login_ui.visible:
        return

    if _chat_focused:
        _handle_chat_typing(event)
        return

    if _handle_dialog_choice_hotkey(event.keycode):
        return

    if event.keycode == KEY_ENTER:
        _chat_focused = true
        _mark_ui_dirty()
        return

    if event.keycode == KEY_I:
        _windows_state["inventory_open"] = not _windows_state["inventory_open"]
        _mark_ui_dirty()
        return
    if event.keycode == KEY_C:
        _windows_state["paperdoll_open"] = not _windows_state["paperdoll_open"]
        _mark_ui_dirty()
        return
    if event.keycode == KEY_G:
        _windows_state["guild_open"] = not _windows_state["guild_open"]
        _mark_ui_dirty()
        return

    if _blocks_world_input():
        return

    match event.keycode:
        KEY_1:
            _send(LineageProtocol.use_item_intent("HealthPotion"))
        KEY_2:
            var enemy_target = _nearest_enemy_id()
            if enemy_target >= 0:
                _send(LineageProtocol.cast_spell_intent("Fireball", enemy_target))
        KEY_3:
            _send(LineageProtocol.cast_spell_intent("Heal", null))
        KEY_Q:
            var lightning_target = _nearest_enemy_id()
            if lightning_target >= 0:
                _send(LineageProtocol.cast_spell_intent("Lightning", lightning_target))
        KEY_E:
            var poison_target = _nearest_enemy_id()
            if poison_target >= 0:
                _send(LineageProtocol.cast_spell_intent("PoisonArrow", poison_target))
        KEY_R:
            _send(LineageProtocol.cast_spell_intent("Bless", null))
        KEY_T:
            _send(LineageProtocol.equip_intent("BronzeSword"))
        KEY_Y:
            _send(LineageProtocol.equip_intent("LeatherArmor"))
        KEY_U:
            _send(LineageProtocol.unequip_intent("Weapon"))
        KEY_Z:
            _send(LineageProtocol.unequip_intent("Armor"))

func _handle_chat_typing(event: InputEventKey) -> void:
    if event.keycode == KEY_ENTER:
        var raw := _chat_input.strip_edges()
        _chat_input = ""
        _chat_focused = false
        _mark_ui_dirty()
        if raw.is_empty():
            return
        _send_chat_or_command(raw)
        return

    if event.keycode == KEY_ESCAPE:
        _chat_input = ""
        _chat_focused = false
        _mark_ui_dirty()
        return

    if event.keycode == KEY_BACKSPACE:
        if _chat_input.length() > 0:
            _chat_input = _chat_input.substr(0, _chat_input.length() - 1)
            _mark_ui_dirty()
        return

    var unicode_code := event.unicode
    if unicode_code <= 0:
        return
    var ch := char(unicode_code)
    if ch == "\n" or ch == "\r" or ch == "\t":
        return
    if _chat_input.length() >= MAX_CHAT_INPUT:
        return
    _chat_input += ch
    _mark_ui_dirty()

func _handle_world_click(screen_position: Vector2) -> void:
    if _login_ui.visible:
        return
    if _chat_focused or _blocks_world_input():
        return

    var world_position := screen_position - _world_layer.position
    var clicked_id := _entity_id_near_position(world_position, INTERACT_RADIUS)
    if clicked_id >= 0:
        var state: Dictionary = _entity_registry.entities.get(clicked_id, {})
        var kind := str(state.get("kind", ""))
        if kind == "Enemy":
            _send(LineageProtocol.attack_intent(clicked_id))
            return
        if kind == "LootGold" or kind == "LootHealthPotion":
            _send(LineageProtocol.loot_intent(clicked_id))
            return
        if kind == "NpcMerchant":
            _send(LineageProtocol.interact_npc_intent(clicked_id, null))
            return

    _send(LineageProtocol.move_intent(world_position.x, world_position.y))

func _on_inventory_item_pressed(item_name: String) -> void:
    if item_name == "BronzeSword" or item_name == "LeatherArmor":
        _send(LineageProtocol.equip_intent(item_name))
    elif item_name == "HealthPotion" or item_name.begins_with("Scroll"):
        _send(LineageProtocol.use_item_intent(item_name))

func _blocks_world_input() -> bool:
    return _windows_state["inventory_open"] or _windows_state["paperdoll_open"] or _windows_state["guild_open"]

func _nearest_enemy_id() -> int:
    if _local_player_id < 0:
        return -1
    var me: Dictionary = _entity_registry.entities.get(_local_player_id, {})
    if me.is_empty():
        return -1
    var me_pos := Vector2(float(me.get("x", 0.0)), float(me.get("y", 0.0)))

    var best_id := -1
    var best_dist := INF
    for entity_id in _entity_registry.entities.keys():
        var state: Dictionary = _entity_registry.entities[entity_id]
        if str(state.get("kind", "")) != "Enemy":
            continue
        if str(state.get("map_id", "")) != _local_map_id:
            continue
        if int(state.get("health_current", 0)) <= 0:
            continue
        var pos := Vector2(float(state.get("x", 0.0)), float(state.get("y", 0.0)))
        var dist := me_pos.distance_to(pos)
        if dist < best_dist:
            best_dist = dist
            best_id = int(entity_id)
    return best_id

func _entity_id_near_position(world_position: Vector2, radius: float) -> int:
    var best_id := -1
    var best_dist := radius
    for entity_id in _entity_registry.entities.keys():
        var state: Dictionary = _entity_registry.entities[entity_id]
        if str(state.get("map_id", "")) != _local_map_id:
            continue
        var pos := Vector2(float(state.get("x", 0.0)), float(state.get("y", 0.0)))
        var dist := pos.distance_to(world_position)
        if dist <= best_dist:
            best_dist = dist
            best_id = int(entity_id)
    return best_id

func _on_login_submitted(username: String, class_name: String) -> void:
    _send(LineageProtocol.login_request(username, class_name))
    _login_ui.set_status("Login sent...")

func _send(message: Dictionary) -> void:
    _network.send_message(message)

func _on_network_error(message: String) -> void:
    _push_system_line("Network error: %s" % message)
    _login_ui.set_status("Network error")

func _on_server_message_received(message: Variant) -> void:
    if typeof(message) != TYPE_DICTIONARY:
        return
    var data: Dictionary = message

    if data.has("LoginResponse"):
        var response: Dictionary = data["LoginResponse"]
        if bool(response.get("success", false)):
            _login_ui.set_status("Login success")
            _login_ui.visible = false
            _mark_ui_dirty()
        else:
            _login_ui.set_status("Login failed: %s" % str(response.get("message", "")))
        return

    if data.has("AssignedPlayer"):
        var assigned: Dictionary = data["AssignedPlayer"]
        _local_player_id = int(assigned.get("player_id", -1))
        _inventory_state.clear()
        _hud_state["known_spells"] = []
        _hud_state["guild_members"] = []
        _hud_state["guild_name"] = null
        _hud_state["guild_role"] = null
        _entity_registry.clear()
        _clear_entity_nodes()
        _mark_ui_dirty()
        return

    if data.has("EntityState"):
        var state: Dictionary = data["EntityState"]
        var entity_id := int(state.get("entity_id", -1))
        if _local_player_id != entity_id and str(state.get("map_id", "")) != _local_map_id:
            return
        if _local_player_id == entity_id:
            _local_map_id = str(state.get("map_id", _local_map_id))
            _hud_state["class"] = str(state.get("class", _hud_state["class"]))
            _hud_state["guild_name"] = state.get("guild_name", null)
        _entity_registry.upsert_entity_state(state)
        _sync_entity_node(state)
        return

    if data.has("MapChangeEvent"):
        var evt: Dictionary = data["MapChangeEvent"]
        _local_map_id = str(evt.get("map_id", _local_map_id))
        _clear_non_local_entities()
        return

    if data.has("ItemDespawnEvent"):
        var evt: Dictionary = data["ItemDespawnEvent"]
        var item_id := int(evt.get("item_id", -1))
        _entity_registry.remove_entity(item_id)
        _remove_entity_node(item_id)
        return

    if data.has("DamageEvent"):
        var evt: Dictionary = data["DamageEvent"]
        _spawn_floating_text_for_entity(
            int(evt.get("target_id", -1)),
            "-%s" % str(evt.get("amount", 0)),
            Color(1.0, 0.8, 0.2)
        )
        return

    if data.has("InventoryUpdate"):
        var evt: Dictionary = data["InventoryUpdate"]
        if int(evt.get("player_id", -1)) == _local_player_id:
            var item_type := str(evt.get("item_type", ""))
            var amount := int(evt.get("amount", 0))
            if amount <= 0:
                _inventory_state.erase(item_type)
            else:
                _inventory_state[item_type] = amount
            _mark_ui_dirty()
        return

    if data.has("ManaUpdate"):
        var evt: Dictionary = data["ManaUpdate"]
        if int(evt.get("player_id", -1)) == _local_player_id:
            _hud_state["mana_current"] = int(evt.get("current", _hud_state["mana_current"]))
            _hud_state["mana_max"] = int(evt.get("max", _hud_state["mana_max"]))
            _mark_ui_dirty()
        return

    if data.has("ExpUpdateEvent"):
        var evt: Dictionary = data["ExpUpdateEvent"]
        if int(evt.get("player_id", -1)) == _local_player_id:
            _hud_state["level"] = int(evt.get("level", _hud_state["level"]))
            _hud_state["exp_current"] = int(evt.get("exp_current", _hud_state["exp_current"]))
            _hud_state["exp_next"] = int(evt.get("exp_next", _hud_state["exp_next"]))
            _hud_state["str_stat"] = int(evt.get("str_stat", _hud_state["str_stat"]))
            _hud_state["dex"] = int(evt.get("dex", _hud_state["dex"]))
            _hud_state["int_stat"] = int(evt.get("int_stat", _hud_state["int_stat"]))
            _hud_state["con"] = int(evt.get("con", _hud_state["con"]))
            _mark_ui_dirty()
        return

    if data.has("LevelUpEvent"):
        var evt: Dictionary = data["LevelUpEvent"]
        if int(evt.get("player_id", -1)) == _local_player_id:
            _push_system_line("LEVEL UP! Level %d" % int(evt.get("new_level", 1)))
        return

    if data.has("HealEvent"):
        var evt: Dictionary = data["HealEvent"]
        _spawn_floating_text_for_entity(
            int(evt.get("target_id", -1)),
            "+%s" % str(evt.get("amount", 0)),
            Color(0.4, 1.0, 0.5)
        )
        return

    if data.has("EquipmentUpdate"):
        var evt: Dictionary = data["EquipmentUpdate"]
        if int(evt.get("player_id", -1)) == _local_player_id:
            _hud_state["equipment"] = evt.get("equipment", {"weapon": null, "armor": null})
            _mark_ui_dirty()
        return

    if data.has("StatusEffectUpdate"):
        var evt: Dictionary = data["StatusEffectUpdate"]
        if int(evt.get("player_id", -1)) == _local_player_id:
            _hud_state["status_effects"] = evt.get("effects", [])
            _mark_ui_dirty()
        return

    if data.has("SpellLearnedEvent"):
        var evt: Dictionary = data["SpellLearnedEvent"]
        if int(evt.get("player_id", -1)) == _local_player_id:
            var spell_name := str(evt.get("spell", ""))
            var known_spells := _hud_state["known_spells"] as Array
            if not known_spells.has(spell_name):
                known_spells.append(spell_name)
            _push_system_line("你學會了 [%s]!" % spell_name)
            _mark_ui_dirty()
        return

    if data.has("QuestUpdateEvent"):
        var evt: Dictionary = data["QuestUpdateEvent"]
        if int(evt.get("player_id", -1)) == _local_player_id:
            var quest_id := str(evt.get("quest_id", ""))
            _hud_state["quest_entries"][quest_id] = evt.get("status", "")
            _mark_ui_dirty()
        return

    if data.has("DialogEvent"):
        var evt: Dictionary = data["DialogEvent"]
        if int(evt.get("player_id", -1)) == _local_player_id:
            _dialog_state["visible"] = true
            _dialog_state["text"] = str(evt.get("text", ""))
            _dialog_state["choices"] = []
            _dialog_state["npc_id"] = null
            _rebuild_dialog_choices()
            _mark_ui_dirty()
        return

    if data.has("DialogueResponse"):
        var evt: Dictionary = data["DialogueResponse"]
        if int(evt.get("player_id", -1)) == _local_player_id:
            _dialog_state["visible"] = true
            _dialog_state["text"] = str(evt.get("text", ""))
            _dialog_state["choices"] = evt.get("choices", [])
            _dialog_state["npc_id"] = evt.get("npc_id", null)
            _rebuild_dialog_choices()
            _mark_ui_dirty()
        return

    if data.has("ChatEvent"):
        var evt: Dictionary = data["ChatEvent"]
        var channel := str(evt.get("channel", "Say"))
        var sender := str(evt.get("sender", ""))
        var text := str(evt.get("message", ""))
        _push_chat_line("[%s] %s: %s" % [channel, sender, text])
        return

    if data.has("SystemNotice"):
        var evt: Dictionary = data["SystemNotice"]
        if int(evt.get("player_id", -1)) == _local_player_id:
            _push_system_line(str(evt.get("text", "")))
        return

    if data.has("GuildUpdateEvent"):
        var evt: Dictionary = data["GuildUpdateEvent"]
        if int(evt.get("player_id", -1)) == _local_player_id:
            _hud_state["guild_name"] = evt.get("guild_name", null)
            _hud_state["guild_role"] = evt.get("role", null)
            _hud_state["guild_members"] = evt.get("member_usernames", [])
            _mark_ui_dirty()
        return

    if data.has("GuildInviteEvent"):
        var evt: Dictionary = data["GuildInviteEvent"]
        if int(evt.get("player_id", -1)) == _local_player_id:
            _push_system_line("Guild invite from %s to join %s. Use /guild accept or /guild deny." % [
                str(evt.get("from_username", "")),
                str(evt.get("guild_name", "")),
            ])
        return

    if data.has("GuildActionError"):
        var evt: Dictionary = data["GuildActionError"]
        if int(evt.get("player_id", -1)) == _local_player_id:
            _push_system_line("Guild error: %s" % str(evt.get("message", "")))
        return

    if data.has("DeathEvent"):
        var evt: Dictionary = data["DeathEvent"]
        if int(evt.get("target_id", -1)) == _local_player_id:
            var lost := evt.get("exp_lost", null)
            if lost != null:
                _push_system_line("你死了！失去了 %s 點經驗值。" % str(lost))
        return

func _sync_entity_node(state: Dictionary) -> void:
    var entity_id := int(state.get("entity_id", -1))
    if entity_id < 0:
        return

    var node := _entity_nodes.get(entity_id, null)
    if node == null:
        var marker := Polygon2D.new()
        marker.polygon = PackedVector2Array([
            Vector2(-12.0, -12.0),
            Vector2(12.0, -12.0),
            Vector2(12.0, 12.0),
            Vector2(-12.0, 12.0),
        ])
        _world_layer.add_child(marker)
        node = marker
        _entity_nodes[entity_id] = node
        _ensure_hp_bar(node)

    node.position = Vector2(float(state.get("x", 0.0)), float(state.get("y", 0.0)))
    var kind := str(state.get("kind", ""))
    if kind == "Player":
        node.color = Color(0.2, 0.4, 1.0)
    elif kind == "Enemy":
        node.color = Color(0.86, 0.2, 0.2)
    elif kind == "NpcMerchant":
        node.color = Color(0.2, 0.85, 0.2)
    elif kind == "Portal":
        node.color = Color(0.78, 0.5, 0.9)
    else:
        node.color = Color(0.9, 0.8, 0.1)

    if entity_id == _local_player_id:
        node.color = Color(0.1, 0.95, 0.95)

    if int(state.get("health_current", 1)) <= 0:
        node.color = Color(0.35, 0.35, 0.35)

    _update_hp_bar(
        node,
        int(state.get("health_current", 0)),
        int(state.get("health_max", 0))
    )

func _clear_entity_nodes() -> void:
    for entity_id in _entity_nodes.keys():
        var node: Node = _entity_nodes[entity_id]
        if is_instance_valid(node):
            node.queue_free()
    _entity_nodes.clear()

func _clear_non_local_entities() -> void:
    var keep := {}
    if _local_player_id >= 0 and _entity_registry.entities.has(_local_player_id):
        keep[_local_player_id] = _entity_registry.entities[_local_player_id]

    for entity_id in _entity_nodes.keys():
        if int(entity_id) == _local_player_id:
            continue
        _remove_entity_node(int(entity_id))
    _entity_registry.entities = keep

func _remove_entity_node(entity_id: int) -> void:
    var node := _entity_nodes.get(entity_id, null)
    if node != null and is_instance_valid(node):
        node.queue_free()
    _entity_nodes.erase(entity_id)

func _push_chat_line(line: String) -> void:
    _chat_history.append(line)
    if _chat_history.size() > MAX_CHAT_HISTORY:
        _chat_history = _chat_history.slice(_chat_history.size() - MAX_CHAT_HISTORY, _chat_history.size())
    _mark_ui_dirty()

func _push_system_line(line: String) -> void:
    _push_chat_line("[System] %s" % line)

func _send_chat_or_command(raw: String) -> void:
    if raw.begins_with("/guild "):
        _handle_guild_command(raw)
        return

    var parsed := _parse_chat_command(raw)
    _send(LineageProtocol.chat_intent(parsed["channel"], parsed["target"], parsed["message"]))

func _parse_chat_command(raw: String) -> Dictionary:
    var trimmed := raw.strip_edges()
    if trimmed.begins_with("/guildchat "):
        return {"channel": "Guild", "target": null, "message": trimmed.substr(11).strip_edges()}
    if trimmed.begins_with("/g "):
        return {"channel": "Guild", "target": null, "message": trimmed.substr(3).strip_edges()}
    if trimmed.begins_with("/shout "):
        return {"channel": "Shout", "target": null, "message": trimmed.substr(7).strip_edges()}
    if trimmed.begins_with("/sh "):
        return {"channel": "Shout", "target": null, "message": trimmed.substr(4).strip_edges()}
    if trimmed.begins_with("/say "):
        return {"channel": "Say", "target": null, "message": trimmed.substr(5).strip_edges()}
    if trimmed.begins_with("/s "):
        return {"channel": "Say", "target": null, "message": trimmed.substr(3).strip_edges()}
    if trimmed.begins_with("/whisper ") or trimmed.begins_with("/w "):
        var rest := trimmed.substr(9) if trimmed.begins_with("/whisper ") else trimmed.substr(3)
        var parts := rest.split(" ", false, 1)
        if parts.size() == 2:
            return {"channel": "Whisper", "target": parts[0], "message": parts[1]}
    return {"channel": "Say", "target": null, "message": trimmed}

func _handle_guild_command(raw: String) -> void:
    var rest := raw.substr(7).strip_edges()
    if rest.is_empty():
        _push_system_line("Usage: /guild create|invite|leave|disband|accept|deny")
        return

    var parts := rest.split(" ", false)
    var action := parts[0]

    match action:
        "create":
            if parts.size() < 2:
                _push_system_line("Usage: /guild create <name>")
                return
            var guild_name := rest.substr(7).strip_edges()
            _send(LineageProtocol.create_guild_intent(guild_name))
        "invite":
            if parts.size() < 2:
                _push_system_line("Usage: /guild invite <player>")
                return
            _send(LineageProtocol.invite_to_guild_intent(parts[1]))
        "leave":
            _send(LineageProtocol.leave_guild_intent())
        "disband":
            _send(LineageProtocol.disband_guild_intent())
        "accept":
            _send(LineageProtocol.respond_guild_invite_intent(true))
        "deny":
            _send(LineageProtocol.respond_guild_invite_intent(false))
        _:
            _push_system_line("Usage: /guild create|invite|leave|disband|accept|deny")

func _mark_ui_dirty() -> void:
    _ui_dirty = true

func _handle_dialog_choice_hotkey(keycode: int) -> bool:
    if not bool(_dialog_state["visible"]):
        return false
    var choices: Array = _dialog_state["choices"] as Array
    if choices.is_empty():
        return false

    if keycode < KEY_1 or keycode > KEY_9:
        return false
    var choice_index := int(keycode - KEY_1)
    if choice_index < 0 or choice_index >= choices.size():
        return false
    _send_dialog_choice(choice_index)
    return true

func _rebuild_dialog_choices() -> void:
    for child in _dialog_choice_box.get_children():
        child.queue_free()

    var choices: Array = _dialog_state["choices"] as Array
    for i in range(choices.size()):
        var btn := Button.new()
        btn.text = "%d. %s" % [i + 1, str(choices[i])]
        btn.pressed.connect(_send_dialog_choice.bind(i))
        _dialog_choice_box.add_child(btn)

func _send_dialog_choice(choice_index: int) -> void:
    var npc_id = _dialog_state["npc_id"]
    if npc_id == null:
        return
    _send(LineageProtocol.interact_npc_intent(int(npc_id), choice_index))

func _ensure_hp_bar(node: Node) -> void:
    if node.has_meta("hp_bg") and node.has_meta("hp_fill"):
        return

    var hp_bg := Polygon2D.new()
    hp_bg.polygon = PackedVector2Array([
        Vector2(-18, -28),
        Vector2(18, -28),
        Vector2(18, -23),
        Vector2(-18, -23),
    ])
    hp_bg.color = Color(0.0, 0.0, 0.0, 0.8)
    hp_bg.visible = false
    node.add_child(hp_bg)

    var hp_fill := Polygon2D.new()
    hp_fill.polygon = PackedVector2Array([
        Vector2(-17, -27),
        Vector2(17, -27),
        Vector2(17, -24),
        Vector2(-17, -24),
    ])
    hp_fill.color = Color(0.88, 0.15, 0.15, 1.0)
    hp_fill.visible = false
    node.add_child(hp_fill)

    node.set_meta("hp_bg", hp_bg)
    node.set_meta("hp_fill", hp_fill)

func _update_hp_bar(node: Node, current_hp: int, max_hp: int) -> void:
    if not node.has_meta("hp_bg") or not node.has_meta("hp_fill"):
        return

    var hp_bg: Polygon2D = node.get_meta("hp_bg")
    var hp_fill: Polygon2D = node.get_meta("hp_fill")

    if max_hp <= 0 or current_hp <= 0:
        hp_bg.visible = false
        hp_fill.visible = false
        return

    hp_bg.visible = true
    hp_fill.visible = true

    var ratio := clamp(float(current_hp) / float(max_hp), 0.0, 1.0)
    var width := max(1.0, 34.0 * ratio)
    hp_fill.polygon = PackedVector2Array([
        Vector2(-17, -27),
        Vector2(-17 + width, -27),
        Vector2(-17 + width, -24),
        Vector2(-17, -24),
    ])

func _spawn_floating_text_for_entity(entity_id: int, text: String, color: Color) -> void:
    var target := _entity_nodes.get(entity_id, null)
    if target == null or not is_instance_valid(target):
        return

    var label := Label.new()
    label.text = text
    label.modulate = color
    label.position = _world_layer.position + target.position + Vector2(-10.0, -34.0)
    _canvas_layer.add_child(label)

    var tween := create_tween()
    tween.tween_property(label, "position", label.position + Vector2(0.0, -28.0), 0.45)
    tween.parallel().tween_property(label, "modulate:a", 0.0, 0.45)
    tween.finished.connect(func() -> void:
        if is_instance_valid(label):
            label.queue_free()
    )

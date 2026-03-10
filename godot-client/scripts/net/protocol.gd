extends RefCounted
class_name LineageProtocol

static func encode_client_message(message: Dictionary) -> PackedByteArray:
    var text := JSON.stringify(message)
    return text.to_utf8_buffer()

static func decode_server_message(payload: PackedByteArray) -> Variant:
    var text := payload.get_string_from_utf8()
    var json := JSON.new()
    var err := json.parse(text)
    if err != OK:
        return null
    return json.data

# serde enum shape from Rust: {"VariantName": {...}}
static func wrap_client_message(variant_name: String, data: Dictionary) -> Dictionary:
    var msg := {}
    msg[variant_name] = data
    return msg

static func login_request(username: String, class_name: String = "Knight") -> Dictionary:
    return wrap_client_message("LoginRequest", {
        "username": username,
        "class": class_name,
    })

static func move_intent(x: float, y: float) -> Dictionary:
    return wrap_client_message("MoveIntent", {
        "target_x": x,
        "target_y": y,
    })

static func attack_intent(target_id: int) -> Dictionary:
    return wrap_client_message("AttackIntent", {
        "target_id": target_id,
    })

static func loot_intent(item_id: int) -> Dictionary:
    return wrap_client_message("LootIntent", {
        "item_id": item_id,
    })

static func cast_spell_intent(spell_name: String, target_id: Variant = null) -> Dictionary:
    return wrap_client_message("CastSpellIntent", {
        "spell": spell_name,
        "target_id": target_id,
    })

static func equip_intent(item_type: String) -> Dictionary:
    return wrap_client_message("EquipIntent", {
        "item_type": item_type,
    })

static func unequip_intent(slot_name: String) -> Dictionary:
    return wrap_client_message("UnequipIntent", {
        "slot": slot_name,
    })

static func use_item_intent(item_type: String) -> Dictionary:
    return wrap_client_message("UseItemIntent", {
        "item_type": item_type,
    })

static func interact_intent(target_id: int) -> Dictionary:
    return wrap_client_message("InteractIntent", {
        "target_id": target_id,
    })

static func interact_npc_intent(target_id: int, choice_index: Variant) -> Dictionary:
    return wrap_client_message("InteractNpcIntent", {
        "target_id": target_id,
        "choice_index": choice_index,
    })

static func chat_intent(channel_name: String, target: Variant, message: String) -> Dictionary:
    return wrap_client_message("ChatIntent", {
        "channel": channel_name,
        "target": target,
        "message": message,
    })

static func create_guild_intent(guild_name: String) -> Dictionary:
    return wrap_client_message("CreateGuildIntent", {"guild_name": guild_name})

static func invite_to_guild_intent(target_username: String) -> Dictionary:
    return wrap_client_message("InviteToGuildIntent", {"target_username": target_username})

static func respond_guild_invite_intent(accepted: bool) -> Dictionary:
    return wrap_client_message("RespondToGuildInvite", {"accepted": accepted})

static func leave_guild_intent() -> Dictionary:
    return wrap_client_message("LeaveGuildIntent", null)

static func disband_guild_intent() -> Dictionary:
    return wrap_client_message("DisbandGuildIntent", null)

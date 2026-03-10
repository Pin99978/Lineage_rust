extends RefCounted
class_name EntityRegistry

# Server entity_id -> simple state dictionary for early migration phase.
var entities: Dictionary = {}

func upsert_entity_state(state: Dictionary) -> void:
    var entity_id = state.get("entity_id", null)
    if entity_id == null:
        return
    entities[entity_id] = state

func remove_entity(entity_id: int) -> void:
    entities.erase(entity_id)

func clear() -> void:
    entities.clear()

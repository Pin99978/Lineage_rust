extends Control
class_name LoginUI

signal login_submitted(username: String, class_name: String)

@onready var _panel := PanelContainer.new()
@onready var _username_edit := LineEdit.new()
@onready var _class_edit := OptionButton.new()
@onready var _status_label := Label.new()

func _ready() -> void:
    _build_ui()

func _build_ui() -> void:
    add_child(_panel)
    _panel.set_anchors_and_offsets_preset(Control.PRESET_CENTER)
    _panel.custom_minimum_size = Vector2(420, 220)

    var vbox := VBoxContainer.new()
    vbox.add_theme_constant_override("separation", 8)
    _panel.add_child(vbox)

    var title := Label.new()
    title.text = "Lineage Rust - Login"
    vbox.add_child(title)

    _username_edit.placeholder_text = "Username"
    _username_edit.text = "adventurer"
    vbox.add_child(_username_edit)

    _class_edit.add_item("Prince")
    _class_edit.add_item("Knight")
    _class_edit.add_item("Elf")
    _class_edit.add_item("Wizard")
    _class_edit.add_item("DarkElf")
    _class_edit.select(1)
    vbox.add_child(_class_edit)

    var login_btn := Button.new()
    login_btn.text = "Login"
    login_btn.pressed.connect(_on_login_pressed)
    vbox.add_child(login_btn)

    _status_label.text = ""
    vbox.add_child(_status_label)

func _on_login_pressed() -> void:
    var username := _username_edit.text.strip_edges()
    if username.is_empty():
        _status_label.text = "Username required"
        return

    var class_name := _class_edit.get_item_text(_class_edit.get_selected_id())
    emit_signal("login_submitted", username, class_name)

func set_status(text: String) -> void:
    _status_label.text = text

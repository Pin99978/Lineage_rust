extends Node
class_name ClientNetwork

signal server_message_received(message: Variant)
signal network_error(message: String)

const DEFAULT_SERVER_IP := "127.0.0.1"
const DEFAULT_SERVER_PORT := 5000
const MAX_PACKET_SIZE := 4096

var _udp := PacketPeerUDP.new()
var _connected := false

func connect_to_server(ip: String = DEFAULT_SERVER_IP, port: int = DEFAULT_SERVER_PORT) -> bool:
    var err := _udp.bind(0)
    if err != OK:
        emit_signal("network_error", "UDP bind failed: %s" % err)
        return false

    _udp.set_dest_address(ip, port)
    _connected = true
    return true

func send_message(message: Dictionary) -> void:
    if not _connected:
        emit_signal("network_error", "Not connected")
        return

    var payload := LineageProtocol.encode_client_message(message)
    var err := _udp.put_packet(payload)
    if err != OK:
        emit_signal("network_error", "send failed: %s" % err)

func poll_messages() -> void:
    if not _connected:
        return

    while _udp.get_available_packet_count() > 0:
        var packet := _udp.get_packet()
        if packet.size() > MAX_PACKET_SIZE:
            emit_signal("network_error", "packet too large: %s" % packet.size())
            continue

        var decoded := LineageProtocol.decode_server_message(packet)
        if decoded == null:
            emit_signal("network_error", "JSON parse failed")
            continue

        emit_signal("server_message_received", decoded)

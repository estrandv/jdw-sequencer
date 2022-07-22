
from pythonosc import udp_client
from pythonosc import osc_bundle_builder
from pythonosc import osc_message_builder
from pythonosc import osc_server
from pythonosc import dispatcher

def create_msg(addr, args):
    msg = osc_message_builder.OscMessageBuilder(address=addr)
    for arg in args:
        msg.add_arg(arg)
    return msg.build()

def create_timed(time, msg):
    bundle = osc_bundle_builder.OscBundleBuilder(osc_bundle_builder.IMMEDIATELY)
    bundle.add_content(create_msg("/bundle_info", ["timed_msg"]))
    bundle.add_content(create_msg("/timed_msg_info", [time]))
    bundle.add_content(msg)
    return bundle.build()


# Hardcoded default port of jdw-sequencer main application
client = udp_client.SimpleUDPClient("127.0.0.1", 14441)

# TODO: Explain parts and name args
main_bundle = osc_bundle_builder.OscBundleBuilder(osc_bundle_builder.IMMEDIATELY)
main_bundle.add_content(create_msg("/bundle_info", ["update_queue"]))
main_bundle.add_content(create_msg("/update_queue_info", ["python_test_queue"]))

message_bundle = osc_bundle_builder.OscBundleBuilder(osc_bundle_builder.IMMEDIATELY)
message_bundle.add_content(create_timed(0.5, create_msg("/test", ["...."])))
message_bundle.add_content(create_timed(0.5, create_msg("/test", ["."])))
message_bundle.add_content(create_timed(0.5, create_msg("/test", ["."])))
message_bundle.add_content(create_timed(0.5, create_msg("/test", ["."])))

main_bundle.add_content(message_bundle.build())

client.send(main_bundle.build())

dispatcher = dispatcher.Dispatcher()
dispatcher.map("/test", print)

server = osc_server.ThreadingOSCUDPServer(
    ("127.0.0.1", 14443), dispatcher) # Out-port of the sequencer application
server.serve_forever()

from pythonosc import udp_client
from pythonosc import osc_bundle_builder
from pythonosc import osc_message_builder

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


# Hardcoded default port of jdw-sc main application
client = udp_client.SimpleUDPClient("127.0.0.1", 14447)

# TODO: Explain parts and name args
main_bundle = osc_bundle_builder.OscBundleBuilder(osc_bundle_builder.IMMEDIATELY)
main_bundle.add_content(create_msg("/bundle_info", ["update_queue"]))
main_bundle.add_content(create_msg("/update_queue_info", ["python_test_queue"]))

message_bundle = osc_bundle_builder.OscBundleBuilder(osc_bundle_builder.IMMEDIATELY)
message_bundle.add_content(create_timed(0.0, create_msg("/test", ["One"])))
message_bundle.add_content(create_timed(1.0, create_msg("/test", ["Two"])))

main_bundle.add_content(message_bundle.build())

# TODO: Not working. There is an issue with the sequencer. 
# The OSC client is constantly polling, so you can never get a lock on the out-sock
# Poll needs to be constant not to miss anything
# The only solution is to split out and in parts 
client.send(main_bundle.build())



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
main_bundle.add_content(create_msg("/update_queue_info", ["python_test_queue", 1])) # One shot flag == 1

bun = osc_bundle_builder.OscBundleBuilder(osc_bundle_builder.IMMEDIATELY)

#message_bundle.add_content(create_timed(1.0, create_msg("/play_sample", ["ext_id_1", "example", 2, "bd", "ofs", 0.13])))
def simple_sample(family, index, time, offset):
    return create_timed(time, create_msg("/play_sample", ["ext_id_", "example", index, family, "ofs", offset]))

def simple_note(time, gate, freq):
    bun.add_content(create_timed(time, create_msg("/note_on_timed", ["gentle", "gentle_" + str(freq), gate, "freq", freq, "relT", 0.5])))

simple_note("0.5", 3.2, 180.0)
bun.add_content(simple_sample("bd", 1, "0.5", 0.02))
#simple_note(0.0, 0.1, 320.0)
bun.add_content(simple_sample("bd", 0, "1.0", 0.08))
#simple_note(0.0, 0.2, 180.0)
bun.add_content(simple_sample("bd", 2, "1.0", 0.08))
#simple_note(0.25, 0.3, 200.0)
bun.add_content(simple_sample("sn", 0, "1.0", 0.08))

# Other test: See if "feel" is different when not using sample lookup
#message_bundle.add_content(create_timed(0.5, create_msg("/note_on_timed", ["sampler", "gentle_x", 0.5, "buf", 1.0])))
#message_bundle.add_content(create_timed(0.5, create_msg("/note_on_timed", ["sampler", "gentle_x", 0.5, "buf", 1.0])))
#message_bundle.add_content(create_timed(1.0, create_msg("/note_on_timed", ["sampler", "gentle_x", 0.5, "buf", 2.0])))

main_bundle.add_content(bun.build())

client.send(create_msg("/set_bpm", [220]))
client.send(main_bundle.build())

#dispatcher = dispatcher.Dispatcher()
#dispatcher.map("/play_sample", print)

#server = osc_server.ThreadingOSCUDPServer(
#    ("127.0.0.1", 13331), dispatcher) # Out-port of the sequencer application
#server.serve_forever()
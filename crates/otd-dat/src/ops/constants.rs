pub const NULL: &str = "nullDAT";
/// Fires Python callbacks every frame. Always a cook root — see
/// `otd_engine::execute`.
pub const EXECUTE: &str = "executeDAT";

pub const TABLE: &str = "tableDAT";
pub const TEXT: &str = "textDAT";
pub const SCRIPT: &str = "scriptDAT";
pub const WEBCLIENT: &str = "webclientDAT";
pub const WEBSERVER: &str = "webserverDAT";
pub const UDP_IN: &str = "udpinDAT";
pub const UDP_OUT: &str = "udpoutDAT";

pub const UDP_RECEIVE_CALLBACKS: &str = "\
# dat — this operator; rowIndex — table row (1 = first message row)
# message — text; bytes — space-separated byte values; peer — \"host:port\"
def onReceive(dat, rowIndex, message, bytes, peer):
    pass
";

pub const TCPIP_RECEIVE_CALLBACKS: &str = "\
# dat — this operator; rowIndex — table row (1 = first message row)
def onConnect(dat):
    pass

def onDisconnect(dat):
    pass

def onReceive(dat, rowIndex, message, peer):
    pass
";

pub const SERIAL_RECEIVE_CALLBACKS: &str = "\
def onReceive(dat, rowIndex, message):
    pass
";

pub const WEBSOCKET_CALLBACKS: &str = "\
def onOpen(dat):
    pass

def onClose(dat):
    pass

def onReceiveText(dat, rowIndex, message):
    pass
";

pub const MQTT_CLIENT_CALLBACKS: &str = "\
def onConnect(dat):
    pass

def onDisconnect(dat):
    pass

def onMessage(dat, topic, payload):
    pass
";

pub const FILE_IN: &str = "fileinDAT";
pub const FILE_OUT: &str = "fileoutDAT";
pub const FOLDER: &str = "folderDAT";
pub const INSERT: &str = "insertDAT";
pub const REORDER: &str = "reorderDAT";
pub const XML: &str = "xmlDAT";
pub const KEYBOARD_IN: &str = "keyboardinDAT";
pub const MONITOR: &str = "monitorDAT";
pub const MQTT_CLIENT: &str = "mqttclientDAT";
pub const NDI: &str = "ndiDAT";
pub const OSC_IN: &str = "oscinDAT";
pub const OSC_OUT: &str = "oscoutDAT";
pub const SERIAL: &str = "serialDAT";
pub const TCPIP: &str = "tcpipDAT";
pub const WEBSOCKET: &str = "websocketDAT";

pub const WEBCLIENT_CALLBACKS: &str = "\
\"\"\"webclientDAT callbacks\"\"\"

def onConnect(dat):
    pass

def onDisconnect(dat):
    pass

def onMessage(dat, message):
    pass
";

pub const WEBSERVER_CALLBACKS: &str = r##"
"""webserverDAT callbacks"""
from typing import Dict, Any

def onHTTPRequest(dat, request: Dict[str, Any], response: Dict[str, Any]) -> Dict[str, Any]:
	response['statusCode'] = 200
	response['statusReason'] = 'OK'
	response['content-type'] = 'text/html; charset=utf-8'
	response['data'] = 'hello from opentouchdesigner'
	return response

def onWebSocketOpen(dat, client: str, uri: str):
	return

def onWebSocketClose(dat, client: str):
	return

def onWebSocketReceiveText(dat, client: str, data: str):
	dat.webSocketSendText(client, data)
	return

def onWebSocketReceiveBinary(dat, client: str, data: bytes):
	dat.webSocketSendBinary(client, data)
	return

def onWebSocketReceivePing(dat, client: str, data: bytes):
	dat.webSocketSendPong(client, data=data)
	return

def onWebSocketReceivePong(dat, client: str, data: bytes):
	return

def onServerStart(dat):
	return

def onServerStop(dat):
	return
"##;

pub const EXECUTE_SOURCE: &str = "\
# Runs once, the first time this node cooks.
def onStart():
    pass

# Runs every frame, before the rest of the network cooks.
def onFrameStart(frame):
    pass

# Runs every frame, after it has.
def onFrameEnd(frame):
    pass
";

pub const CHOP_EXECUTE_SOURCE: &str = "\
# me - this DAT
#
# channel - the Channel object which has changed
# sampleIndex - the index of the changed sample
# val - the numeric value of the changed sample
# prev - the previous sample value
#
# Make sure the corresponding toggle is enabled in the CHOP Execute DAT.

def onOffToOn(channel, sampleIndex, val, prev):
    pass

def whileOn(channel, sampleIndex, val, prev):
    pass

def onOnToOff(channel, sampleIndex, val, prev):
    pass

def whileOff(channel, sampleIndex, val, prev):
    pass

def onValueChange(channel, sampleIndex, val, prev):
    pass
";

pub const PAR_EXECUTE_SOURCE: &str = "\
# A watched parameter changed. `value` and `prev` are numbers.
def onValueChange(par, value, prev):
    pass
";

pub const PANEL_EXECUTE_SOURCE: &str = "\
# Panel value changed (drag, click, keyboard).
def onValueChange(panelValue):
    pass

def onOffToOn(panelValue):
    pass

def onOnToOff(panelValue):
    pass
";

pub const PANEL_EXECUTE: &str = "panelexecuteDAT";
pub const PANEL_TABLE_TEXT: &str = "paneltabletextDAT";

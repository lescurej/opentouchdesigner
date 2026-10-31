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
pub const IN: &str = "inDAT";
pub const OUT: &str = "outDAT";

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
# A watched channel changed value.
def onValueChange(channel, value, prev):
    pass

# It crossed the threshold upwards, or downwards.
def onOffToOn(channel, value):
    pass

def onOnToOff(channel, value):
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

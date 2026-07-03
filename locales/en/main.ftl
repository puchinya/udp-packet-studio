# Tab Titles
tabs-collections = 📁 Collections
tabs-composer = 🚀 Composer
tabs-logs = 📊 Logs
tabs-inspector = 🔍 Inspector
tabs-multicast = 🌐 Multicast

# Title Bar / Bind Controls
titlebar-bind-addr = Bind:
titlebar-btn-stop = ⏹ Stop
titlebar-btn-bind = ▶ Bind
titlebar-status-active = ● Active
titlebar-status-offline = ● Offline
titlebar-preferences = Preferences...
titlebar-about = About...
about-title = About
about-desc = A professional desktop utility to compose, send, and inspect UDP packets, with support for decoding protocols like ECHONET Lite in real-time.
about-license-label = Application License (LICENSE.md):
about-show-oss = 🌐 Show Open Source Licenses
about-oss-title = Third Party Licenses
about-oss-description = This software is built using the following open source libraries and resources:
about-back = ⬅ Back to App Info


# Status Bar
statusbar-bound = Bound: { $addr }
statusbar-broadcast = 📣 Broadcast Enabled
statusbar-not-bound = Socket not bound
statusbar-auto-save-enabled = 💾 Auto-Save: Enabled ({ $format })
statusbar-auto-save-disabled = 💾 Auto-Save: Disabled

# Settings Dialog
settings-title = ⚙ Preferences
settings-tab-general = General
settings-tab-log-display = Log Display
settings-tab-log-saving = Log Saving
settings-tab-others = Others
settings-lang-section = Language Settings
settings-lang-label = Language:
settings-lang-system = System Setting
settings-lang-ja = 日本語 (Japanese)
settings-lang-en = English
settings-theme-section = Theme Settings
settings-theme-label = Theme:
settings-theme-system = System Setting
settings-theme-light = Light
settings-theme-dark = Dark
settings-auto-save-section = Log Auto-Save Settings
settings-auto-save-enable = Enable log auto-save
settings-auto-save-format = Log Format:
settings-auto-save-dir = Save Directory:
settings-browse = 📁 Browse...
settings-log-limit-section = Log Limit Settings
settings-max-display-bytes = Max Data Display Limit (Bytes):
settings-max-log-lines = Max Log Lines stored:
settings-layout-section = Layout Settings
settings-reset-layout-btn = Reset Window Layout
settings-close = Close
settings-reset = Reset Settings
settings-reset-confirm-title = Confirm Reset
settings-reset-confirm-msg = Are you sure you want to reset all settings? The application will restart automatically.
settings-ok = OK
settings-cancel = Cancel
dialog-error-title = Error

# Collections View
collections-new = + New
collections-new-tip = Create a new empty collection
collections-import = 📥 Import
collections-import-tip = Import a collection from a YAML file
collections-empty-list = No collections. Click 'New' to start!
collections-unnamed-col = Unnamed Collection
collections-del-col-tip = Delete Collection
collections-exp-col-tip = Export Collection (YAML)
collections-add-req-tip = Add Request
collections-empty-col = Empty collection
collections-unnamed-req = Unnamed Request
collections-del-req-tip = Delete Request
collections-send-tip = Send Packets
collections-invalid-payload-tip = Invalid payload format
collections-edit-title = 📝 Edit Request
collections-edit-name = Name:
collections-edit-target = Target:
collections-edit-target-ip = Target IP:
collections-edit-target-port = Target Port:
collections-edit-format = Format:
collections-edit-payload = Payload:
collections-edit-hex-tip = Example: 10 81 00 01 or 10810001
collections-edit-invalid-payload = ⚠ Invalid payload format: { $msg }
collections-edit-load = 📂 Load to Composer
collections-edit-send = 🚀 Send

# Tooltips and extra status info
statusbar-auto-save-tip = Click to toggle auto-save
statusbar-open-log-dir = 📁 Open Log Folder
statusbar-open-log-dir-tip = Click to open auto-save directory
statusbar-logged-packets = Logged packets: { $count }

# Collection actions and naming
collections-import-success = Collection imported successfully from { $path }
collections-import-fail-parse = Failed to parse YAML collection: { $msg }
collections-import-fail-read = Failed to read file: { $msg }
collections-export-success = Collection exported to { $path }
collections-export-fail = Failed to export collection: { $msg }
collections-created-name = Collection { $idx }
collections-req-created-name = Request { $idx }

# ECHONET Lite Helper
el-err-tid = Transaction ID (TID) must be exactly 2 bytes (4 hex characters)
el-err-seoj = Source Object (SEOJ) must be exactly 3 bytes (6 hex characters)
el-err-deoj = Destination Object (DEOJ) must be exactly 3 bytes (6 hex characters)
el-err-epc = Property Code (EPC) must be exactly 1 byte (2 hex characters)
el-err-edt-empty = Property Data (EDT) cannot be empty for Set/Write requests
el-err-edt-even = Property Data (EDT) must have an even number of hex characters

el-helper-checkbox = 💡 ECHONET Lite Packet Helper
el-builder-title = 💡 ECHONET Lite Frame Builder
el-label-tid = Transaction ID (TID):
el-label-seoj = Source Object (SEOJ):
el-label-deoj = Destination Object (DEOJ):
el-deoj-preset-ac = Home Air Conditioner (013001)
el-deoj-preset-meter = Smart Electric Meter (028801)
el-deoj-preset-node = Node Profile Object (0EF001)
el-deoj-preset-custom = Custom Object...

el-label-esv = Service Code (ESV):
el-esv-preset-get = Get (0x62 - Property Read Request)
el-esv-preset-getres = Get_Res (0x72 - Property Read Response)
el-esv-preset-setc = SetC (0x61 - Property Write, Response Required)
el-esv-preset-setcsna = SetC_SNA (0x71 - Property Write Not Accepted)
el-esv-preset-seti = SetI (0x60 - Property Write, No Response)
el-esv-preset-setisna = SetI_SNA (0x70 - Property Write Not Accepted, No Response)
el-esv-preset-infreq = INF_REQ (0x63 - Property Notification Request)
el-esv-preset-inf = INF (0x73 - Property Notification)
el-esv-preset-infsna = INF_SNA (0x74 - Property Notification Not Accepted)
el-esv-preset-infc = INFC (0x7A - Property Notification (Response Required))
el-esv-preset-infcres = INFC_Res (0x7B - Property Notification Response)
el-esv-preset-setget = SetGet (0x6E - Property Write & Read Request)
el-esv-preset-setgetres = SetGet_Res (0x7E - Property Write & Read Response)
el-esv-preset-setgetsna = SetGet_SNA (0x7F - Property Write & Read Not Accepted)
el-esv-preset-setres = Set_Res (0x71 - Property Write Response)
el-esv-preset-setires = SetI_Res (0x74 - Property Write Response, No Response Required)

el-label-epc = Property Code (EPC):
el-epc-preset-status = Operation Status (0x80)
el-epc-preset-mode = Operation Mode (0xB0)
el-epc-preset-power = Instantaneous Power (0xE0)
el-epc-preset-custom = Custom Property...

el-label-edt = Property Data (EDT, hex):
el-edt-on = ON (30)
el-edt-off = OFF (31)

el-btn-generate = ⚙ Generate and Insert ECHONET Lite Hex
el-btn-add-epc = + Add Property
el-err-prefix = ECHONET Lite builder error: { $msg }

# Composer Tab
composer-dest-addr = Destination Address:
composer-payload-format = Payload Format:
composer-format-text = Text (UTF-8)
composer-format-hex = Hex (Spaces optional)
composer-payload-content = Payload Content:
composer-invalid-payload = ⚠ Invalid payload format: { $msg }
composer-btn-send = 🚀 Send
composer-start-listener-tip = ⚠ Start listener socket first.

composer-save-title = 💾 Save Request to Collection
composer-save-name = Name:
composer-save-collection = Collection:
composer-save-no-collections = No collections. Click 'Save' to create one.
composer-btn-save = 💾 Save
composer-save-default-col = My Requests
composer-save-created-req = Request { $idx }

composer-ip-preset-section = Presets
composer-ip-preset-nif-bcast = NIF Broadcast
composer-ip-history-section = History

composer-port-preset-section = Presets
composer-port-preset-echonet = ECHONET Lite : 3610

# Log Viewer Tab
log-btn-clear = 🗑 Clear
log-btn-save = 💾 Save Logs
log-btn-save-tip = Export logs to selected format
log-checkbox-autoscroll = Auto-scroll
log-label-ip-filter = Filter:
log-filter-apply-btn = Apply
log-filter-tooltip = Packet filter usage:
  - port == <port>: match source or destination port
  - srcport == <port>: match source port only
  - dstport == <port>: match destination port only
  - ip == <ip_pattern>: match source or dest IP (e.g. 192.168.3.*, wildcard supported)
  - payload == <hex>: match full payload (e.g. 010203 or 01:02:03)
  - payload[1:3] == <hex>: match slice of payload
  - payload matches "<regex>": regex match on payload string (case-insensitive)
  - and / or / ( ): logical operators
  - Implicit IP filter applies if input starts with a digit
log-hdr-no = No.
log-hdr-time = Time
log-hdr-type = Type
log-hdr-source-ip = Src IP
log-hdr-send-port = Src Port
log-hdr-dest-ip = Dest IP
log-hdr-recv-port = Dest Port
log-hdr-length = Length
log-hdr-info = Info (Preview)
log-save-success = Logs saved successfully to { $path }
log-save-fail = Failed to save logs: { $msg }

# Inspector Panel
ins-label-timestamp = Timestamp: { $ts }
ins-label-sent-to = Sent To:
ins-label-received-from = Source:
ins-label-event-target = Event target:
ins-label-error-target = Error target:
ins-label-size = Length: { $len } B
ins-label-decode-as = Decode As:
ins-proto-raw = 🔌 Raw (Hex)
ins-proto-ascii = 📝 Text (ASCII)
ins-proto-echonet = 💡 ECHONET Lite
ins-title-hex-dump = Hex Dump View:
ins-title-ascii-view = ASCII Text View (with control code visualizers):
ins-title-echonet-decode = ECHONET Lite Protocol Decode:
ins-el-err-too-short = ⚠ Packet too short to be a valid ECHONET Lite frame (min 12 bytes).
ins-el-warn-ehd1 = ⚠ EHD1 is 0x{ $val } (Expected 0x10 for ECHONET Lite)
ins-el-label-ehd1 = EHD1 (Header 1):
ins-el-label-ehd2 = EHD2 (Header 2):
ins-el-format = Format { $fmt }
ins-el-label-tid = Transaction ID (TID):
ins-el-label-seoj = Source Object (SEOJ):
ins-el-label-deoj = Dest Object (DEOJ):
ins-el-label-esv = Service Code (ESV):
ins-el-label-opc = Property Count (OPC):
ins-el-title-props = Parsed Properties:
ins-el-err-malformed = ⚠ Malformed ECHONET Lite properties: Packet truncated.
ins-select-log-item = Select a log item in the logs panel to inspect its contents

# ECHONET Lite Decoded objects/values
ins-el-obj-unknown = Unknown
ins-el-obj-controller = Controller
ins-el-obj-node = Node Profile
ins-el-obj-ac = Home Air Conditioner
ins-el-obj-meter = Smart Meter
ins-el-obj-custom = Custom/Unknown Device

ins-el-esv-seti = SetI (Set Property - No Response Required)
ins-el-esv-setc = SetC (Set Property - Response Required)
ins-el-esv-get = Get (Get Property Value)
ins-el-esv-inf-req = INF_REQ (Property Value Write Request)
ins-el-esv-set-res = Set_Res (Set Property Response)
ins-el-esv-get-res = Get_Res (Get Property Response)
ins-el-esv-inf = INF (Inform Property Value)
ins-el-esv-infc = INFC (Inform Property Value Response)
ins-el-esv-seti-sna = SetI_SNA (Set SNA - No Response)
ins-el-esv-setc-sna = SetC_SNA (Set SNA Response)
ins-el-esv-get-sna = Get_SNA (Get SNA Response)
ins-el-esv-inf-sna = INF_SNA (Inform SNA Response)
ins-el-esv-unknown = Unknown Service (0x{ $esv })

ins-el-epc-status = -> Operation Status
ins-el-epc-location = -> Installation Location
ins-el-epc-version = -> Standard Version Info
ins-el-epc-id = -> Identification Number
ins-el-epc-fault = -> Fault Status
ins-el-epc-manufacturer = -> Manufacturer Code
ins-el-epc-mode = -> Operation Mode
ins-el-epc-temp = -> Set Temperature
ins-el-epc-temp-cool = -> Set Temp Cooling
ins-el-epc-node-instances = -> Self-node instance list
ins-el-epc-node-classes = -> Self-node class list

ins-el-edt-empty = Empty
ins-el-edt-on = ON (0x30)
ins-el-edt-off = OFF (0x31)
ins-el-edt-fault = Fault (0x41)
ins-el-edt-normal = Normal (0x42)
ins-el-edt-auto = Automatic (0x41)
ins-el-edt-cool = Cooling (0x42)
ins-el-edt-heat = Heating (0x43)
ins-el-edt-dehumid = Dehumidifying (0x44)
ins-el-edt-circulator = Air Circulator (0x45)
ins-el-edt-unknown = Unknown (0x{ $val })
ins-el-edt-temp = { $temp } °C (0x{ $val })

# Multicast Panel
mc-status-offline = ⚠ Listener Offline:
mc-status-offline-tip = You must Bind to a local port in the title bar first before joining multicast groups.
mc-join-title = 🌐 Join a Multicast Group
mc-label-multicast-ip = Multicast IP:
mc-label-interface-ip = Local Interface IP:
mc-label-presets = Quick Presets:
mc-preset-tip = Join { $ip }
mc-btn-join = + Join Multicast Group
mc-title-joined-list = 👥 Currently Joined Groups
mc-no-memberships = No active multicast memberships on this socket.
mc-hdr-multicast-addr = Group
mc-hdr-interface-addr = Interface
mc-btn-leave = Leave
mc-err-empty-fields = Multicast address and Interface address cannot be empty.

# Sockets Window / Dropdown
sockets-window-title = 🔌 Sockets Manager
sockets-btn-add = ➕ Add Socket
sockets-lbl-name = Name:
sockets-tooltip-delete = Delete this socket
sockets-navbar-btn-list = 🔌 Sockets...

# Syslog & SNMP Inspector
ins-proto-syslog = 📝 Syslog
ins-proto-snmp = 📡 SNMP
ins-syslog-rfc = Standard:
ins-syslog-priority = Priority:
ins-syslog-facility = Facility:
ins-syslog-severity = Severity:
ins-syslog-timestamp = Timestamp:
ins-syslog-hostname = Hostname:
ins-syslog-appname = App Name:
ins-syslog-procid = Process ID:
ins-syslog-msgid = Message ID:
ins-syslog-message = Message:

ins-snmp-version = Version:
ins-snmp-community = Community:
ins-snmp-pdutype = PDU Type:
ins-snmp-reqid = Request ID:
ins-snmp-errstatus = Error Status:
ins-snmp-errindex = Error Index:
ins-snmp-varbinds = Variable Bindings:

# Syslog & SNMP Helper
syslog-helper-checkbox = 📝 Syslog Builder
snmp-helper-checkbox = 📡 SNMP Builder

syslog-version = RFC Version:
syslog-auto-ts = Auto Timestamp
syslog-hostname-lbl = Hostname:
syslog-appname-lbl = App Name:
syslog-procid-lbl = Proc ID:
syslog-msgid-lbl = Msg ID:
syslog-msg-lbl = Message:
syslog-btn-generate = Generate Syslog

snmp-version-lbl = SNMP Version:
snmp-community-lbl = Community:
snmp-pdutype-lbl = PDU Type:
snmp-reqid-lbl = Request ID:
snmp-errstatus-lbl = Error Status:
snmp-errindex-lbl = Error Index:
snmp-varbind-add = ➕ Add Variable
snmp-varbind-oid = OID:
snmp-varbind-type = Type:
snmp-varbind-val = Value:
snmp-btn-generate = Generate SNMP





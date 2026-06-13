# UDP Packet Studio - Log Export File Specifications

This document defines the schema and file formats for exporting communication logs in UDP Packet Studio. Logs can be exported in **CSV (Comma-Separated Values)**, **JSON (JavaScript Object Notation)**, or **PCAP (Packet Capture)** formats.

---

## 1. CSV Format Specification

The CSV export uses standard comma-separated values, using double-quotes `"` to escape strings. The file starts with a header row.

### Header Fields

| Field Name | Type | Description |
| :--- | :--- | :--- |
| `No` | Integer | The sequential index of the log entry (1-based). |
| `Timestamp` | String | Local date and time of the event in `%Y-%m-%d %H:%M:%S.%3f` format (e.g. `2026-06-13 14:20:57.123`). |
| `Direction` | String | Event type. Must be one of `SENT`, `RECV`, `INFO`, `ERROR`. |
| `IP` | String | The remote network IP address. System events use `-`. |
| `Port` | String | The remote network port. System events use `-`. |
| `Length` | Integer | Length of the payload data in bytes. |
| `DataHex` | String | Space-separated hex-encoded bytes of the payload (e.g., `10 81 00 01`). |
| `DataText` | String | Plain-text fallback representation of the payload. Newlines are replaced by spaces, and double quotes are escaped. |

### Concrete Example

```csv
No,Timestamp,Direction,IP,Port,Length,DataHex,DataText
1,"2026-06-13 14:20:57.123","SENT","127.0.0.1","3610",14,"10 81 00 01 05 FF 01 01 30 01 62 01 80 00","..........b..."
2,"2026-06-13 14:20:57.125","RECV","127.0.0.1","3610",14,"10 81 00 01 05 FF 01 01 30 01 72 01 80 01 30","..........r...0"
3,"2026-06-13 14:20:58.000","INFO","-","-",33,"","Listening socket bound to 127.0.0.1:3610"
```

---

## 2. JSON Format Specification

The JSON export is serialized as a root-level JSON array of packet log entry objects.

### Field Definitions

| JSON Key | Type | Description |
| :--- | :--- | :--- |
| `timestamp` | String | RFC 3339 formatted local datetime with timezone (e.g., `"2026-06-13T14:20:57.123456+09:00"`). |
| `direction` | String | Event direction/type. Serialized enum: `"Sent"`, `"Received"`, `"SystemInfo"`, `"SystemError"`. |
| `ip` | String | Remote IP address (e.g., `"127.0.0.1"`). System events use `"-"`. |
| `port` | String | Remote port (e.g., `"3610"`). System events use `"-"`. |
| `data` | Array | Array of unsigned 8-bit integers representing the raw payload bytes. |

### Concrete Example

```json
[
  {
    "timestamp": "2026-06-13T14:20:57.123456+09:00",
    "direction": "Sent",
    "ip": "127.0.0.1",
    "port": "3610",
    "data": [16, 129, 0, 1, 5, 255, 1, 1, 48, 1, 98, 1, 128, 0]
  },
  {
    "timestamp": "2026-06-13T14:20:57.125123+09:00",
    "direction": "Received",
    "ip": "127.0.0.1",
    "port": "3610",
    "data": [16, 129, 0, 1, 5, 255, 1, 1, 48, 1, 114, 1, 128, 1, 48]
  },
  {
    "timestamp": "2026-06-13T14:20:58.000000+09:00",
    "direction": "SystemInfo",
    "ip": "-",
    "port": "-",
    "data": [76, 105, 115, 116, 101, 110, 105, 110, 103, 32, 115, 111, 99, 107, 101, 116, 32, 98, 111, 117, 110, 100, 32, 116, 111, 32, 49, 50, 55, 46, 48, 46, 48, 46, 49, 58, 51, 54, 49, 48]
  }
]
```

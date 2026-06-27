# ECHONET Lite/UDP Packet Studio - Collection File Specification

This document defines the schema and file specification for exporting and importing packet collections in UDP Packet Studio. Collections are represented in **YAML (YAML Ain't Markup Language)** format.

---

## 1. Schema Overview

An exported collection is serialized as a root-level YAML object containing the collection's metadata and a nested list of request definitions.

### Field Definitions

| YAML Key | Type | Required | Description |
| :--- | :--- | :--- | :--- |
| `name` | String | Yes | The user-defined display name of the collection. |
| `requests` | List | Yes | An array of request objects (packet definitions) grouped under this collection. |

### Request Field Definitions (nested under `requests`)

| YAML Key | Type | Required | Description |
| :--- | :--- | :--- | :--- |
| `name` | String | Yes | Display name of the request. |
| `target_ip` | String | Yes | Target destination IP address (e.g., `127.0.0.1`). |
| `target_port` | String | Yes | Target destination port number (e.g., `3610`). |
| `payload_type` | String | Yes | Format of the payload. Must be either `"Text"` or `"Hex"`. |
| `payload` | String | Yes | The raw payload content (plain text or space-separated hex bytes). |

---

## 2. Concrete Example

Below is a valid example of a collection containing two ECHONET Lite queries (ordered sequentially, with internal IDs and transient folder state omitted):

```yaml
name: ECHONET Lite Queries
requests:
  - name: Aircon Get Operation
    target_ip: 127.0.0.1
    target_port: 3610
    payload_type: Hex
    payload: 10 81 00 01 05 FF 01 01 30 01 62 01 80 00
  - name: Node Profile Get
    target_ip: 127.0.0.1
    target_port: 3610
    payload_type: Hex
    payload: 10 81 00 02 05 FF 01 0E F0 01 62 01 D6 00
```

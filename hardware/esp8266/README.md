# Familiar ESP8266 display

This firmware renders Familiar's 小虎 (`tabby-cat`) sprite pack on the integrated
128×128 ST7735 display and follows the authenticated `/api/v1/display-stream` WebSocket.
The device is a renderer only: prompts, activity summaries, notifications, and
agent identifiers are not sent by the display endpoint.

## Hardware

The default PlatformIO environment targets the photographed ESP-12F/CH340C
integrated display board with at least 4 MB flash. Its PCB is expected to wire
the ST7735 as follows:

| ST7735 | ESP8266 GPIO | D1 mini label |
| --- | ---: | --- |
| SCL/SCK | 14 | D5 |
| SDA/MOSI | 13 | D7 |
| CS | 15 | D8 |
| DC | 0 | D3 |
| RST | 2 | D4 |
| GND | GND | GND |
| VCC | 3.3 V | 3V3 |

Connect the backlight according to the display module's specification. Do not
drive a bare LED backlight from an ESP8266 GPIO without suitable current
limiting or a transistor.

## Server setup

Keep the endpoint inside a trusted LAN. In the server's TOML configuration:

```toml
[server]
bind = "0.0.0.0:19528"

[server.auth]
enabled = true
token_file = "/absolute/private/path/familiar-token"
auto_generate = true
```

Start the headless backend with the same config file:

```bash
familiar-cli serve --config /absolute/path/server.toml
familiar-cli auth show --config /absolute/path/server.toml
```

Allow TCP port 19528 only from the local subnet. Do not forward it from the
internet. The firmware uses unencrypted `ws://`; Wi-Fi credentials and the
Bearer token are visible to anyone able to inspect the firmware or trusted LAN.

## Build and flash

Create an isolated environment containing both PlatformIO and the pinned image
conversion dependency, then create the local secrets file:

```bash
python3 -m venv .venv
.venv/bin/python -m pip install platformio==6.1.18 -r tools/requirements.txt
cp include/secrets.example.h include/secrets.h
```

Edit `include/secrets.h` with the Wi-Fi details, server's LAN IPv4 address, and
token. Build and upload from this directory:

```bash
.venv/bin/pio run
.venv/bin/pio run --target upload
.venv/bin/pio device monitor
```

The PlatformIO pre-build hook regenerates the assets from
`../../sprites/tabby-cat`. It produces nine 192×192 previews, a contact sheet,
and `src/generated/tabby_assets.h`. Generation uses a shared 31-color palette
plus transparent index 0 and verifies each RLE stream by decoding it back to
the indexed image.

To verify checked-in generated assets without rewriting them:

```bash
python3 tools/convert_tabby_assets.py \
  --source ../../sprites/tabby-cat --output . --check
```

## Display behavior

- The top bar shows `CONNECTING`, `ONLINE`, or `OFFLINE`.
- The center contains the 96×96 小虎 sprite, decoded directly from the 2× source asset.
- The bottom bar shows the active agent count.
- The last valid state remains visible while disconnected.
- WebSocket retries use an exponential interval from 1 to 30 seconds.
- A new server ID resets revision ordering after a backend restart.

If colors are inverted on a different ST7735 module, change the ST7735 tab
variant or `TFT_RGB_ORDER` in `platformio.ini`. If the image is
rotated, adjust `setRotation(0)` in `src/pet_renderer.cpp`.

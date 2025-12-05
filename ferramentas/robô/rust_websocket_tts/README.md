# Rust WebSocket Audio Client for Robot

A WebSocket client that connects to a TTS server, receives base64-encoded audio data, and plays it immediately through the robot's speakers.

## Features

- **WebSocket Client**: Connects to remote TTS WebSocket server
- **Base64 Decoding**: Automatically decodes base64-encoded audio
- **Multiple Format Support**: Plays MP3, WAV, OGG, FLAC
- **Auto-Reconnection**: Automatically reconnects if connection is lost
- **Instant Playback**: Audio plays as soon as it's received
- **Pure Rust**: No external dependencies except audio libraries

## Architecture

```
Backend Server (10.140.0.11:8000/v1/audio)
    │
    │ WebSocket Connection
    │ Sends 3 messages in sequence:
    │   1. Text: {"message": "...", "data": {"texto": "..."}}
    │   2. Binary: Raw audio bytes (MP3/WAV/OGG)
    │   3. Text: {"done": true}
    │
    ▼
Robot (this client)
    │
    ├─ Receive text response (log it)
    ├─ Receive binary audio (raw bytes, not base64)
    ├─ Detect audio format from magic bytes
    ├─ Decode audio format (MP3/WAV/OGG/FLAC)
    ├─ Play through speakers
    └─ Receive done signal (mark complete)
```

## Prerequisites

- Rust 1.70 or later
- Audio output device (speakers)
- Network access to TTS server

### Linux Dependencies

```bash
# Ubuntu/Debian
sudo apt-get install libasound2-dev pkg-config

# Fedora
sudo dnf install alsa-lib-devel
```

### macOS
No additional dependencies needed.

## Building

```bash
cargo build --release
```

## Running

### Default Configuration

Connects to `ws://10.140.0.11:8000/v1/audio`:

```bash
cargo run --release
```

### Custom Server URL

```bash
# Connect to different server
export WS_URL="ws://192.168.1.100:8000/v1/audio"
cargo run --release
```

## Message Format

The backend sends **three separate messages** for each response:

### 1. Text Response (JSON)
```json
{
  "message": "Question processed and answered successfully",
  "data": {
    "id": 789,
    "pergunta_id": 123,
    "respondido_por_tipo": "modelo",
    "texto": "O museu foi fundado em 1950...",
    "criado_em": "2025-12-05T10:00:01Z"
  }
}
```

### 2. Binary Audio Data
- **Format:** Raw audio bytes (MP3/WAV/OGG/FLAC)
- **Encoding:** NOT base64-encoded, direct binary data
- **Size:** Variable (typically several KB)
- **Detection:** Client auto-detects format from magic bytes

### 3. Done Signal (JSON)
```json
{"done": true}
```

### Error Response
If an error occurs:
```json
{"error": "Error message here"}
```

## Supported Audio Formats

- ✅ MP3
- ✅ WAV
- ✅ OGG/Vorbis
- ✅ FLAC

## How It Works

1. **Connect**: Client connects to WebSocket server at `ws://10.140.0.11:8000/v1/audio`
2. **Listen**: Waits for messages from backend server
3. **Receive Text**: Gets JSON response with text answer from the model
4. **Receive Audio**: Gets raw binary audio data (not base64)
5. **Detect Format**: Auto-detects audio format from magic bytes (MP3/WAV/OGG/FLAC)
6. **Play**: Plays audio through speakers immediately
7. **Receive Done**: Gets completion signal `{"done": true}`
8. **Repeat**: Continues listening for more responses
9. **Auto-Reconnect**: If disconnected, reconnects after 5 seconds

## Project Structure

```
src/
├── main.rs                 # Entry point, connection management
├── audio_decoder.rs        # Audio decoding and playback (rodio)
└── utils/
    └── websocket_handler.rs # WebSocket client
```

## Configuration

Environment variables:

| Variable | Default | Description |
|----------|---------|-------------|
| `WS_URL` | `ws://10.140.0.11:8000/v1/audio` | WebSocket server URL |

## Logs

The application provides detailed logging:

```
=== Rust WebSocket Audio Client ===
Configuration:
  WebSocket URL: ws://10.140.0.11:8000/v1/audio
Connecting to WebSocket server...
Connected successfully!
Received text message: 248 bytes
✓ Text response received: O museu foi fundado em 1950...
Received binary audio message: 15234 bytes
Playing audio response for: "O museu foi fundado em 1950..."
Processing raw binary audio: 15234 bytes
Detected audio format: mp3
Audio playback started
Audio playback completed
✓ Audio playback completed successfully
Received text message: 14 bytes
✓ Processing complete signal received
```

## Troubleshooting

### Cannot connect to server

```
Error: Failed to connect to WebSocket server
```

**Solutions:**
- Check server is running: `curl http://10.140.0.11:8000/health`
- Check network connectivity: `ping 10.140.0.11`
- Verify server URL is correct
- Check firewall rules

### No audio output

**Solutions:**
- Check speakers are connected
- Check volume is up
- Linux: `aplay -l` to list audio devices
- macOS: Check System Preferences → Sound

### Build errors on Linux

```
error: failed to run custom build command for `alsa-sys`
```

**Solution:**
```bash
sudo apt-get install libasound2-dev pkg-config
```

### Connection keeps disconnecting

The client will automatically reconnect every 5 seconds. This is normal behavior if:
- Server is restarting
- Network is unstable
- Server closes idle connections

## Testing

### Test Server (Python)

Create a test server to simulate the backend:

```python
import asyncio
import websockets
import json

async def handle_client(websocket):
    print("Client connected")
    
    # Read test audio file
    with open("test.mp3", "rb") as f:
        audio_bytes = f.read()
    
    # 1. Send text response
    text_response = {
        "message": "Question processed and answered successfully",
        "data": {
            "id": 1,
            "pergunta_id": 123,
            "respondido_por_tipo": "modelo",
            "texto": "Este é o museu Catavento, fundado em 2009.",
            "criado_em": "2025-12-05T10:00:01Z"
        }
    }
    await websocket.send(json.dumps(text_response))
    print("✓ Sent text response")
    
    # 2. Send binary audio (raw bytes, not base64)
    await websocket.send(audio_bytes)
    print(f"✓ Sent binary audio: {len(audio_bytes)} bytes")
    
    # 3. Send done signal
    await websocket.send(json.dumps({"done": True}))
    print("✓ Sent done signal")

async def main():
    async with websockets.serve(handle_client, "0.0.0.0", 8000):
        print("Test server running on ws://0.0.0.0:8000")
        await asyncio.Future()  # run forever

asyncio.run(main())
```

Then run the client:
```bash
WS_URL="ws://localhost:8000" cargo run --release
```

## Connecting to Backend

The client is designed to work with the backend server at:
```
ws://10.140.0.11:8000/v1/audio
```

**Important Notes:**
- The backend sends responses for questions processed through its ML pipeline
- You cannot send messages to this endpoint - it's receive-only
- The backend initiates audio responses when:
  - A visitor asks a question via the web interface
  - The robot's speech-to-text system sends a question
  - A text query is processed through the `/v1/modelo` API

To test sending questions to the backend, use the backend's WebSocket endpoints directly (requires authentication).

## Deployment on Robot

### 1. Build for release

```bash
cargo build --release
```

### 2. Copy binary to robot

```bash
# Binary location
./target/release/rust_websocket_tts

# Copy to robot
scp ./target/release/rust_websocket_tts robot@10.140.0.11:/home/robot/
```

### 3. Run on robot

```bash
ssh robot@10.140.0.11
./rust_websocket_tts
```

### 4. Run as system service (optional)

Create `/etc/systemd/system/audio-client.service`:

```ini
[Unit]
Description=WebSocket Audio Client
After=network.target

[Service]
Type=simple
User=robot
ExecStart=/home/robot/rust_websocket_tts
Restart=always
RestartSec=5
Environment="WS_URL=ws://10.140.0.11:8000/v1/audio"

[Install]
WantedBy=multi-user.target
```

Enable and start:
```bash
sudo systemctl enable audio-client
sudo systemctl start audio-client
sudo systemctl status audio-client
```

## Development

### Enable debug logging

```bash
RUST_LOG=debug cargo run
```

### Build for different target

```bash
# For ARM (common in robots)
cargo build --release --target armv7-unknown-linux-gnueabihf
```

## Performance

- **Latency**: Audio starts playing immediately upon receipt
- **Memory**: Minimal - processes one message at a time
- **CPU**: Low - async I/O, efficient decoding
- **Network**: Handles reconnections gracefully

## License

MIT

## Contributing

Contributions welcome! Please ensure:
- Code compiles without warnings
- Tests pass
- Follows Rust formatting (`cargo fmt`)
- Passes clippy lints (`cargo clippy`)

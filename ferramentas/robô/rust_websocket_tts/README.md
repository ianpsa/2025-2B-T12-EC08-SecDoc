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
TTS Server (10.140.0.11:8000/v1/audio)
    │
    │ WebSocket Connection
    │ Sends: {"audio_data": "<base64>", "format": "mp3"}
    │
    ▼
Robot (this client)
    │
    ├─ Receive WebSocket message
    ├─ Decode base64 → audio bytes
    ├─ Decode audio format (MP3/WAV/etc.)
    └─ Play through speakers
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

The client expects to receive JSON messages:

```json
{
  "audio_data": "<base64-encoded-audio>",
  "format": "mp3",
  "message_id": "optional-id"
}
```

### Fields

- `audio_data` (required): Base64-encoded audio data
- `format` (optional): Audio format - `"mp3"`, `"wav"`, `"ogg"`, `"flac"` (default: "mp3")
- `message_id` (optional): ID for tracking/logging

## Supported Audio Formats

- ✅ MP3
- ✅ WAV
- ✅ OGG/Vorbis
- ✅ FLAC

## How It Works

1. **Connect**: Client connects to WebSocket server at `ws://10.140.0.11:8000/v1/audio`
2. **Listen**: Waits for audio messages from server
3. **Receive**: Gets JSON with base64-encoded audio
4. **Decode**: Decodes base64 → audio bytes
5. **Play**: Plays audio through speakers
6. **Repeat**: Continues listening for more messages
7. **Auto-Reconnect**: If disconnected, reconnects after 5 seconds

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
Received text message: 15234 bytes
Processing audio message - Format: mp3
Decoded 15000 bytes of audio data
Audio playback started
Audio playback completed
Audio playback completed successfully
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

Create a test server to send audio:

```python
import asyncio
import websockets
import json
import base64

async def send_audio(websocket):
    # Read test audio file
    with open("test.mp3", "rb") as f:
        audio_data = base64.b64encode(f.read()).decode('utf-8')
    
    # Send to client
    message = {
        "audio_data": audio_data,
        "format": "mp3"
    }
    await websocket.send(json.dumps(message))
    print("Audio sent!")

async def main():
    async with websockets.serve(send_audio, "0.0.0.0", 8000):
        print("Test server running on ws://0.0.0.0:8000")
        await asyncio.Future()  # run forever

asyncio.run(main())
```

Then run the client:
```bash
WS_URL="ws://localhost:8000" cargo run --release
```

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

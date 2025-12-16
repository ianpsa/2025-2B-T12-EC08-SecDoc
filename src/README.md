# WebRTC Audio Streamer for Unitree Go2

This Rust application streams audio to the Unitree Go2 robot via WebRTC, receiving audio data through WebSocket connections.

## Overview

This service is based on the Python examples in `ferramentas/robô/rust_websocket_tts/go2_webrtc/examples/go2/audio/`, specifically:
- `mp3_player/play_mp3.py` - Main audio playback example
- `mp3_player/webrtc_audio_player.py` - WebRTC audio hub implementation

## Architecture

```
┌─────────────────┐       WebSocket        ┌─────────────────────┐
│  Audio Client   │─────(MP3/WAV/PCM)─────>│  WebSocket Server   │
│  (Laptop/PC)    │       Port 8080         │   (This Service)    │
└─────────────────┘                         └─────────────────────┘
                                                      │
                                                      │ Audio Pipeline
                                                      │ (Decode + Process)
                                                      ↓
                                            ┌─────────────────────┐
                                            │  WebRTC Connection  │
                                            │   (Audio Track)     │
                                            └─────────────────────┘
                                                      │
                                                      │ WebRTC (Opus)
                                                      ↓
                                            ┌─────────────────────┐
                                            │   Unitree Go2 Robot │
                                            │   (Plays Audio)     │
                                            └─────────────────────┘
```

### Key Components

1. **WebSocket Server** (`utils/websocket_server.rs`)
   - Listens on `0.0.0.0:8080` by default
   - Accepts audio data in binary or base64-encoded format
   - Forwards audio to the processing pipeline

2. **Audio Processing Pipeline** (`utils/streaming_pipeline.rs`)
   - Decodes MP3/WAV/other formats to PCM
   - Establishes WebRTC connection on first audio
   - Streams PCM audio to the robot

3. **WebRTC Connection Manager** (`utils/webrtc_connection.rs`)
   - Manages WebRTC peer connection
   - Handles signaling with robot
   - Adds audio tracks (like Python's `conn.pc.addTrack()`)

4. **Audio Decoder** (`utils/audio_decoder.rs`)
   - Uses FFmpeg to decode various audio formats
   - Converts to 16kHz mono PCM (robot requirement)

5. **WebRTC Audio Player** (`utils/webrtc_audio_player.rs`)
   - Creates audio tracks for WebRTC
   - Streams PCM samples to the robot

## Requirements

### System Dependencies

```bash
# Install FFmpeg libraries (required for audio decoding)
sudo apt-get install -y libavformat-dev libavcodec-dev libavutil-dev libswresample-dev

# Install Rust (if not already installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

### Rust Dependencies

All dependencies are defined in `Cargo.toml`:
- `tokio` - Async runtime
- `tokio-tungstenite` - WebSocket server
- `webrtc` - WebRTC implementation
- `ffmpeg-next` - FFmpeg bindings for audio decoding
- `reqwest` - HTTP client for signaling
- And more...

## Building

```bash
cd src/
cargo build --release
```

The compiled binary will be at: `target/release/webrtc_audio_streamer`

## Deployment

⚠️ **IMPORTANT**: This service **must run ON the robot itself**, not on your laptop!

The robot's WebRTC server is only accessible via `localhost` (127.0.0.1) on the robot.

### Option 1: Run directly on robot

1. Copy the entire `src/` directory to the robot:
```bash
scp -r src/ unitree@192.168.123.161:~/webrtc_audio_streamer/
```

2. SSH into the robot and build:
```bash
ssh unitree@192.168.123.161
cd ~/webrtc_audio_streamer/
cargo build --release
```

3. Run the service:
```bash
./target/release/webrtc_audio_streamer
```

### Option 2: Cross-compile for robot (faster)

If your robot uses ARM architecture (like Go2):

```bash
# Install cross-compilation tools
rustup target add aarch64-unknown-linux-gnu

# Cross-compile
cargo build --release --target aarch64-unknown-linux-gnu

# Copy binary to robot
scp target/aarch64-unknown-linux-gnu/release/webrtc_audio_streamer unitree@192.168.123.161:~/
```

## Configuration

Configure via environment variables:

| Variable | Default | Description |
|----------|---------|-------------|
| `ROBOT_IP` | `127.0.0.1` | Robot's WebRTC server IP (use localhost when running ON robot) |
| `WEBSOCKET_ADDR` | `0.0.0.0:8080` | WebSocket server bind address |

### Examples

```bash
# Default (running on robot, connecting to localhost)
cargo run --release

# Custom WebSocket port
WEBSOCKET_ADDR=0.0.0.0:9090 cargo run --release

# If testing from external machine (NOT RECOMMENDED - won't work with Go2's WebRTC)
ROBOT_IP=192.168.123.161 cargo run --release
```

## Usage

### 1. Start the service on the robot

```bash
./target/release/webrtc_audio_streamer
```

Output:
```
==============================================
  Audio WebSocket to Unitree Go2 WebRTC
  Direct Audio Track Streaming
==============================================

  WebSocket: ws://0.0.0.0:8080
  Robot IP: 127.0.0.1
  
  ⏳ Waiting for first audio via WebSocket...
  ⏳ WebRTC will connect automatically on first audio
==============================================
```

### 2. Send audio from your laptop

You can use the Python client from `ferramentas/robô/rust_websocket_tts/`:

```bash
# From your laptop (not the robot)
cd ferramentas/robô/rust_websocket_tts/
python3 audio_stream.py --file test_tone.mp3
```

Or create a simple WebSocket client:

```python
import websockets
import asyncio
import base64

async def send_audio():
    uri = "ws://192.168.123.161:8080"  # Robot's IP
    
    async with websockets.connect(uri) as websocket:
        # Read audio file
        with open("audio.mp3", "rb") as f:
            audio_data = f.read()
        
        # Send as base64 text message
        audio_b64 = base64.b64encode(audio_data).decode('utf-8')
        await websocket.send(audio_b64)
        
        # Or send as binary message
        # await websocket.send(audio_data)
        
        # Wait for acknowledgment
        response = await websocket.recv()
        print(f"Server: {response}")

asyncio.run(send_audio())
```

### 3. Test with curl (for raw PCM)

```bash
# Generate test tone (16kHz mono PCM)
ffmpeg -f lavfi -i "sine=frequency=440:duration=1" -ar 16000 -ac 1 -f s16le tone.pcm

# Send via WebSocket (requires websocat)
cat tone.pcm | websocat ws://192.168.123.161:8080
```

## Supported Audio Formats

The service automatically detects and decodes:
- **MP3** - MPEG Layer 3 audio
- **WAV** - Waveform Audio File Format
- **PCM** - Raw PCM audio (16-bit signed, little-endian)
- **OGG** - Ogg Vorbis
- **FLAC** - Free Lossless Audio Codec
- **AAC** - Advanced Audio Coding
- Any format supported by FFmpeg

All formats are converted to **16kHz mono PCM** before streaming to the robot.

## Troubleshooting

### "Failed to establish WebRTC connection"

**Possible causes:**
1. Service not running on the robot (must run ON robot, not laptop)
2. Robot's WebRTC service not running
3. Incorrect ROBOT_IP (should be `127.0.0.1` when running on robot)

**Solutions:**
```bash
# Check if robot's WebRTC service is running
curl http://127.0.0.1:8081/

# Verify you're running the service ON the robot
hostname  # Should show robot's hostname

# Use correct ROBOT_IP
ROBOT_IP=127.0.0.1 ./webrtc_audio_streamer
```

### "Timeout waiting for WebRTC connection"

The signaling succeeded but ICE/DTLS handshake failed.

**Solutions:**
- Ensure robot is not sleeping/standby mode
- Check robot's network connectivity
- Restart robot's WebRTC service

### "Failed to decode audio"

Invalid or corrupted audio data.

**Solutions:**
- Verify audio file is valid: `ffplay audio.mp3`
- Check WebSocket is sending complete file
- Try a different audio format

### "WebSocket connection closed"

Client disconnected.

**Normal behavior** - the service will wait for new connections.

## Performance Notes

- **Latency**: ~100-200ms from WebSocket to robot playback
- **Throughput**: Supports multiple audio streams sequentially
- **Memory**: ~50MB baseline + audio buffers
- **CPU**: Minimal when idle, ~10-20% during decoding

## Comparison with Python Implementation

| Feature | Python (play_mp3.py) | This (Rust) |
|---------|---------------------|-------------|
| Audio Decoding | aiortc MediaPlayer | FFmpeg + Custom |
| WebRTC | aiortc | webrtc-rs |
| Performance | Moderate | High |
| Deployment | pip install | Compile + Copy |
| Memory Usage | Higher | Lower |
| Streaming | Direct file | WebSocket chunks |

## Related Files

- Python examples: `ferramentas/robô/rust_websocket_tts/go2_webrtc/examples/go2/audio/`
- Full Rust implementation: `ferramentas/robô/rust_websocket_tts/`
- Deployment guide: `ferramentas/robô/rust_websocket_tts/DEPLOY_ON_ROBOT.md`

## License

See repository root for license information.

## Contributing

Improvements welcome! Areas of interest:
- Reduce audio latency
- Support real-time streaming (chunked audio)
- Add audio transcoding options
- Improve error handling

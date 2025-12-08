# Rust WebSocket → ROS2 Audio Bridge for Unitree GO2

A WebSocket client that connects to a TTS server, receives audio (MP3/base64), decodes it to PCM, and publishes it to ROS2 for playback on the Unitree GO2 robot.

## Features

- **WebSocket Client**: Connects to remote TTS WebSocket server
- **Base64 Decoding**: Automatically decodes base64-encoded audio (if sent as text)
- **MP3 Decoding**: Converts MP3 to raw PCM audio using Symphonia
- **ROS2 Publishing**: Publishes PCM audio to ROS2 topics
- **Unitree GO2 Integration**: Specifically designed for Unitree GO2 robot audio playback
- **Auto-Reconnection**: Automatically reconnects if connection is lost
- **Pure Rust**: High performance with minimal dependencies

## Architecture

```
WebSocket Server → Base64 Decode (if needed) → MP3 Decode → ROS2 Publish → Unitree GO2 Speaker
```

### Detailed Flow

```
1. WebSocket receives audio (binary MP3 or base64-encoded)
   ↓
2. If base64: decode to raw MP3 bytes
   ↓
3. MP3 Decoder (Symphonia): MP3 → PCM (S16LE format)
   ↓
4. ROS2 Publisher: publish PCM to "audiodata" topic
   ↓
5. Unitree GO2: subscribes to topic and plays audio
```

## Prerequisites

### System Requirements
- **Rust 1.70 or later**
- **ROS2** (Humble, Iron, or compatible)
- **Unitree GO2 robot** or ROS2 environment
- Network access to TTS server

### Linux Dependencies

```bash
# Ubuntu/Debian (for ROS2 and audio)
sudo apt-get install libasound2-dev pkg-config

# ROS2 Installation (if not already installed)
# Follow: https://docs.ros.org/en/humble/Installation.html
```

### ROS2 Setup

You **must** have ROS2 installed and sourced:

```bash
# Install ROS2 (Ubuntu/Debian example - Humble)
sudo apt install ros-humble-desktop

# Source ROS2 in your shell
source /opt/ros/humble/setup.bash

# Add to ~/.bashrc for persistence
echo "source /opt/ros/humble/setup.bash" >> ~/.bashrc
```

## Building

### Quick Build (with helper script)

```bash
./build.sh
```

This script automatically:
- Detects your ROS2 installation
- Sources the ROS2 environment
- Builds the project

### Manual Build

```bash
# Source ROS2 first
source /opt/ros/humble/setup.bash  # or your ROS2 distro

# Build
cargo build --release
```

### Build Error?

If you see:
```
ROS_DISTRO not set: Source your ROS!
```

**Solution:** Source ROS2 before building:
```bash
source /opt/ros/humble/setup.bash
./build.sh
```

## Running

### Using Helper Script (Recommended)

```bash
./run.sh
```

### Manual Run

```bash
# Source ROS2
source /opt/ros/humble/setup.bash

# Set WebSocket URL (optional)
export WS_URL="ws://your-server:8080/v1/audio"

# Run
cargo run --release
```

### Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `WS_URL` | `ws://localhost:8080/v1/audio` | WebSocket server URL |
| `ROS_DISTRO` | Auto-detected | ROS2 distribution (humble/iron/foxy) |

## Configuration

### ROS2 Topic Name

By default, the code publishes to `"audiodata"` topic. If your Unitree GO2 uses a different topic:

1. **Discover the actual topic name:**
   ```bash
   # On the Unitree GO2 or development machine
   python3 scripts/list_topics.py
   ```

2. **Update the code** in `src/utils/websocket_handler.rs:85`:
   ```rust
   let ros_publisher = RosAudioPublisher::new(
       "rust_websocket_audio",
       "audiodata"  // ← Change this to your topic name
   )?;
   ```

3. **Rebuild:**
   ```bash
   ./build.sh
   ```

## Project Structure

```
src/
├── main.rs                       # Entry point, ROS2 + WebSocket initialization
├── ros_audio_msg.rs             # AudioData message structure
├── mp3_decoder.rs               # MP3 → PCM decoding (Symphonia)
├── audio_decoder.rs             # Legacy rodio player (kept for compatibility)
└── utils/
    ├── mod.rs                   # Module declarations
    ├── ros_audio_publisher.rs   # ROS2 publisher (r2r)
    └── websocket_handler.rs     # WebSocket client with audio processing

scripts/
└── list_topics.py               # ROS2 topic discovery tool

build.sh                         # Build helper (sources ROS2 automatically)
run.sh                           # Run helper (sources ROS2 automatically)
```

## How It Works

### 1. WebSocket Connection
```rust
// Connect to WebSocket server
WebSocketAudioClient::new("ws://server:8080/v1/audio")
```

### 2. Receive Audio
The client handles two formats:

**Binary MP3** (most common):
```
WebSocket: Message::Binary(mp3_bytes) → process_binary_audio()
```

**Base64-encoded MP3**:
```
WebSocket: Message::Text("data:audio/mp3;base64,....") → process_base64_audio()
```

### 3. Decode MP3 to PCM
```rust
// Using Symphonia library
let decoded = decode_mp3_to_pcm(mp3_bytes)?;
// → DecodedAudio { samples: Vec<u8>, sample_rate: 16000, channels: 1 }
```

### 4. Publish to ROS2
```rust
// Publish PCM audio to ROS2 topic
ros_publisher.publish_audio(decoded.samples).await?;
// → Unitree GO2 subscribes and plays
```

## Message Format

### Client Sends (Text Question)
```json
{
  "type": "text",
  "texto": "Olá, como você está?",
  "checkpoint_id": 1,
  "estado": "pendente",
  "question_topic": "general",
  "tour_id": 1
}
```

### Server Sends (Three Messages)

**1. Text Response**
```json
{
  "message": "Question processed successfully",
  "data": {
    "texto": "Estou bem, obrigado!"
  }
}
```

**2. Audio Data** (Binary MP3 or Base64)
- Binary: Raw MP3 bytes
- Base64: `"data:audio/mp3;base64,...."`

**3. Done Signal**
```json
{"done": true}
```

## ROS2 Details

### Topic Information
- **Topic Name:** `audiodata` (configurable)
- **Message Type:** `std_msgs/msg/ByteMultiArray`
- **Data Format:** Raw PCM bytes (S16LE: signed 16-bit little-endian)
- **QoS Profile:** Default (reliable, volatile)

### ROS2 Commands

```bash
# List all topics
ros2 topic list

# Check if audiodata topic exists
ros2 topic list | grep audio

# Echo audio data (see what's being published)
ros2 topic echo /audiodata

# Check topic info
ros2 topic info /audiodata

# Monitor publishing rate
ros2 topic hz /audiodata
```

## Testing

### 1. Test ROS2 Connection

```bash
# Terminal 1: Run the client
source /opt/ros/humble/setup.bash
./run.sh

# Terminal 2: Monitor ROS2 topic
source /opt/ros/humble/setup.bash
ros2 topic echo /audiodata
```

You should see audio data being published.

### 2. Test Without WebSocket

Modify `main.rs` to test ROS2 directly:

```rust
// Create test audio
let test_pcm = vec![0u8; 16000 * 2]; // 1 second of silence at 16kHz mono

// Publish
ros_publisher.publish_audio(test_pcm).await?;
```

### 3. Discover ROS2 Topics

```bash
# Run the discovery script
python3 scripts/list_topics.py
```

This will show all available topics and highlight audio-related ones.

## Deployment on Unitree GO2

### 1. Build for Target Architecture

```bash
# Build on x86_64 for the robot
./build.sh --release
```

### 2. Copy to Robot

```bash
# Copy binary
scp target/release/rust_websocket_tts unitree@<robot-ip>:/home/unitree/

# Copy scripts
scp -r scripts unitree@<robot-ip>:/home/unitree/
```

### 3. Run on Robot

```bash
# SSH to robot
ssh unitree@<robot-ip>

# Source ROS2 (usually pre-configured on GO2)
source /opt/ros/humble/setup.bash

# Run
cd /home/unitree
./rust_websocket_tts
```

### 4. Run as Systemd Service

Create `/etc/systemd/system/audio-bridge.service`:

```ini
[Unit]
Description=WebSocket to ROS2 Audio Bridge
After=network.target ros2.service

[Service]
Type=simple
User=unitree
WorkingDirectory=/home/unitree
Environment="WS_URL=ws://10.140.0.11:8080/v1/audio"
ExecStartPre=/bin/bash -c 'source /opt/ros/humble/setup.bash'
ExecStart=/home/unitree/rust_websocket_tts
Restart=always
RestartSec=5

[Install]
WantedBy=multi-user.target
```

Enable and start:
```bash
sudo systemctl daemon-reload
sudo systemctl enable audio-bridge
sudo systemctl start audio-bridge
sudo systemctl status audio-bridge
```

## Troubleshooting

### Build Fails: "ROS_DISTRO not set"

**Problem:** ROS2 environment not sourced.

**Solution:**
```bash
source /opt/ros/humble/setup.bash  # or your ROS2 distro
./build.sh
```

### Runtime Error: "Failed to create ROS2 context"

**Problem:** ROS2 not sourced or not installed.

**Solution:**
```bash
# Check ROS2 installation
ls /opt/ros

# Source ROS2
source /opt/ros/humble/setup.bash

# Run
./run.sh
```

### No Audio on Robot

**Solutions:**
1. **Check topic name:**
   ```bash
   ros2 topic list | grep audio
   python3 scripts/list_topics.py
   ```

2. **Check if robot is listening:**
   ```bash
   ros2 node list
   ros2 topic info /audiodata
   ```

3. **Test publishing manually:**
   ```bash
   ros2 topic pub /audiodata std_msgs/msg/ByteMultiArray "{data: [0, 1, 2, 3]}"
   ```

4. **Check robot volume:**
   ```bash
   amixer  # Check volume levels
   ```

### MP3 Decode Fails

**Problem:** "Failed to decode MP3"

**Possible causes:**
- Audio is not actually MP3 format
- Corrupted audio data
- Unsupported MP3 encoding

**Debug:**
```bash
# Save received audio to file (add to code)
std::fs::write("debug.mp3", &audio_bytes)?;

# Try to play with external tool
ffplay debug.mp3
```

### WebSocket Connection Fails

**Solutions:**
- Check server is running: `curl http://server:8080/health`
- Check firewall: `telnet server 8080`
- Check URL format: `ws://` not `http://`
- Check network connectivity

## Performance

- **Latency:** ~50-100ms from WebSocket → ROS2 publish
- **Memory:** < 50MB typical usage
- **CPU:** < 5% on modern systems (mostly in MP3 decode)
- **Network:** Handles network jitter and reconnections

## Logs

Example successful run:
```
=== Rust WebSocket → ROS2 Audio Bridge ===
🤖 Unitree GO2 Audio Client
Configuration:
  WebSocket URL: ws://localhost:8080/v1/audio
  ROS2 Topic: audiodata
🤖 Initializing ROS2 audio publisher
   Node name: rust_websocket_audio
   Topic: audiodata
✓ ROS2 context created
✓ ROS2 node 'rust_websocket_audio' created
✓ Publisher created on topic 'audiodata'
🎉 ROS2 audio publisher ready!
Connecting to WebSocket server...
Connected successfully!
Received binary audio message: 24567 bytes
🎧 Processing audio: 24567 bytes
📊 Detected audio format: mp3
🎵 Starting MP3 decode: 24567 bytes
🔄 Decoding MP3 packets...
✓ Decoded 48000 samples total
✓ Converted to 96000 PCM bytes
🎉 MP3 decode complete!
✓ Decoded audio: 96000 bytes PCM, 16000Hz, 1 channels
📤 Publishing audio to topic 'audiodata': 96000 bytes
✓ Audio published successfully
🎉 Audio published to ROS2 successfully!
```

## Development

### Run with Debug Logging

```bash
RUST_LOG=debug ./run.sh
```

### Format Code

```bash
cargo fmt
```

### Lint Code

```bash
cargo clippy
```

### Run Tests

```bash
cargo test
```

## TODO / Future Improvements

- [ ] Support more audio formats (WAV, OGG directly)
- [ ] Add audio buffering for smoother playback
- [ ] Add volume control via ROS2 parameters
- [ ] Add audio streaming (chunk-by-chunk) for long audio
- [ ] Add metrics/monitoring (Prometheus)
- [ ] Add configuration file (YAML/TOML)
- [ ] Add retry logic for failed ROS2 publishes

## License

MIT

## Contributing

Contributions welcome! Please:
- Follow Rust best practices
- Add tests for new features
- Update documentation
- Run `cargo fmt` and `cargo clippy`

## Related Projects

- Backend Server: `2025-2B-T12-EC08-BACK/`
- Unitree ROS2: `unitree_ros2` packages
- r2r: Rust ROS2 bindings
- Symphonia: Pure Rust audio decoding

## Support

For issues or questions:
1. Check this README
2. Run `python3 scripts/list_topics.py` to diagnose ROS2 setup
3. Check logs with `RUST_LOG=debug`
4. Open an issue with logs and environment details

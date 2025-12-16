# Quick Start Guide

Get audio streaming to your Unitree Go2 robot in 5 minutes!

## Prerequisites

- Unitree Go2 robot with network access
- SSH access to the robot (username: `unitree`)
- Rust installed (on robot or for cross-compilation)
- FFmpeg libraries installed

## Step 1: Install Dependencies on Robot

SSH into your robot:
```bash
ssh unitree@192.168.123.161
```

Install FFmpeg libraries:
```bash
sudo apt-get update
sudo apt-get install -y libavformat-dev libavcodec-dev libavutil-dev libswresample-dev
```

Install Rust (if not installed):
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

## Step 2: Copy Code to Robot

From your laptop:
```bash
# Copy the entire src directory
scp -r src/ unitree@192.168.123.161:~/webrtc_audio_streamer/
```

## Step 3: Build on Robot

SSH into robot and build:
```bash
ssh unitree@192.168.123.161
cd ~/webrtc_audio_streamer/
cargo build --release
```

Build time: ~5-10 minutes on first build

## Step 4: Run the Service

```bash
./target/release/webrtc_audio_streamer
```

You should see:
```
==============================================
  Audio WebSocket to Unitree Go2 WebRTC
  Direct Audio Track Streaming
==============================================

  WebSocket: ws://0.0.0.0:8080
  Robot IP: 127.0.0.1
  
  ⏳ Waiting for first audio via WebSocket...
==============================================
```

## Step 5: Send Audio from Your Laptop

### Option A: Using Python WebSocket Client

Create `send_audio.py` on your laptop:
```python
import websockets
import asyncio
import sys

async def send_audio(robot_ip, audio_file):
    uri = f"ws://{robot_ip}:8080"
    
    async with websockets.connect(uri) as websocket:
        with open(audio_file, "rb") as f:
            audio_data = f.read()
        
        print(f"Sending {len(audio_data)} bytes to {uri}...")
        await websocket.send(audio_data)
        
        response = await websocket.recv()
        print(f"Robot: {response}")

if __name__ == "__main__":
    if len(sys.argv) < 3:
        print("Usage: python send_audio.py <robot_ip> <audio_file>")
        sys.exit(1)
    
    asyncio.run(send_audio(sys.argv[1], sys.argv[2]))
```

Install websockets:
```bash
pip install websockets
```

Send audio:
```bash
python send_audio.py 192.168.123.161 your_audio.mp3
```

### Option B: Using websocat (command-line tool)

Install websocat:
```bash
# Linux/macOS
cargo install websocat

# Or download from: https://github.com/vi/websocat
```

Send audio:
```bash
cat your_audio.mp3 | websocat ws://192.168.123.161:8080
```

## Step 6: Verify Audio Playback

The robot should:
1. Establish WebRTC connection (logs on robot console)
2. Process and decode the audio
3. Play the audio through its speakers

Check the robot console for:
```
[PIPELINE] *** First audio received! Initializing WebRTC... ***
[WEBRTC] 🟢 Connection state: Connected
[PIPELINE] ✓ Audio processing successful
[PIPELINE] ✓ Message #1 sent to audio player
```

## Troubleshooting

### Issue: "Connection refused"

**Cause**: Service not running on robot

**Solution**: Make sure you started `webrtc_audio_streamer` on the robot

### Issue: "Failed to establish WebRTC connection"

**Cause**: Service not running ON the robot (running on laptop won't work)

**Solution**: 
1. Verify you're SSH'd into the robot: `hostname`
2. Run the service from robot's terminal
3. Use `ROBOT_IP=127.0.0.1` (localhost)

### Issue: "Failed to decode audio"

**Cause**: Corrupted or invalid audio file

**Solution**: 
1. Test the audio file: `ffplay your_audio.mp3`
2. Try a different format: Convert to WAV: `ffmpeg -i input.mp3 output.wav`
3. Ensure complete file is sent (check file size)

### Issue: No sound from robot

**Cause**: Robot volume too low or muted

**Solution**:
1. Check robot's volume settings
2. Try a louder/clearer audio file
3. Verify robot's speakers are working: Test with robot's built-in sounds

## Next Steps

- See `README.md` for detailed documentation
- Customize WebSocket port: `WEBSOCKET_ADDR=0.0.0.0:9090 cargo run`
- Build a client app for continuous audio streaming
- Integrate with your robot control application

## Example Use Cases

### 1. Robot Voice Announcements
```bash
# Generate speech
espeak "The robot is ready" -w announcement.wav

# Send to robot
cat announcement.wav | websocat ws://192.168.123.161:8080
```

### 2. Music Playback
```bash
# Stream music file
python send_audio.py 192.168.123.161 music.mp3
```

### 3. Alert Sounds
```bash
# Generate alert tone
ffmpeg -f lavfi -i "sine=frequency=1000:duration=0.5" -ar 16000 alert.wav

# Send to robot
cat alert.wav | websocat ws://192.168.123.161:8080
```

## Architecture Summary

```
Your Laptop                    Robot
─────────────────────────────────────────────────────
[Audio File]                   [WebSocket Server :8080]
     │                                  │
     │ WebSocket                       │
     │ (Binary/Base64)                 │
     └──────────────────────────>      │
                                        ↓
                              [Audio Decoder (FFmpeg)]
                                        │
                                        ↓
                              [WebRTC Audio Track]
                                        │
                                        ↓
                              [Robot Speakers 🔊]
```

The service runs entirely on the robot, receives audio via WebSocket, and plays it through WebRTC.

## Support

For issues or questions, see the full `README.md` or check:
- `ferramentas/robô/rust_websocket_tts/` - Full implementation reference
- `ferramentas/robô/rust_websocket_tts/go2_webrtc/examples/` - Python examples

# WebSocket Audio Player for Unitree Robot

This tool connects to a WebSocket server, receives base64-encoded MP3 audio, converts it to robot-compatible format, and plays it on the Unitree robot.

## Features

- **WebSocket Client**: Connects to a WebSocket server to receive audio
- **MP3 to WAV Conversion**: Automatically converts MP3 to 16kHz mono WAV (robot-compatible format)
- **Real-time Playback**: Receives and plays audio on the robot
- **Automatic Cleanup**: Removes temporary files after playback
- **Prevents Multiple Playbacks**: Skips new audio if already playing

## Requirements

Install dependencies:

```bash
pip install -r requirements.txt
```

You also need the `unitree_webrtc_connect` library from the `go2_webrtc` package.

## Architecture

```
┌─────────────────────┐         ┌─────────────────────┐         ┌─────────────────┐
│  WebSocket Server   │────────>│  Audio Receiver     │────────>│  Unitree Robot  │
│  (Your Server)      │  MP3    │  (This Script)      │  16kHz  │  (Go2)          │
│                     │  base64 │                     │  WAV    │                 │
└─────────────────────┘         └─────────────────────┘         └─────────────────┘
                                         │
                                         ├─ Receives MP3 base64
                                         ├─ Decodes base64
                                         ├─ Converts to 16kHz mono WAV
                                         ├─ Uploads to robot
                                         └─ Plays audio
```

## Usage

### 1. Start the WebSocket Server (Test Server)

Start the test server that will send audio to the receiver:

```bash
python test_websocket_client.py <audio_file_path> <format> [host] [port]
```

**Arguments:**
- `audio_file_path`: Path to the MP3/WAV file to send
- `format`: Audio format - "mp3" or "wav"
- `host` (optional): Server host (default: 0.0.0.0)
- `port` (optional): Server port (default: 8765)

**Example:**
```bash
python test_websocket_client.py teste.mp3 mp3 0.0.0.0 8765
```

This starts a WebSocket server on `ws://0.0.0.0:8765` that sends the audio file to connecting clients.

### 2. Start the Audio Receiver (Robot Client)

In another terminal, start the receiver that connects to the robot and WebSocket server:

```bash
python websocket_audio_receiver.py <robot_ip> <websocket_url>
```

**Arguments:**
- `robot_ip`: IP address of your Unitree robot
- `websocket_url`: WebSocket server URL to connect to

**Example:**
```bash
python websocket_audio_receiver.py 192.168.123.161 ws://localhost:8765
```

The receiver will:
1. Connect to the robot via WebRTC
2. Connect to the WebSocket server
3. Receive audio data
4. Convert MP3 to 16kHz mono WAV
5. Upload and play on the robot

## Message Format

The WebSocket server should send JSON messages in this format:

```json
{
  "audio": "<base64_encoded_audio_data>",
  "format": "mp3"
}
```

**Fields:**
- `audio`: Base64-encoded audio file (MP3 or WAV)
- `format`: Audio format - "mp3" or "wav" (default: "mp3")

## Python Example (Custom Server)

```python
import asyncio
import json
import base64
import websockets

async def send_audio_to_client(websocket):
    """Send audio to connected client"""
    # Read MP3 file
    with open("audio.mp3", "rb") as f:
        audio_bytes = f.read()
    
    # Encode to base64
    audio_b64 = base64.b64encode(audio_bytes).decode("utf-8")
    
    # Create message
    message = {
        "audio": audio_b64,
        "format": "mp3"
    }
    
    # Send to client
    await websocket.send(json.dumps(message))
    print("Audio sent to client")

async def main():
    async with websockets.serve(send_audio_to_client, "0.0.0.0", 8765):
        print("Server started on ws://0.0.0.0:8765")
        await asyncio.Future()  # Run forever

asyncio.run(main())
```

## Audio Format

### Input Format (Received)
- **MP3** (recommended): Any bitrate, will be converted
- **WAV**: Any sample rate/channels, will be converted

### Output Format (Sent to Robot)
- **Format**: WAV (PCM)
- **Sample Rate**: 16kHz
- **Channels**: Mono (1 channel)
- **Bit Depth**: 16-bit

The script automatically converts any input audio to the robot-compatible format.

## Troubleshooting

### No Sound from Robot
**Most Common Cause**: Robot volume is muted or too low
- Solution: Open Unitree Go2 app → Settings → Volume → Set to ≥50%

### Cannot Connect to Robot
- Verify the robot IP address is correct
- Ensure you're on the same network as the robot
- Check that the robot is powered on and WebRTC is enabled

### Cannot Connect to WebSocket Server
- Verify the WebSocket URL is correct
- Ensure the server is running before starting the receiver
- Check firewall settings

### Audio Already Playing Message
- The receiver skips new audio if currently playing
- Wait for current audio to finish
- This prevents audio overlap

### Conversion Errors
- Ensure `pydub` is installed: `pip install pydub`
- Install FFmpeg: `sudo apt install ffmpeg` (Linux) or download from ffmpeg.org
- Check that the input audio file is valid

## Testing Workflow

1. **Prepare audio file**: 
   ```bash
   # Use any MP3 file
   cp your_audio.mp3 teste.mp3
   ```

2. **Start test server**:
   ```bash
   python test_websocket_client.py teste.mp3 mp3
   ```

3. **Start receiver** (in another terminal):
   ```bash
   python websocket_audio_receiver.py 192.168.123.161 ws://localhost:8765
   ```

4. **Check robot volume** in Unitree app if no sound

## Integration Example

To integrate with your own application:

```python
import asyncio
import json
import base64
import websockets

async def stream_audio_to_robot(audio_file_path):
    """Stream audio file to robot via WebSocket"""
    
    # Start server
    async def handle_client(websocket):
        # Read and encode audio
        with open(audio_file_path, "rb") as f:
            audio_b64 = base64.b64encode(f.read()).decode("utf-8")
        
        # Send to client (robot receiver)
        message = {"audio": audio_b64, "format": "mp3"}
        await websocket.send(json.dumps(message))
        await asyncio.sleep(1)  # Keep connection open
    
    # Start server
    async with websockets.serve(handle_client, "0.0.0.0", 8765):
        print("Audio server started")
        await asyncio.Future()

# Run
asyncio.run(stream_audio_to_robot("my_audio.mp3"))
```

Then run the receiver:
```bash
python websocket_audio_receiver.py 192.168.123.161 ws://your-server-ip:8765
```

## Security Considerations

- The receiver connects to external WebSocket servers
- Validate the WebSocket URL before connecting
- Consider using WSS (WebSocket Secure) for encrypted connections
- Implement authentication on your WebSocket server
- Validate audio data size to prevent memory issues

## License

Same as parent project.

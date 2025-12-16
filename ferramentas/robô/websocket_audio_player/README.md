# WebSocket Audio Player for Unitree Robot

This tool receives base64-encoded audio via WebSocket and plays it on the Unitree robot through WebRTC.

## Features

- WebSocket server that listens for incoming audio data
- Accepts base64-encoded audio in WAV or MP3 format
- Automatically uploads and plays audio on the robot
- Returns status updates to the client (received, success, error)
- Automatic cleanup of temporary files

## Requirements

Install dependencies:

```bash
pip install -r requirements.txt
```

You also need the `unitree_webrtc_connect` library from the `go2_webrtc` package.

## Usage

### 1. Start the WebSocket Audio Server

```bash
python websocket_audio_receiver.py [ROBOT_IP] [WS_PORT]
```

**Arguments:**
- `ROBOT_IP` (optional): IP address of your Unitree robot (default: 192.168.123.161)
- `WS_PORT` (optional): WebSocket server port (default: 8765)

**Example:**
```bash
python websocket_audio_receiver.py 192.168.123.161 8765
```

The server will:
1. Connect to the robot via WebRTC
2. Start a WebSocket server on `0.0.0.0:8765`
3. Wait for incoming audio data

### 2. Send Audio to the Robot

#### Using the Test Client

```bash
python test_websocket_client.py <websocket_url> <audio_file_path> [format]
```

**Arguments:**
- `websocket_url`: WebSocket server URL (e.g., ws://192.168.123.100:8765)
- `audio_file_path`: Path to the audio file to send
- `format` (optional): Audio format - "wav" or "mp3" (default: wav)

**Example:**
```bash
python test_websocket_client.py ws://192.168.123.100:8765 my_audio.wav wav
```

#### Using Your Own Client

Send a JSON message via WebSocket with the following format:

```json
{
  "audio": "<base64_encoded_audio_data>",
  "format": "wav"
}
```

**Python example:**
```python
import asyncio
import json
import base64
import websockets

async def send_audio():
    # Read and encode audio file
    with open("audio.wav", "rb") as f:
        audio_bytes = f.read()
    audio_b64 = base64.b64encode(audio_bytes).decode("utf-8")
    
    # Create message
    message = {
        "audio": audio_b64,
        "format": "wav"
    }
    
    # Send via WebSocket
    async with websockets.connect("ws://192.168.123.100:8765") as ws:
        await ws.send(json.dumps(message))
        
        # Receive responses
        async for response in ws:
            data = json.loads(response)
            print(f"Status: {data['status']}, Message: {data['message']}")
            if data['status'] in ['success', 'error']:
                break

asyncio.run(send_audio())
```

**JavaScript/Node.js example:**
```javascript
const WebSocket = require('ws');
const fs = require('fs');

const ws = new WebSocket('ws://192.168.123.100:8765');

ws.on('open', () => {
    // Read and encode audio file
    const audioBuffer = fs.readFileSync('audio.wav');
    const audioB64 = audioBuffer.toString('base64');
    
    // Send message
    const message = {
        audio: audioB64,
        format: 'wav'
    };
    
    ws.send(JSON.stringify(message));
});

ws.on('message', (data) => {
    const response = JSON.parse(data);
    console.log(`Status: ${response.status}, Message: ${response.message}`);
    
    if (response.status === 'success' || response.status === 'error') {
        ws.close();
    }
});
```

## Response Format

The server sends JSON responses with the following structure:

```json
{
  "status": "received|success|error",
  "message": "Description of the status"
}
```

**Status values:**
- `received`: Audio data received, processing started
- `success`: Audio uploaded and playing on robot
- `error`: An error occurred (see message for details)

## Audio Format Support

- **WAV**: Recommended format, best compatibility
- **MP3**: Also supported

The robot accepts standard audio formats. For best results, use:
- Sample rate: 16kHz or 44.1kHz
- Bit depth: 16-bit
- Channels: Mono or Stereo

## Troubleshooting

### Cannot connect to robot
- Verify the robot IP address is correct
- Ensure you're on the same network as the robot
- Check that the robot's WebRTC service is running

### Audio not playing
- Verify the audio format is supported (WAV or MP3)
- Check the audio file is not corrupted
- Ensure the base64 encoding is correct

### WebSocket connection fails
- Check if the port (8765) is already in use
- Verify firewall settings allow WebSocket connections
- Ensure the server is running before connecting

## Architecture

1. **WebSocket Server**: Listens for incoming connections on specified port
2. **Base64 Decoder**: Decodes incoming audio data
3. **Temporary Storage**: Saves decoded audio to temporary file
4. **WebRTC Connection**: Maintains connection to robot
5. **Audio Hub**: Manages audio upload and playback on robot
6. **Cleanup**: Removes temporary files after playback

## Security Considerations

- The server listens on `0.0.0.0` (all interfaces) by default
- Consider implementing authentication for production use
- Validate audio data size to prevent memory issues
- Use HTTPS/WSS in production environments

## License

Same as parent project.

# Troubleshooting Guide - Audio Not Playing on Robot

## Problem: Audio uploads successfully but doesn't play

Based on your logs, the audio is uploading correctly and the playback command returns successfully (code: 0), but you're not hearing the audio. Here are the possible causes and solutions:

### 1. Robot Volume is Muted or Too Low

**Most Common Issue!** The robot's volume might be set to 0 or very low.

**Solution:** Check and adjust robot volume through the Unitree app or use VUI commands:
- Open the Unitree Go2 mobile app
- Go to Settings → Volume
- Ensure volume is at least 50%

**Alternative:** Use the robot's voice commands if VUI is enabled:
```python
# Add this to check/set volume (if VUI commands support it)
# You may need to use the Unitree app for this
```

### 2. Audio Format Issues

The robot might have specific audio format requirements.

**Current Upload:** Your audio is being converted to WAV with 44.1kHz sample rate (line 127-129 in webrtc_audiohub.py)

**Recommended Audio Specifications:**
- **Format:** WAV (PCM)
- **Sample Rate:** 16kHz or 44.1kHz  
- **Bit Depth:** 16-bit
- **Channels:** Mono or Stereo
- **Max File Size:** ~10MB recommended

**Test with known-good audio:**
```bash
# Create a simple test tone with correct specs
ffmpeg -f lavfi -i "sine=frequency=440:duration=2" -ar 16000 -ac 1 test_tone.wav

# Send it
python test_websocket_client.py ws://192.168.123.161:8765 test_tone.wav wav
```

### 3. Robot Audio Service Not Running

The audio service on the robot might not be running properly.

**Solution:** Restart the robot or check if the audio service is active:
```bash
# If you have SSH access to the robot
ssh unitree@<robot-ip>
sudo systemctl status unitree-audio  # or whatever the service is called
```

### 4. Competing Audio Processes

Another process might be using the audio output.

**Check:** See if any other audio is playing or if companion mode/obstacle avoidance is active (these might have their own audio feedback).

**Solution:** Disable other features temporarily to test.

### 5. Timing Issue with Playback

The robot might need more time after upload before playback.

**Already Fixed:** The updated code now includes:
- 0.5s wait after upload (line 82)
- 0.3s wait after playback command (line 107)

**If still not working, try increasing these delays:**
```python
# In websocket_audio_receiver.py, line 82
await asyncio.sleep(1.0)  # Increase from 0.5 to 1.0

# Line 107
await asyncio.sleep(0.5)  # Increase from 0.3 to 0.5
```

### 6. Audio File Corruption

The base64 encoding/decoding might be corrupting the audio.

**Test:**
1. Save the decoded audio locally before sending:
```python
# Add this in play_base64_audio() after line 66
with open("/tmp/debug_audio.wav", "wb") as f:
    f.write(audio_bytes)
print("Debug audio saved to /tmp/debug_audio.wav - test this file locally!")
```

2. Play the debug file on your computer to verify it's valid
3. If it plays on your computer but not on the robot, the issue is robot-specific

### 7. Check Robot Logs

Access robot logs to see if there are any audio-related errors:

```bash
# SSH into robot
ssh unitree@<robot-ip>

# Check system logs
journalctl -u unitree-audio -f  # Real-time audio service logs
dmesg | grep -i audio  # System audio messages
```

### 8. Test with Built-in Audio

Verify the robot's speaker is working:

**Using the mobile app:**
- Try playing the built-in audio files ("Bem vindos", "Latido", etc.)
- If these don't play either, the speaker hardware or service has an issue

**Using the API:**
```python
# Test playing existing audio
uuid = "ba80e033-ca69-404d-b2d4-84c42aea4d67"  # "Bem vindos" from your logs
await audio_hub.play_by_uuid(uuid)
```

### 9. Monitor Playback State

Subscribe to the playback state topic to see what's happening:

```python
# Add this to your code to monitor playback
async def monitor_playback_state(connection):
    def callback(data):
        print(f"Playback state: {data}")
    
    await connection.datachannel.pub_sub.subscribe(
        "rt/audiohub/player/state",
        callback
    )

# Call this after connecting
await monitor_playback_state(self.webrtc_conn)
```

### 10. Network/Bluetooth Interference

If using Bluetooth or WiFi audio output, interference might cause issues.

**Solution:**
- Ensure stable network connection
- Try moving closer to the robot
- Disable Bluetooth if not needed

## Debugging Steps

1. **First, verify speaker works:**
   - Use Unitree app to play built-in sounds
   - Check volume is > 50%

2. **Test with known-good audio:**
   - Use example WAV files from `go2_webrtc/examples/go2/audio/mp3_player/`
   - Try `dog-barking.wav` which is known to work

3. **Enable verbose logging:**
```python
# In websocket_audio_receiver.py, line 16
logging.basicConfig(level=logging.DEBUG)  # Change from INFO to DEBUG
```

4. **Check audio list:**
```python
# Verify your audio is actually uploaded
response = await audio_hub.get_audio_list()
print(json.dumps(response, indent=2))
```

5. **Try playing from robot's existing files:**
```python
# From your logs, these files exist:
# - "Bem vindos": ba80e033-ca69-404d-b2d4-84c42aea4d67
# - "Latido": 4c5040dc-2923-40d0-bbd5-6245cfd4efa7

await audio_hub.play_by_uuid("4c5040dc-2923-40d0-bbd5-6245cfd4efa7")
# If this doesn't play either, the issue is with the robot's audio output
```

## Quick Diagnostic Script

Create `test_audio_playback.py`:

```python
import asyncio
from unitree_webrtc_connect.webrtc_driver import UnitreeWebRTCConnection, WebRTCConnectionMethod
from unitree_webrtc_connect.webrtc_audiohub import WebRTCAudioHub
import logging

logging.basicConfig(level=logging.INFO)
logger = logging.getLogger(__name__)

async def test():
    # Connect
    conn = UnitreeWebRTCConnection(WebRTCConnectionMethod.LocalSTA, ip="192.168.123.161")
    await conn.connect()
    
    audio_hub = WebRTCAudioHub(conn, logger)
    
    # Get audio list
    response = await audio_hub.get_audio_list()
    print("Audio list:", response)
    
    # Try playing existing audio (use UUID from your robot)
    uuid = "ba80e033-ca69-404d-b2d4-84c42aea4d67"  # "Bem vindos"
    print(f"Playing UUID: {uuid}")
    await audio_hub.play_by_uuid(uuid)
    
    # Wait to hear it
    await asyncio.sleep(5)
    print("Done")

asyncio.run(test())
```

Run this and check if you hear "Bem vindos" playing. If not, the issue is with the robot's audio output system itself.

## Most Likely Solution

**Check the robot's volume settings in the Unitree app!** This is the most common cause of "audio not playing" issues.

import logging
import asyncio
import json
import base64
import tempfile
import os
import sys
import shutil
import hashlib
import time
import websockets
from pydub import AudioSegment
from unitree_webrtc_connect.webrtc_driver import (
    UnitreeWebRTCConnection,
    WebRTCConnectionMethod,
)
from unitree_webrtc_connect.webrtc_audiohub import WebRTCAudioHub

# Configure logging
logging.basicConfig(level=logging.INFO, format='%(asctime)s - %(levelname)s - %(message)s')
logger = logging.getLogger(__name__)

# ==================================================================================
# ⚙️ CONFIGURATION
# ==================================================================================

# ✅ SET THIS TO TRUE (Since it stopped playing, we know we need this!)
SEND_PLAY_COMMAND = True  

# Ignore duplicate audio messages received within this many seconds
IGNORE_DUPLICATES_SECONDS = 30.0 

# ==================================================================================

def check_ffmpeg():
    """Check if ffmpeg is installed"""
    if not shutil.which("ffmpeg") or not shutil.which("ffprobe"):
        logger.error("❌ FFMPEG NOT FOUND. Please install it (sudo apt install ffmpeg)")
        return False
    return True

class WebSocketAudioStreamer:
    def __init__(self, robot_ip: str, websocket_url: str, retry_interval: float = 5.0):
        self.robot_ip = robot_ip
        self.websocket_url = websocket_url
        self.retry_interval = retry_interval
        self.webrtc_conn = None
        self.audio_hub = None
        self.temp_dir = tempfile.mkdtemp()
        self.should_stop = False
        
        self.audio_queue = asyncio.Queue()
        self.upload_lock = asyncio.Lock()
        
        # State to prevent loops/duplicates
        self.last_played_hash = None
        self.last_played_time = 0
        
        logger.info(f"Temporary directory created: {self.temp_dir}")

    async def initialize_robot_connection(self):
        logger.info(f"Connecting to robot at {self.robot_ip}")
        self.webrtc_conn = UnitreeWebRTCConnection(
            WebRTCConnectionMethod.LocalSTA, ip=self.robot_ip
        )
        await self.webrtc_conn.connect()
        logger.info("✅ WebRTC connection established")
        self.audio_hub = WebRTCAudioHub(self.webrtc_conn, logger)

    async def process_audio_queue(self):
        """Worker that processes audio one by one"""
        logger.info("🎶 Audio processing worker started")
        while not self.should_stop:
            try:
                audio_data_b64, audio_format = await self.audio_queue.get()
                
                # --- DEDUPLICATION CHECK ---
                audio_hash = hashlib.md5(audio_data_b64.encode()).hexdigest()
                current_time = time.time()
                
                if (audio_hash == self.last_played_hash and 
                    (current_time - self.last_played_time) < IGNORE_DUPLICATES_SECONDS):
                    logger.warning("⚠️  Duplicate received (ignoring)")
                    self.audio_queue.task_done()
                    continue

                self.last_played_hash = audio_hash
                self.last_played_time = current_time
                
                # Use lock to ensure strictly sequential processing
                async with self.upload_lock:
                    await self.convert_and_play_audio(audio_data_b64, audio_format)
                
                self.audio_queue.task_done()
                
            except asyncio.CancelledError:
                break
            except Exception as e:
                logger.error(f"Worker error: {e}")

    async def convert_and_play_audio(self, audio_data_b64: str, audio_format: str):
        try:
            audio_bytes = base64.b64decode(audio_data_b64)
            timestamp = int(time.time() * 1000)
            
            # File Paths
            input_path = os.path.join(self.temp_dir, f"in_{timestamp}.{audio_format}")
            output_path = os.path.join(self.temp_dir, f"out_{timestamp}.wav")

            # Write Input
            with open(input_path, "wb") as f:
                f.write(audio_bytes)
            
            # Convert to WAV (16kHz, Mono)
            try:
                audio = AudioSegment.from_file(input_path, format=audio_format)
                audio = audio.set_frame_rate(16000).set_channels(1).set_sample_width(2)
                audio.export(output_path, format="wav")
            except Exception as e:
                logger.error(f"❌ Conversion failed: {e}")
                return

            # Upload
            logger.info(f"📤 Uploading {len(audio_bytes)} bytes to robot...")
            await self.audio_hub.upload_audio_file(output_path)
            
            # Wait for filesystem to settle
            await asyncio.sleep(1.0)

            # Explicit Play Command
            if SEND_PLAY_COMMAND:
                logger.info("🔍 Searching for file on robot...")
                response = await self.audio_hub.get_audio_list()
                
                if response:
                    # Parse the nested JSON response
                    try:
                        if isinstance(response, str):
                            data_obj = json.loads(response)
                        else:
                            data_obj = response
                            
                        inner_data = data_obj.get("data", {})
                        if isinstance(inner_data, str):
                            inner_data = json.loads(inner_data) # sometimes it's double encoded
                            
                        audio_list = inner_data.get("audio_list", [])
                        
                        # Find our file
                        target_name = os.path.splitext(os.path.basename(output_path))[0]
                        
                        match = next((a for a in audio_list if a["CUSTOM_NAME"] == target_name), None)
                        if match:
                            uuid = match["UNIQUE_ID"]
                            logger.info(f"▶️  PLAYING (UUID: {uuid})")
                            await self.audio_hub.play_by_uuid(uuid)
                        else:
                            logger.warning(f"⚠️ Could not find '{target_name}' in robot list")
                    except Exception as json_err:
                        logger.error(f"❌ Error parsing robot audio list: {json_err}")
            
            # Cleanup
            await asyncio.sleep(1.0)
            if os.path.exists(input_path): os.remove(input_path)
            if os.path.exists(output_path): os.remove(output_path)

        except Exception as e:
            logger.error(f"Playback sequence error: {e}")

    async def connect_and_listen(self):
        while not self.should_stop:
            try:
                logger.info(f"Connecting to {self.websocket_url}...")
                async with websockets.connect(self.websocket_url) as websocket:
                    logger.info("✅ Connected to Server")
                    
                    async for message in websocket:
                        try:
                            data = json.loads(message)
                            if "audio" in data:
                                await self.audio_queue.put((data["audio"], data.get("format", "mp3")))
                        except Exception:
                            pass
            except Exception:
                logger.warning(f"Connection lost. Retrying in {self.retry_interval}s...")
                await asyncio.sleep(self.retry_interval)

    async def run(self):
        try:
            check_ffmpeg()
            await self.initialize_robot_connection()
            worker = asyncio.create_task(self.process_audio_queue())
            await self.connect_and_listen()
            worker.cancel()
        except KeyboardInterrupt:
            logger.info("Stopping...")
        finally:
            shutil.rmtree(self.temp_dir, ignore_errors=True)

async def main():
    if len(sys.argv) < 3:
        print("Usage: python receiver.py <robot_ip> <websocket_url>")
        sys.exit(1)
    await WebSocketAudioStreamer(sys.argv[1], sys.argv[2]).run()

if __name__ == "__main__":
    asyncio.run(main())
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

# --- CONFIGURATION ---
# Set to True if your robot DOES NOT auto-play uploaded files.
# Set to False if your robot plays automatically (prevents double play).
SEND_PLAY_COMMAND = False  

# Time to ignore duplicate files (in seconds)
IGNORE_DUPLICATES_SECONDS = 30.0 
# ---------------------

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
        
        # LOCK: Ensures we never ask the WebRTC lib to upload two things at once
        self.upload_lock = asyncio.Lock()
        
        # State for deduplication
        self.last_hash = None
        self.last_time = 0
        
        logger.info(f"Temp dir: {self.temp_dir}")

    async def initialize_robot_connection(self):
        logger.info(f"Connecting to robot at {self.robot_ip}")
        self.webrtc_conn = UnitreeWebRTCConnection(
            WebRTCConnectionMethod.LocalSTA, ip=self.robot_ip
        )
        await self.webrtc_conn.connect()
        logger.info("✅ WebRTC Connected")
        self.audio_hub = WebRTCAudioHub(self.webrtc_conn, logger)

    async def process_audio_queue(self):
        logger.info("🎶 Worker started")
        while not self.should_stop:
            try:
                # Get next item
                audio_b64, audio_fmt = await self.audio_queue.get()
                
                # Check Hash (Deduplication)
                current_hash = hashlib.md5(audio_b64.encode()).hexdigest()
                now = time.time()
                
                if (current_hash == self.last_hash and 
                    (now - self.last_time) < IGNORE_DUPLICATES_SECONDS):
                    logger.warning("⚠️ Duplicate ignored (already played recently)")
                    self.audio_queue.task_done()
                    continue

                # It is a valid new file
                self.last_hash = current_hash
                self.last_time = now
                
                # Use the Lock to prevent overlapping WebRTC calls
                async with self.upload_lock:
                    await self.convert_and_play(audio_b64, audio_fmt)
                
                self.audio_queue.task_done()
                
            except asyncio.CancelledError:
                break
            except Exception as e:
                logger.error(f"Worker Error: {e}")

    async def convert_and_play(self, audio_b64, fmt):
        try:
            # Decode
            raw = base64.b64decode(audio_b64)
            ts = int(time.time() * 1000)
            in_path = os.path.join(self.temp_dir, f"in_{ts}.{fmt}")
            out_path = os.path.join(self.temp_dir, f"out_{ts}.wav")

            with open(in_path, "wb") as f:
                f.write(raw)
            
            # Convert
            try:
                audio = AudioSegment.from_file(in_path, format=fmt)
                audio = audio.set_frame_rate(16000).set_channels(1).set_sample_width(2)
                audio.export(out_path, format="wav")
            except Exception as e:
                logger.error(f"Convert failed: {e}")
                return

            # --- WEBRTC UPLOAD SECTION ---
            logger.info(f"📤 Uploading {len(raw)} bytes...")
            
            # 1. Upload
            await self.audio_hub.upload_audio_file(out_path)
            
            # 2. Wait strictly to let the WebRTC channel clear
            await asyncio.sleep(1.0) 

            # 3. Only send play command if configured
            if SEND_PLAY_COMMAND:
                logger.info("▶️ Sending manual PLAY command...")
                resp = await self.audio_hub.get_audio_list()
                if resp:
                    # Parse messy JSON inside JSON
                    inner = json.loads(resp.get("data", {}).get("data", "{}"))
                    lst = inner.get("audio_list", [])
                    
                    target = os.path.splitext(os.path.basename(out_path))[0]
                    found = next((x for x in lst if x["CUSTOM_NAME"] == target), None)
                    
                    if found:
                        await self.audio_hub.play_by_uuid(found["UNIQUE_ID"])
            else:
                logger.info("✅ Upload done (Auto-play assumed)")

            # Cleanup
            if os.path.exists(in_path): os.remove(in_path)
            if os.path.exists(out_path): os.remove(out_path)

        except Exception as e:
            logger.error(f"Processing error: {e}")

    async def connect_and_listen(self):
        while not self.should_stop:
            try:
                logger.info(f"Connecting to Server {self.websocket_url}...")
                async with websockets.connect(self.websocket_url) as ws:
                    logger.info("✅ Connected")
                    async for msg in ws:
                        data = json.loads(msg)
                        if "audio" in data:
                            # Just put in queue, let the locked worker handle it
                            await self.audio_queue.put((data["audio"], data.get("format", "mp3")))
            except Exception as e:
                logger.warning(f"Connection error: {e}")
                await asyncio.sleep(self.retry_interval)

    async def run(self):
        # Check ffmpeg first
        if not shutil.which("ffmpeg"):
            logger.error("❌ FFmpeg missing!")
            return

        await self.initialize_robot_connection()
        
        # Start the Locked Worker
        task = asyncio.create_task(self.process_audio_queue())
        
        await self.connect_and_listen()
        task.cancel()

async def main():
    if len(sys.argv) < 3:
        print("Usage: python receiver.py <ROBOT_IP> <WS_URL>")
        sys.exit(1)
    
    streamer = WebSocketAudioStreamer(sys.argv[1], sys.argv[2])
    await streamer.run()

if __name__ == "__main__":
    asyncio.run(main())
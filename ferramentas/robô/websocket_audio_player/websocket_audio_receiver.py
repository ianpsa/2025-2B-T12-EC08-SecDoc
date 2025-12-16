import logging
import asyncio
import json
import base64
import tempfile
import os
import sys
import shutil
import time
import websockets
from pydub import AudioSegment
from unitree_webrtc_connect.webrtc_driver import (
    UnitreeWebRTCConnection,
    WebRTCConnectionMethod,
)
from unitree_webrtc_connect.webrtc_audiohub import WebRTCAudioHub

logging.basicConfig(level=logging.INFO, format='%(asctime)s - %(message)s', datefmt='%H:%M:%S')
logger = logging.getLogger(__name__)

# --- CONFIGURATION ---
SEND_PLAY_COMMAND = True  
STREAM_CHUNKS = True      # True: Wait for chunk A to finish before playing chunk B (No overlap)
# ---------------------

class WebSocketAudioStreamer:
    def __init__(self, robot_ip: str, websocket_url: str):
        self.robot_ip = robot_ip
        self.websocket_url = websocket_url
        self.retry_interval = 5.0
        self.webrtc_conn = None
        self.audio_hub = None
        self.temp_dir = tempfile.mkdtemp()
        self.audio_queue = asyncio.Queue()
        self.upload_lock = asyncio.Lock()
        self.chunk_counter = 0  # Simple counter for filenames
        
        logger.info(f"Temp dir: {self.temp_dir}")

    async def initialize_robot_connection(self):
        logger.info(f"Connecting to robot at {self.robot_ip}")
        self.webrtc_conn = UnitreeWebRTCConnection(
            WebRTCConnectionMethod.LocalSTA, ip=self.robot_ip
        )
        await self.webrtc_conn.connect()
        logger.info("✅ WebRTC Connected")
        self.audio_hub = WebRTCAudioHub(self.webrtc_conn, logger)

    async def find_uuid_fast(self, target_name_base, retries=10):
        """
        Fast polling to find the file UUID.
        Checks every 0.2s. Total wait = 0.2 * 10 = 2 seconds max.
        """
        for _ in range(retries):
            try:
                response = await self.audio_hub.get_audio_list()
                if response:
                    # Parse inner JSON if needed
                    data = response if isinstance(response, dict) else json.loads(response)
                    inner = data.get("data", {})
                    if isinstance(inner, str): inner = json.loads(inner)
                    
                    audio_list = inner.get("audio_list", [])
                    
                    # FUZZY MATCH: Check if target name is contained in the custom name
                    # This fixes issues where robot adds/removes extensions
                    for audio in audio_list:
                        if target_name_base in audio["CUSTOM_NAME"]:
                            return audio["UNIQUE_ID"]
                            
            except Exception:
                pass
            
            await asyncio.sleep(0.2) # Fast retry
        return None

    async def process_audio_queue(self):
        logger.info("🎶 Streamer Ready")
        while True:
            try:
                # 1. Get next chunk
                audio_b64, audio_fmt = await self.audio_queue.get()
                
                # 2. Lock prevents simultaneous uploads
                async with self.upload_lock:
                    await self.handle_chunk(audio_b64, audio_fmt)
                
                self.audio_queue.task_done()
            except Exception as e:
                logger.error(f"Loop error: {e}")

    async def handle_chunk(self, audio_b64, fmt):
        try:
            self.chunk_counter += 1
            # Simple filename: chk_1, chk_2, etc.
            base_name = f"chk_{self.chunk_counter}"
            
            # Paths
            in_path = os.path.join(self.temp_dir, f"{base_name}.{fmt}")
            out_path = os.path.join(self.temp_dir, f"{base_name}.wav")

            # Decode
            with open(in_path, "wb") as f:
                f.write(base64.b64decode(audio_b64))
            
            # Convert & Calculate Duration
            duration_sec = 0
            try:
                audio = AudioSegment.from_file(in_path, format=fmt)
                duration_sec = len(audio) / 1000.0
                # Export as 16k Mono
                audio.set_frame_rate(16000).set_channels(1).set_sample_width(2).export(
                    out_path, format="wav"
                )
            except Exception:
                logger.error("Convert failed")
                return

            # Upload
            # logger.info(f"📤 Uploading {base_name} ({duration_sec:.2f}s)")
            await self.audio_hub.upload_audio_file(out_path)
            
            if SEND_PLAY_COMMAND:
                # Find UUID (Fast polling)
                uuid = await self.find_uuid_fast(base_name)
                
                if uuid:
                    logger.info(f"▶️  Playing {base_name} ({duration_sec:.1f}s)")
                    await self.audio_hub.play_by_uuid(uuid)
                    
                    # STREAMING: Wait for audio to finish before next chunk
                    if STREAM_CHUNKS and duration_sec > 0:
                        await asyncio.sleep(duration_sec - 0.1) # Small overlap adjustment
                else:
                    logger.warning(f"⚠️ Skipped {base_name} (Not found in list)")

            # Cleanup
            if os.path.exists(in_path): os.remove(in_path)
            if os.path.exists(out_path): os.remove(out_path)

        except Exception as e:
            logger.error(f"Chunk error: {e}")

    async def run(self):
        if not shutil.which("ffmpeg"):
            logger.error("❌ FFmpeg required")
            return

        await self.initialize_robot_connection()
        asyncio.create_task(self.process_audio_queue())

        # WebSocket Loop
        while True:
            try:
                logger.info(f"Connecting to {self.websocket_url}...")
                async with websockets.connect(self.websocket_url) as ws:
                    logger.info("✅ Connected")
                    async for msg in ws:
                        try:
                            d = json.loads(msg)
                            if "audio" in d:
                                await self.audio_queue.put((d["audio"], d.get("format", "mp3")))
                        except: pass
            except:
                await asyncio.sleep(self.retry_interval)

if __name__ == "__main__":
    if len(sys.argv) < 3:
        print("Usage: python receiver.py <IP> <WS_URL>")
        sys.exit(1)
    
    try:
        asyncio.run(WebSocketAudioStreamer(sys.argv[1], sys.argv[2]).run())
    except KeyboardInterrupt:
        pass
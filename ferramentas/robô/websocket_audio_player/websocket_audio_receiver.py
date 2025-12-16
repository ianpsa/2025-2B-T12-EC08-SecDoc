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

# Set to True to play audio manually (Required if robot doesn't auto-play)
SEND_PLAY_COMMAND = True  

# If true, we wait for the audio to finish playing before processing the next chunk.
# This creates a "streaming" effect without cutting off the previous sentence.
WAIT_FOR_AUDIO_DURATION = True

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
        self.upload_lock = asyncio.Lock() # Prevents simultaneous uploads
        
        logger.info(f"Temporary directory created: {self.temp_dir}")

    async def initialize_robot_connection(self):
        logger.info(f"Connecting to robot at {self.robot_ip}")
        self.webrtc_conn = UnitreeWebRTCConnection(
            WebRTCConnectionMethod.LocalSTA, ip=self.robot_ip
        )
        await self.webrtc_conn.connect()
        logger.info("✅ WebRTC connection established")
        self.audio_hub = WebRTCAudioHub(self.webrtc_conn, logger)

    async def get_uuid_with_retry(self, target_filename, retries=5, delay=1.0):
        """Polls the robot multiple times until the file appears in the list."""
        target_name = os.path.splitext(os.path.basename(target_filename))[0]
        
        for i in range(retries):
            try:
                # logger.info(f"🔎 Scanning for file '{target_name}' (Attempt {i+1}/{retries})...")
                response = await self.audio_hub.get_audio_list()
                
                if response:
                    # Handle double-encoded JSON if necessary
                    if isinstance(response, str):
                        data_obj = json.loads(response)
                    else:
                        data_obj = response
                        
                    inner_data = data_obj.get("data", {})
                    if isinstance(inner_data, str):
                        inner_data = json.loads(inner_data)
                        
                    audio_list = inner_data.get("audio_list", [])
                    
                    # Find match
                    match = next((a for a in audio_list if a["CUSTOM_NAME"] == target_name), None)
                    if match:
                        return match["UNIQUE_ID"]
            except Exception as e:
                logger.warning(f"Error checking list: {e}")
            
            # Wait before next check
            await asyncio.sleep(delay)
            
        return None

    async def process_audio_queue(self):
        """Worker that processes audio sequentially"""
        logger.info("🎶 Audio processing worker started")
        
        while not self.should_stop:
            try:
                # 1. Get audio data from queue
                audio_data_b64, audio_format = await self.audio_queue.get()
                
                # 2. Process strictly one by one (Sequential Playback)
                async with self.upload_lock:
                    await self.convert_upload_and_play(audio_data_b64, audio_format)
                
                self.audio_queue.task_done()
                
            except asyncio.CancelledError:
                break
            except Exception as e:
                logger.error(f"Worker error: {e}")

    async def convert_upload_and_play(self, audio_data_b64: str, audio_format: str):
        try:
            # --- 1. Decode & Save ---
            audio_bytes = base64.b64decode(audio_data_b64)
            timestamp = int(time.time() * 1000)
            
            input_path = os.path.join(self.temp_dir, f"in_{timestamp}.{audio_format}")
            output_path = os.path.join(self.temp_dir, f"out_{timestamp}.wav")

            with open(input_path, "wb") as f:
                f.write(audio_bytes)
            
            # --- 2. Convert & Get Duration ---
            audio_duration_sec = 0
            try:
                audio = AudioSegment.from_file(input_path, format=audio_format)
                
                # Store duration so we know how long to wait
                audio_duration_sec = len(audio) / 1000.0
                
                # Convert to Unitree Format (16kHz, Mono, 16-bit)
                audio = audio.set_frame_rate(16000).set_channels(1).set_sample_width(2)
                audio.export(output_path, format="wav")
            except Exception as e:
                logger.error(f"❌ Conversion failed: {e}")
                return

            # --- 3. Upload ---
            # logger.info(f"📤 Uploading chunk ({audio_duration_sec:.2f}s)...")
            await self.audio_hub.upload_audio_file(output_path)
            
            # --- 4. Find File on Robot (With Retry) ---
            if SEND_PLAY_COMMAND:
                # Short initial wait for FS to flush
                await asyncio.sleep(0.5) 
                
                uuid = await self.get_uuid_with_retry(output_path)
                
                if uuid:
                    logger.info(f"▶️  Playing Chunk ({audio_duration_sec:.1f}s)")
                    await self.audio_hub.play_by_uuid(uuid)
                    
                    # --- 5. STREAMING LOGIC ---
                    if WAIT_FOR_AUDIO_DURATION and audio_duration_sec > 0:
                        # Wait for the audio to actually finish playing before 
                        # letting the next chunk start. This prevents cuts.
                        # We subtract a tiny bit (0.2s) to make the transition tighter.
                        wait_time = max(0, audio_duration_sec - 0.2)
                        await asyncio.sleep(wait_time)
                else:
                    logger.error(f"❌ Timed out: Could not find file '{os.path.basename(output_path)}' on robot")

            # --- 6. Cleanup ---
            # Clean up immediately to keep disk usage low
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
                                # Put in queue. The Worker will process it sequentially.
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
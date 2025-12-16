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
# Set to False if your robot automatically plays audio upon upload
SEND_PLAY_COMMAND = True 

# Ignore duplicate audio messages received within this many seconds
DEDUPLICATION_WINDOW = 2.0 
# ---------------------

def check_ffmpeg():
    """Check if ffmpeg is installed"""
    ffmpeg_path = shutil.which("ffmpeg")
    ffprobe_path = shutil.which("ffprobe")

    if not ffmpeg_path or not ffprobe_path:
        logger.error("=" * 70)
        logger.error("❌ FFMPEG NOT FOUND")
        logger.error("=" * 70)
        logger.error("FFmpeg is required to convert MP3 audio to WAV format.")
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
        
        # Cache for deduplication: {hash: timestamp}
        self.processed_hashes = {}
        
        logger.info(f"Temporary directory created: {self.temp_dir}")

    async def initialize_robot_connection(self):
        """Establish WebRTC connection with the robot"""
        try:
            logger.info(f"Connecting to robot at {self.robot_ip}")
            self.webrtc_conn = UnitreeWebRTCConnection(
                WebRTCConnectionMethod.LocalSTA, ip=self.robot_ip
            )
            await self.webrtc_conn.connect()
            logger.info("WebRTC connection established")

            self.audio_hub = WebRTCAudioHub(self.webrtc_conn, logger)
            logger.info("Audio hub initialized")
        except Exception as e:
            logger.error(f"Failed to connect to robot: {e}")
            raise

    def is_duplicate(self, audio_bytes: bytes) -> bool:
        """Check if this specific audio data was processed recently"""
        audio_hash = hashlib.md5(audio_bytes).hexdigest()
        current_time = time.time()
        
        # specific deduplication logic
        if audio_hash in self.processed_hashes:
            last_time = self.processed_hashes[audio_hash]
            if current_time - last_time < DEDUPLICATION_WINDOW:
                logger.warning(f"⚠️  Duplicate audio ignored (received {current_time - last_time:.2f}s ago)")
                return True
        
        # Update cache and cleanup old entries
        self.processed_hashes[audio_hash] = current_time
        
        # Optional: cleanup dict if it gets too big
        if len(self.processed_hashes) > 100:
            cutoff = current_time - DEDUPLICATION_WINDOW
            self.processed_hashes = {k: v for k, v in self.processed_hashes.items() if v > cutoff}
            
        return False

    async def process_audio_queue(self):
        """Background task to process audio files from the queue sequentially"""
        logger.info("🎶 Audio processing worker started")
        while not self.should_stop:
            try:
                audio_data_b64, audio_format = await self.audio_queue.get()
                
                # Decode and check for duplicates BEFORE converting
                try:
                    audio_bytes = base64.b64decode(audio_data_b64)
                    if not self.is_duplicate(audio_bytes):
                        await self.convert_and_play_audio(audio_bytes, audio_format)
                except Exception as e:
                    logger.error(f"Error preparing audio: {e}")

                self.audio_queue.task_done()
                
            except asyncio.CancelledError:
                break
            except Exception as e:
                logger.error(f"Error in audio processing worker: {e}")

    async def convert_and_play_audio(
        self, audio_bytes: bytes, audio_format: str = "mp3"
    ):
        """Convert and play decoded audio bytes"""
        try:
            # Generate unique filenames using high-res timestamp
            timestamp = int(time.time() * 1000)
            input_filename = f"input_{timestamp}.{audio_format}"
            input_filepath = os.path.join(self.temp_dir, input_filename)

            # Write input file
            with open(input_filepath, "wb") as f:
                f.write(audio_bytes)
            
            output_filename = f"audio_{timestamp}.wav"
            output_filepath = os.path.join(self.temp_dir, output_filename)

            # Conversion Logic
            try:
                if audio_format.lower() == "mp3":
                    audio = AudioSegment.from_mp3(input_filepath)
                else:
                    audio = AudioSegment.from_wav(input_filepath)

                audio = audio.set_frame_rate(16000).set_channels(1).set_sample_width(2)
                audio.export(output_filepath, format="wav", parameters=["-ar", "16000", "-ac", "1"])

            except Exception as e:
                logger.error(f"❌ Conversion failed: {e}")
                if audio_format.lower() == "wav":
                    output_filepath = input_filepath
                    output_filename = input_filename
                else:
                    return 

            # Upload
            logger.info("📤 Uploading audio to robot...")
            await self.audio_hub.upload_audio_file(output_filepath)
            
            # If explicit play command is disabled, stop here
            if not SEND_PLAY_COMMAND:
                logger.info("✅ Upload complete (Auto-play assumed)")
                return

            # Wait for upload to register
            await asyncio.sleep(0.5)

            # Find and Play
            response = await self.audio_hub.get_audio_list()
            if response and isinstance(response, dict):
                data_str = response.get("data", {}).get("data", "{}")
                audio_list = json.loads(data_str).get("audio_list", [])

                target_name = os.path.splitext(output_filename)[0]
                existing_audio = next(
                    (a for a in audio_list if a["CUSTOM_NAME"] == target_name), None
                )

                if existing_audio:
                    uuid = existing_audio["UNIQUE_ID"]
                    logger.info(f"▶️  Sending Play Command (UUID: {uuid})")
                    await self.audio_hub.play_by_uuid(uuid)
                else:
                    logger.error(f"Could not find file '{target_name}' on robot")

            # Cleanup
            await asyncio.sleep(1.0) # Wait a bit before deleting
            try:
                if os.path.exists(input_filepath): os.remove(input_filepath)
                if os.path.exists(output_filepath): os.remove(output_filepath)
            except Exception:
                pass

        except Exception as e:
            logger.error(f"Error executing play sequence: {e}")

    async def connect_and_listen(self):
        attempt = 0
        while not self.should_stop:
            attempt += 1
            try:
                logger.info(f"Connecting to {self.websocket_url}...")
                async with websockets.connect(self.websocket_url) as websocket:
                    logger.info("✅ Connected")
                    attempt = 0 

                    async for message in websocket:
                        try:
                            data = json.loads(message)
                            audio_b64 = data.get("audio")
                            audio_format = data.get("format", "mp3")

                            if audio_b64:
                                # Push to queue
                                await self.audio_queue.put((audio_b64, audio_format))
                                
                        except Exception as e:
                            logger.error(f"Message error: {e}")

            except Exception as e:
                logger.warning(f"Connection lost: {e}")
                
            if not self.should_stop:
                await asyncio.sleep(self.retry_interval)

    async def run(self):
        try:
            check_ffmpeg()
            await self.initialize_robot_connection()
            
            # Start consumer task
            worker_task = asyncio.create_task(self.process_audio_queue())
            
            # Start producer (listener)
            await self.connect_and_listen()
            
            worker_task.cancel()
            
        except KeyboardInterrupt:
            logger.info("Stopped by user")
        finally:
            shutil.rmtree(self.temp_dir, ignore_errors=True)

async def main():
    if len(sys.argv) < 3:
        print("Usage: python receiver.py <robot_ip> <websocket_url>")
        sys.exit(1)

    streamer = WebSocketAudioStreamer(sys.argv[1], sys.argv[2])
    await streamer.run()

if __name__ == "__main__":
    try:
        asyncio.run(main())
    except KeyboardInterrupt:
        pass
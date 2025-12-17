#!/usr/bin/env python3
"""
Persistent WebRTC audio player.
Maintains connection to robot and plays WAV files received via stdin.
Usage: python3 play_audio.py [robot_ip]
Send file paths via stdin (one per line).
"""
import asyncio
import json
import sys
import os
import logging
import hashlib
import base64
import time
from typing import Optional

# Add go2_webrtc to path for unitree modules
SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
GO2_WEBRTC_PATH = os.path.join(SCRIPT_DIR, "..", "go2_webrtc")
if os.path.exists(GO2_WEBRTC_PATH):
    sys.path.insert(0, GO2_WEBRTC_PATH)

logging.basicConfig(level=logging.INFO, format='%(asctime)s - %(message)s', datefmt='%H:%M:%S')
logger = logging.getLogger(__name__)

from unitree_webrtc_connect.webrtc_driver import (
    UnitreeWebRTCConnection,
    WebRTCConnectionMethod,
)
from unitree_webrtc_connect.webrtc_audiohub import WebRTCAudioHub
from unitree_webrtc_connect.constants import AUDIO_API
from pydub import AudioSegment

# Default robot IP
DEFAULT_ROBOT_IP = "192.168.123.161"


class RobotPlayer:
    def __init__(self, robot_ip: str):
        self.robot_ip = robot_ip
        self.webrtc_conn = None
        self.audio_hub = None
        self.connected = False

    async def connect(self):
        logger.info(f"Connecting to robot at {self.robot_ip}")
        self.webrtc_conn = UnitreeWebRTCConnection(
            WebRTCConnectionMethod.LocalSTA, ip=self.robot_ip
        )
        await self.webrtc_conn.connect()
        self.audio_hub = WebRTCAudioHub(self.webrtc_conn, logger)
        await asyncio.sleep(2.0)
        
        # Set play mode to "no_cycle" (play once, don't repeat)
        await self.audio_hub.set_play_mode("no_cycle")
        logger.info("Play mode set to: no_cycle")
        
        self.connected = True
        logger.info("WebRTC ready")

    async def get_audio_list(self) -> list:
        """Get the list of audio files on the robot"""
        try:
            response = await self.audio_hub.get_audio_list()
            if response and isinstance(response, dict):
                data_obj = response.get('data', {})
                if isinstance(data_obj, dict):
                    data_str = data_obj.get('data', '{}')
                else:
                    data_str = str(data_obj)
                return json.loads(data_str).get('audio_list', [])
        except Exception as e:
            logger.error(f"Error getting audio list: {e}")
        return []

    async def find_uuid(self, name: str) -> Optional[str]:
        """Find audio UUID by name"""
        try:
            audio_list = await self.get_audio_list()
            for audio in audio_list:
                if audio.get('CUSTOM_NAME') == name:
                    return audio['UNIQUE_ID']
        except Exception as e:
            logger.error(f"Error finding UUID: {e}")
        return None

    async def fast_upload(self, wav_path: str, file_name: str) -> bool:
        """
        Optimized upload with larger chunks and minimal delays.
        Uses 8KB chunks instead of 4KB for faster transfer.
        """
        try:
            with open(wav_path, 'rb') as f:
                audio_data = f.read()

            file_md5 = hashlib.md5(audio_data).hexdigest()
            b64_data = base64.b64encode(audio_data).decode('utf-8')
            
            # Use larger chunks (8KB) for faster upload
            chunk_size = 8192
            chunks = [b64_data[i:i + chunk_size] for i in range(0, len(b64_data), chunk_size)]
            total_chunks = len(chunks)
            
            logger.info(f"Uploading {file_name} ({len(audio_data)} bytes, {total_chunks} chunks)")

            for i, chunk in enumerate(chunks, 1):
                parameter = {
                    'file_name': file_name,
                    'file_type': 'wav',
                    'file_size': len(audio_data),
                    'current_block_index': i,
                    'total_block_number': total_chunks,
                    'block_content': chunk,
                    'current_block_size': len(chunk),
                    'file_md5': file_md5,
                    'create_time': int(time.time() * 1000)
                }
                
                await self.audio_hub.data_channel.pub_sub.publish_request_new(
                    "rt/api/audiohub/request",
                    {
                        "api_id": AUDIO_API['UPLOAD_AUDIO_FILE'],
                        "parameter": json.dumps(parameter, ensure_ascii=True)
                    }
                )
                
                # Minimal delay - just enough for the robot to process
                if i % 10 == 0:
                    await asyncio.sleep(0.02)

            logger.info("Upload complete")
            return True
            
        except Exception as e:
            logger.error(f"Upload error: {e}")
            return False

    async def play(self, wav_path: str) -> bool:
        if not os.path.exists(wav_path):
            logger.error(f"File not found: {wav_path}")
            return False

        try:
            audio = AudioSegment.from_wav(wav_path)
            duration = len(audio) / 1000.0
        except Exception as e:
            logger.error(f"Read error: {e}")
            return False

        # Use a short unique name based on file hash (avoids duplicates)
        with open(wav_path, 'rb') as f:
            file_hash = hashlib.md5(f.read()).hexdigest()[:8]
        name = f"tts_{file_hash}"

        # Check if already exists
        uuid = await self.find_uuid(name)
        
        if not uuid:
            # Use optimized fast upload
            logger.info(f"Uploading: {name} ({duration:.1f}s)")
            await self.fast_upload(wav_path, name)
            
            # Quick retry to find UUID
            for _ in range(5):
                await asyncio.sleep(0.2)
                uuid = await self.find_uuid(name)
                if uuid:
                    break
            
            if not uuid:
                logger.error("Upload failed")
                return False
        else:
            logger.info(f"Using cached: {name}")

        logger.info(f"Playing ({duration:.1f}s)")
        await self.audio_hub.play_by_uuid(uuid)
        
        # Wait for audio to finish
        await asyncio.sleep(duration + 0.2)
        return True


async def main():
    robot_ip = sys.argv[1] if len(sys.argv) > 1 else DEFAULT_ROBOT_IP
    logger.info(f"Using robot IP: {robot_ip}")
    
    player = RobotPlayer(robot_ip)
    await player.connect()

    print("READY", flush=True)

    loop = asyncio.get_event_loop()
    reader = asyncio.StreamReader()
    protocol = asyncio.StreamReaderProtocol(reader)
    await loop.connect_read_pipe(lambda: protocol, sys.stdin)

    while True:
        try:
            line = await reader.readline()
            if not line:
                break
            path = line.decode().strip()
            if path:
                await player.play(path)
                print("DONE", flush=True)
        except Exception as e:
            logger.error(f"Error: {e}")


if __name__ == "__main__":
    try:
        asyncio.run(main())
    except KeyboardInterrupt:
        pass

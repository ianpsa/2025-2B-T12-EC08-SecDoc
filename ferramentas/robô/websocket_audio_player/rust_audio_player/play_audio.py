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
from typing import Optional

# Add go2_webrtc to path for unitree modules
SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
GO2_WEBRTC_PATH = os.path.join(SCRIPT_DIR, "..", "go2_webrtc")
if os.path.exists(GO2_WEBRTC_PATH):
    sys.path.insert(0, GO2_WEBRTC_PATH)

logging.basicConfig(level=logging.DEBUG, format='%(asctime)s - %(levelname)s - %(message)s', datefmt='%H:%M:%S')
logger = logging.getLogger(__name__)

from unitree_webrtc_connect.webrtc_driver import (
    UnitreeWebRTCConnection,
    WebRTCConnectionMethod,
)
from unitree_webrtc_connect.webrtc_audiohub import WebRTCAudioHub
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
        self.connected = True
        logger.info("WebRTC ready")

    async def find_uuid(self, name: str) -> Optional[str]:
        """Find audio UUID by name (matches example pattern from go2_webrtc)"""
        try:
            response = await self.audio_hub.get_audio_list()
            if response and isinstance(response, dict):
                # Extract nested data structure (response.data.data)
                data_obj = response.get('data', {})
                if isinstance(data_obj, dict):
                    data_str = data_obj.get('data', '{}')
                else:
                    data_str = str(data_obj)
                
                audio_list = json.loads(data_str).get('audio_list', [])
                logger.debug(f"Audio list has {len(audio_list)} items, looking for: {name}")
                
                for audio in audio_list:
                    custom_name = audio.get('CUSTOM_NAME', '')
                    # Try exact match first, then partial match
                    if custom_name == name or name in custom_name:
                        logger.info(f"Found audio: {custom_name} -> {audio['UNIQUE_ID']}")
                        return audio['UNIQUE_ID']
                
                # Log available names for debugging
                if audio_list:
                    names = [a.get('CUSTOM_NAME', 'unknown') for a in audio_list[-5:]]
                    logger.debug(f"Recent audio names: {names}")
        except Exception as e:
            logger.error(f"Error finding UUID: {e}")
        return None

    async def play(self, wav_path: str) -> bool:
        logger.info(f"Playing: {wav_path}")
        
        if not os.path.exists(wav_path):
            logger.error(f"File not found: {wav_path}")
            return False

        try:
            file_size = os.path.getsize(wav_path)
            logger.info(f"File size: {file_size} bytes")
            audio = AudioSegment.from_wav(wav_path)
            duration = len(audio) / 1000.0
            logger.info(f"Duration: {duration:.1f}s")
        except Exception as e:
            logger.error(f"Read error: {e}")
            return False

        # Use filename from path (e.g., M1234567890.wav -> M1234567890)
        name = os.path.splitext(os.path.basename(wav_path))[0]

        # Check if already exists
        uuid = await self.find_uuid(name)
        
        if not uuid:
            # Upload audio file
            logger.info(f"Uploading audio file: {name}")
            await self.audio_hub.upload_audio_file(wav_path)
            
            # Wait for upload to complete and retry finding UUID
            for attempt in range(10):
                await asyncio.sleep(0.5)
                uuid = await self.find_uuid(name)
                if uuid:
                    logger.info(f"Upload successful on attempt {attempt + 1}")
                    break
            
            if not uuid:
                logger.error(f"Upload failed - could not find {name} in audio list")
                return False

        logger.info(f"Playing audio ({duration:.1f}s) with UUID: {uuid}")
        await self.audio_hub.play_by_uuid(uuid)
        await asyncio.sleep(max(0.5, duration - 0.3))
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

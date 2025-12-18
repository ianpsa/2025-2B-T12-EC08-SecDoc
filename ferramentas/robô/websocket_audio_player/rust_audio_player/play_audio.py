#!/usr/bin/env python3
"""
Streaming WebRTC audio player using Megaphone mode.
Plays audio chunks in real-time as they are sent (no waiting for full upload).
Usage: python3 play_audio.py [robot_ip]
Send file paths via stdin (one per line).
"""
import asyncio
import json
import sys
import os
import logging
import base64
from pydub import AudioSegment

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

# Default robot IP
DEFAULT_ROBOT_IP = "192.168.123.161"


class StreamingPlayer:
    """
    Streams audio to robot using Megaphone mode.
    Audio plays immediately as chunks are sent - no waiting for full upload.
    """
    
    def __init__(self, robot_ip: str):
        self.robot_ip = robot_ip
        self.webrtc_conn = None
        self.audio_hub = None
        self.in_megaphone_mode = False

    async def connect(self):
        logger.info(f"Connecting to robot at {self.robot_ip}")
        self.webrtc_conn = UnitreeWebRTCConnection(
            WebRTCConnectionMethod.LocalSTA, ip=self.robot_ip
        )
        await self.webrtc_conn.connect()
        self.audio_hub = WebRTCAudioHub(self.webrtc_conn, logger)
        await asyncio.sleep(2.0)
        logger.info("WebRTC ready")

    async def enter_megaphone(self):
        """Enter megaphone/streaming mode"""
        if not self.in_megaphone_mode:
            await self.audio_hub.enter_megaphone()
            self.in_megaphone_mode = True
            await asyncio.sleep(0.1)
            logger.info("Megaphone mode: ON")

    async def exit_megaphone(self):
        """Exit megaphone mode"""
        if self.in_megaphone_mode:
            await self.audio_hub.exit_megaphone()
            self.in_megaphone_mode = False
            logger.info("Megaphone mode: OFF")

    async def stream_audio(self, wav_path: str) -> bool:
        """
        Stream audio file in real-time using megaphone mode.
        Audio starts playing immediately as chunks arrive.
        """
        if not os.path.exists(wav_path):
            logger.error(f"File not found: {wav_path}")
            return False

        try:
            # Get audio info
            audio = AudioSegment.from_wav(wav_path)
            duration = len(audio) / 1000.0
            logger.info(f"Streaming audio ({duration:.1f}s)")
        except Exception as e:
            logger.error(f"Read error: {e}")
            return False

        try:
            # Read and encode audio
            with open(wav_path, 'rb') as f:
                audio_data = f.read()
            
            b64_data = base64.b64encode(audio_data).decode('utf-8')
            
            # Use 8KB chunks for good balance of speed and reliability
            chunk_size = 8192
            chunks = [b64_data[i:i + chunk_size] for i in range(0, len(b64_data), chunk_size)]
            total_chunks = len(chunks)
            
            logger.info(f"Streaming {total_chunks} chunks...")

            # Enter megaphone mode
            await self.enter_megaphone()

            # Stream chunks - they play as they arrive!
            for i, chunk in enumerate(chunks, 1):
                parameter = {
                    'current_block_size': len(chunk),
                    'block_content': chunk,
                    'current_block_index': i,
                    'total_block_number': total_chunks
                }
                
                await self.audio_hub.data_channel.pub_sub.publish_request_new(
                    "rt/api/audiohub/request",
                    {
                        "api_id": AUDIO_API['UPLOAD_MEGAPHONE'],
                        "parameter": json.dumps(parameter, ensure_ascii=True)
                    }
                )
                
                # Small delay to prevent overwhelming the connection
                # but fast enough for real-time streaming
                if i % 5 == 0:
                    await asyncio.sleep(0.01)

            logger.info("Stream complete")
            
            # Wait for audio to finish playing
            # The audio is already playing, so we just wait for remaining duration
            await asyncio.sleep(0.5)
            
            # Exit megaphone mode
            await self.exit_megaphone()
            
            return True
            
        except Exception as e:
            logger.error(f"Stream error: {e}")
            await self.exit_megaphone()
            return False


async def main():
    robot_ip = sys.argv[1] if len(sys.argv) > 1 else DEFAULT_ROBOT_IP
    logger.info(f"Using robot IP: {robot_ip}")
    
    player = StreamingPlayer(robot_ip)
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
                await player.stream_audio(path)
                print("DONE", flush=True)
        except Exception as e:
            logger.error(f"Error: {e}")


if __name__ == "__main__":
    try:
        asyncio.run(main())
    except KeyboardInterrupt:
        pass

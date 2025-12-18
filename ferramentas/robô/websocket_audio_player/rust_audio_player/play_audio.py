#!/usr/bin/env python3
"""
Simple WebRTC audio player using Megaphone streaming.
Receives WAV file paths from stdin and streams them to the robot.
Usage: python3 play_audio.py [robot_ip]
"""
import asyncio
import json
import sys
import os
import logging
import base64

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

DEFAULT_ROBOT_IP = "192.168.123.161"


class MegaphonePlayer:
    """Streams audio directly to robot using Megaphone mode."""
    
    def __init__(self, robot_ip: str):
        self.robot_ip = robot_ip
        self.webrtc_conn = None
        self.audio_hub = None

    async def connect(self):
        logger.info(f"Connecting to {self.robot_ip}")
        self.webrtc_conn = UnitreeWebRTCConnection(
            WebRTCConnectionMethod.LocalSTA, ip=self.robot_ip
        )
        await self.webrtc_conn.connect()
        self.audio_hub = WebRTCAudioHub(self.webrtc_conn, logger)
        await asyncio.sleep(2.0)
        logger.info("Connected")

    async def stream_wav(self, wav_path: str):
        """Stream a WAV file using megaphone mode (plays immediately)."""
        if not os.path.exists(wav_path):
            logger.error(f"File not found: {wav_path}")
            return False

        try:
            with open(wav_path, 'rb') as f:
                audio_data = f.read()
            
            if len(audio_data) < 44:  # WAV header is 44 bytes
                logger.error("Invalid WAV file")
                return False

            logger.info(f"Streaming {len(audio_data)} bytes")
            
            # Enter megaphone mode
            await self.audio_hub.enter_megaphone()
            await asyncio.sleep(0.1)
            
            # Encode and split into chunks
            b64_data = base64.b64encode(audio_data).decode('utf-8')
            chunk_size = 8192
            chunks = [b64_data[i:i + chunk_size] for i in range(0, len(b64_data), chunk_size)]
            total = len(chunks)
            
            # Stream all chunks
            for i, chunk in enumerate(chunks, 1):
                await self.audio_hub.data_channel.pub_sub.publish_request_new(
                    "rt/api/audiohub/request",
                    {
                        "api_id": AUDIO_API['UPLOAD_MEGAPHONE'],
                        "parameter": json.dumps({
                            'current_block_size': len(chunk),
                            'block_content': chunk,
                            'current_block_index': i,
                            'total_block_number': total
                        })
                    }
                )
                # Small delay every 10 chunks
                if i % 10 == 0:
                    await asyncio.sleep(0.01)
            
            # Exit megaphone mode
            await asyncio.sleep(0.3)
            await self.audio_hub.exit_megaphone()
            
            logger.info("Stream complete")
            return True
            
        except Exception as e:
            logger.error(f"Error: {e}")
            try:
                await self.audio_hub.exit_megaphone()
            except:
                pass
            return False


async def main():
    robot_ip = sys.argv[1] if len(sys.argv) > 1 else DEFAULT_ROBOT_IP
    
    player = MegaphonePlayer(robot_ip)
    await player.connect()

    print("READY", flush=True)

    # Read file paths from stdin
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
                await player.stream_wav(path)
                print("DONE", flush=True)
            
        except Exception as e:
            logger.error(f"Error: {e}")


if __name__ == "__main__":
    try:
        asyncio.run(main())
    except KeyboardInterrupt:
        pass

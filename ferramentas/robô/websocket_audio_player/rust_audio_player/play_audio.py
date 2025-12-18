#!/usr/bin/env python3
"""
Streaming WebRTC audio player using Megaphone mode.
Receives commands from Rust and streams WAV chunks in real-time.
Protocol:
  - START: Enter megaphone mode
  - CHUNK:<path>: Stream a WAV chunk file
  - STOP: Exit megaphone mode and send DONE
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

# Default robot IP
DEFAULT_ROBOT_IP = "192.168.123.161"


class StreamingPlayer:
    """
    Streams audio chunks to robot using Megaphone mode.
    Each chunk plays immediately as it arrives.
    """
    
    def __init__(self, robot_ip: str):
        self.robot_ip = robot_ip
        self.webrtc_conn = None
        self.audio_hub = None
        self.in_megaphone_mode = False
        self.chunk_index = 0

    async def connect(self):
        logger.info(f"Connecting to robot at {self.robot_ip}")
        self.webrtc_conn = UnitreeWebRTCConnection(
            WebRTCConnectionMethod.LocalSTA, ip=self.robot_ip
        )
        await self.webrtc_conn.connect()
        self.audio_hub = WebRTCAudioHub(self.webrtc_conn, logger)
        await asyncio.sleep(2.0)
        logger.info("WebRTC ready")

    async def start_streaming(self):
        """Enter megaphone mode for streaming"""
        if not self.in_megaphone_mode:
            await self.audio_hub.enter_megaphone()
            self.in_megaphone_mode = True
            self.chunk_index = 0
            logger.info("Streaming started")

    async def stop_streaming(self):
        """Exit megaphone mode"""
        if self.in_megaphone_mode:
            await self.audio_hub.exit_megaphone()
            self.in_megaphone_mode = False
            logger.info(f"Streaming stopped ({self.chunk_index} chunks)")

    async def stream_chunk(self, wav_path: str):
        """
        Stream a single WAV chunk to the robot.
        The chunk plays immediately in megaphone mode.
        """
        if not os.path.exists(wav_path):
            logger.error(f"Chunk not found: {wav_path}")
            return

        try:
            # Read and encode chunk
            with open(wav_path, 'rb') as f:
                audio_data = f.read()
            
            b64_data = base64.b64encode(audio_data).decode('utf-8')
            
            # Split into smaller pieces for the API (8KB each)
            piece_size = 8192
            pieces = [b64_data[i:i + piece_size] for i in range(0, len(b64_data), piece_size)]
            total_pieces = len(pieces)

            # Send all pieces of this chunk
            for i, piece in enumerate(pieces, 1):
                self.chunk_index += 1
                parameter = {
                    'current_block_size': len(piece),
                    'block_content': piece,
                    'current_block_index': i,
                    'total_block_number': total_pieces
                }
                
                await self.audio_hub.data_channel.pub_sub.publish_request_new(
                    "rt/api/audiohub/request",
                    {
                        "api_id": AUDIO_API['UPLOAD_MEGAPHONE'],
                        "parameter": json.dumps(parameter, ensure_ascii=True)
                    }
                )

        except Exception as e:
            logger.error(f"Stream chunk error: {e}")


async def main():
    robot_ip = sys.argv[1] if len(sys.argv) > 1 else DEFAULT_ROBOT_IP
    logger.info(f"Robot IP: {robot_ip}")
    
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
            
            cmd = line.decode().strip()
            
            if cmd == "START":
                await player.start_streaming()
            
            elif cmd.startswith("CHUNK:"):
                chunk_path = cmd[6:]  # Remove "CHUNK:" prefix
                await player.stream_chunk(chunk_path)
            
            elif cmd == "STOP":
                await player.stop_streaming()
                print("DONE", flush=True)
            
        except Exception as e:
            logger.error(f"Error: {e}")


if __name__ == "__main__":
    try:
        asyncio.run(main())
    except KeyboardInterrupt:
        pass

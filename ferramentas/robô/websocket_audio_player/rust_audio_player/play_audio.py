#!/usr/bin/env python3
"""
Persistent WebRTC audio player.
Maintains connection to robot and plays WAV files received via stdin.
Usage: python3 play_audio.py <robot_ip>
Send file paths via stdin (one per line).
"""
import asyncio
import json
import sys
import os
import logging
from typing import Optional

logging.basicConfig(level=logging.INFO, format='%(asctime)s - %(message)s', datefmt='%H:%M:%S')
logger = logging.getLogger(__name__)

from unitree_webrtc_connect.webrtc_driver import (
    UnitreeWebRTCConnection,
    WebRTCConnectionMethod,
)
from unitree_webrtc_connect.webrtc_audiohub import WebRTCAudioHub
from pydub import AudioSegment


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
        try:
            response = await self.audio_hub.get_audio_list()
            if response:
                data = response if isinstance(response, dict) else json.loads(response)
                inner = data.get("data", {})
                if isinstance(inner, str):
                    inner = json.loads(inner)
                for audio in inner.get("audio_list", []):
                    if name in audio["CUSTOM_NAME"]:
                        return audio["UNIQUE_ID"]
        except Exception:
            pass
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
            # Upload
            await self.audio_hub.upload_audio_file(wav_path)
            for _ in range(6):
                await asyncio.sleep(0.2)
                uuid = await self.find_uuid(name)
                if uuid:
                    break

        if uuid:
            logger.info(f"Playing ({duration:.1f}s)")
            await self.audio_hub.play_by_uuid(uuid)
            await asyncio.sleep(max(0, duration - 0.3))
            return True
        else:
            logger.error("Upload failed")
            return False


async def main():
    if len(sys.argv) < 2:
        print(f"Usage: {sys.argv[0]} <robot_ip>", file=sys.stderr)
        sys.exit(1)

    player = RobotPlayer(sys.argv[1])
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

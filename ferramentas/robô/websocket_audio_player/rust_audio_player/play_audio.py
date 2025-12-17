#!/usr/bin/env python3
"""
Minimal script to play a WAV file on the Unitree robot via WebRTC.
Called by the Rust audio player: python3 play_audio.py <robot_ip> <wav_file>
"""
import asyncio
import json
import sys
import os
import time
import logging

logging.basicConfig(level=logging.INFO, format='%(asctime)s - %(message)s', datefmt='%H:%M:%S')
logger = logging.getLogger(__name__)

from unitree_webrtc_connect.webrtc_driver import (
    UnitreeWebRTCConnection,
    WebRTCConnectionMethod,
)
from unitree_webrtc_connect.webrtc_audiohub import WebRTCAudioHub
from pydub import AudioSegment


class RobotAudioPlayer:
    def __init__(self, robot_ip: str):
        self.robot_ip = robot_ip
        self.webrtc_conn = None
        self.audio_hub = None

    async def connect(self):
        logger.info(f"Connecting to robot at {self.robot_ip}")
        self.webrtc_conn = UnitreeWebRTCConnection(
            WebRTCConnectionMethod.LocalSTA, ip=self.robot_ip
        )
        await self.webrtc_conn.connect()
        logger.info("✅ WebRTC Connected")
        self.audio_hub = WebRTCAudioHub(self.webrtc_conn, logger)
        await asyncio.sleep(1.5)

    async def find_uuid_in_list(self, target_name):
        try:
            response = await self.audio_hub.get_audio_list()
            if response:
                data = response if isinstance(response, dict) else json.loads(response)
                inner = data.get("data", {})
                if isinstance(inner, str):
                    inner = json.loads(inner)
                for audio in inner.get("audio_list", []):
                    if target_name in audio["CUSTOM_NAME"]:
                        return audio["UNIQUE_ID"]
        except Exception:
            pass
        return None

    async def play(self, wav_path: str) -> bool:
        if not os.path.exists(wav_path):
            logger.error(f"File not found: {wav_path}")
            return False

        try:
            audio = AudioSegment.from_wav(wav_path)
            duration_sec = len(audio) / 1000.0
        except Exception as e:
            logger.error(f"Failed to read WAV: {e}")
            return False

        timestamp = int(time.time() * 1000)
        base_name = f"Music_{timestamp}"

        # Upload with retry
        success_uuid = None
        for attempt in range(1, 4):
            await self.audio_hub.upload_audio_file(wav_path)
            for _ in range(5):
                await asyncio.sleep(0.5)
                success_uuid = await self.find_uuid_in_list(base_name)
                if success_uuid:
                    break
            if success_uuid:
                break
            logger.warning(f"Upload attempt {attempt} failed, retrying...")

        if success_uuid:
            logger.info(f"▶️  Playing ({duration_sec:.1f}s)")
            await self.audio_hub.play_by_uuid(success_uuid)
            await asyncio.sleep(max(0, duration_sec - 0.2))
            return True
        else:
            logger.error("❌ Failed to upload audio")
            return False


async def main():
    if len(sys.argv) < 3:
        print(f"Usage: {sys.argv[0]} <robot_ip> <wav_file>")
        sys.exit(1)

    robot_ip = sys.argv[1]
    wav_file = sys.argv[2]

    player = RobotAudioPlayer(robot_ip)
    await player.connect()
    success = await player.play(wav_file)
    sys.exit(0 if success else 1)


if __name__ == "__main__":
    try:
        asyncio.run(main())
    except KeyboardInterrupt:
        pass


#!/usr/bin/env python3
"""
Optimized Megaphone audio streamer.
Receives WAV file paths from stdin and streams them to the robot with minimal latency.
"""
import asyncio
import json
import sys
import os
import base64

SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
GO2_WEBRTC_PATH = os.path.join(SCRIPT_DIR, "..", "go2_webrtc")
if os.path.exists(GO2_WEBRTC_PATH):
    sys.path.insert(0, GO2_WEBRTC_PATH)

from unitree_webrtc_connect.webrtc_driver import (
    UnitreeWebRTCConnection,
    WebRTCConnectionMethod,
)
from unitree_webrtc_connect.webrtc_audiohub import WebRTCAudioHub
from unitree_webrtc_connect.constants import AUDIO_API

DEFAULT_ROBOT_IP = "192.168.123.161"


class MegaphonePlayer:
    def __init__(self, robot_ip: str):
        self.robot_ip = robot_ip
        self.webrtc_conn = None
        self.audio_hub = None
        self.in_megaphone = False

    async def connect(self):
        self.webrtc_conn = UnitreeWebRTCConnection(
            WebRTCConnectionMethod.LocalSTA, ip=self.robot_ip
        )
        await self.webrtc_conn.connect()
        self.audio_hub = WebRTCAudioHub(self.webrtc_conn)
        await asyncio.sleep(1.0)

    async def ensure_megaphone(self):
        if not self.in_megaphone:
            await self.audio_hub.enter_megaphone()
            self.in_megaphone = True
            await asyncio.sleep(0.05)

    async def stream_wav(self, wav_path: str):
        if not os.path.exists(wav_path):
            return False

        try:
            with open(wav_path, 'rb') as f:
                audio_data = f.read()
            
            if len(audio_data) < 44:
                return False

            await self.ensure_megaphone()
            
            # Larger chunks = fewer requests = less overhead
            b64_data = base64.b64encode(audio_data).decode('utf-8')
            chunk_size = 16384  # Increased from 8192
            chunks = [b64_data[i:i + chunk_size] for i in range(0, len(b64_data), chunk_size)]
            total = len(chunks)
            
            # Stream all chunks with minimal delay
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
                # Minimal delay only every 20 chunks
                if i % 20 == 0:
                    await asyncio.sleep(0.005)
            
            return True
            
        except:
            return False


async def main():
    robot_ip = sys.argv[1] if len(sys.argv) > 1 else DEFAULT_ROBOT_IP
    
    player = MegaphonePlayer(robot_ip)
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
                await player.stream_wav(path)
                print("DONE", flush=True)
            
        except:
            pass


if __name__ == "__main__":
    try:
        asyncio.run(main())
    except KeyboardInterrupt:
        pass

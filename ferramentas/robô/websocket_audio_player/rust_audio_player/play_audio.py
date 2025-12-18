#!/usr/bin/env python3
"""
Simple megaphone player - sends complete WAV file to robot.
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
CHUNK_SIZE = 16384


class SimplePlayer:
    def __init__(self, robot_ip: str):
        self.robot_ip = robot_ip
        self.audio_hub = None

    async def connect(self):
        conn = UnitreeWebRTCConnection(
            WebRTCConnectionMethod.LocalSTA, ip=self.robot_ip
        )
        await conn.connect()
        self.audio_hub = WebRTCAudioHub(conn)
        await asyncio.sleep(0.2)
        await self.audio_hub.enter_megaphone()

    async def play_wav(self, wav_path: str):
        """Send complete WAV file to megaphone."""
        if not os.path.exists(wav_path):
            print("DONE", flush=True)
            return
        
        try:
            with open(wav_path, 'rb') as f:
                data = f.read()
            
            if len(data) < 44:
                print("DONE", flush=True)
                return

            b64 = base64.b64encode(data).decode('utf-8')
            chunks = [b64[i:i + CHUNK_SIZE] for i in range(0, len(b64), CHUNK_SIZE)]
            total = len(chunks)

            # Send all chunks sequentially
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
                
        except Exception as e:
            print(f"Error: {e}", file=sys.stderr)
        
        print("DONE", flush=True)


async def main():
    robot_ip = sys.argv[1] if len(sys.argv) > 1 else DEFAULT_ROBOT_IP
    
    player = SimplePlayer(robot_ip)
    await player.connect()
    
    print("READY", flush=True)

    reader = asyncio.StreamReader()
    protocol = asyncio.StreamReaderProtocol(reader)
    await asyncio.get_event_loop().connect_read_pipe(lambda: protocol, sys.stdin)

    while True:
        try:
            line = await reader.readline()
            if not line:
                break
            path = line.decode().strip()
            if path:
                await player.play_wav(path)
        except:
            pass


if __name__ == "__main__":
    try:
        asyncio.run(main())
    except KeyboardInterrupt:
        pass

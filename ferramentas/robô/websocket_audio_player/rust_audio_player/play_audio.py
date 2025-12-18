#!/usr/bin/env python3
"""
Zero-delay Megaphone streamer.
Maintains WebRTC connection and megaphone mode.
Plays chunks back-to-back with overlap.
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


class StreamPlayer:
    def __init__(self, robot_ip: str):
        self.robot_ip = robot_ip
        self.audio_hub = None
        self.queue = asyncio.Queue()

    async def connect(self):
        conn = UnitreeWebRTCConnection(
            WebRTCConnectionMethod.LocalSTA, ip=self.robot_ip
        )
        await conn.connect()
        self.audio_hub = WebRTCAudioHub(conn)
        await asyncio.sleep(0.2)
        await self.audio_hub.enter_megaphone()

    async def player_loop(self):
        """Play chunks with minimal gap."""
        while True:
            wav_path = await self.queue.get()
            
            try:
                if os.path.exists(wav_path):
                    with open(wav_path, 'rb') as f:
                        data = f.read()
                    
                    if len(data) >= 44:
                        # Send to megaphone
                        b64 = base64.b64encode(data).decode('utf-8')
                        chunks = [b64[i:i + 16384] for i in range(0, len(b64), 16384)]
                        total = len(chunks)

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

                        # Calculate duration and wait (with overlap for next chunk)
                        audio_bytes = len(data) - 44
                        duration = audio_bytes / 32000.0
                        # Wait less to overlap with next chunk start
                        await asyncio.sleep(max(0, duration - 0.08))
            except:
                pass
            
            print("DONE", flush=True)
            self.queue.task_done()


async def main():
    robot_ip = sys.argv[1] if len(sys.argv) > 1 else DEFAULT_ROBOT_IP
    
    player = StreamPlayer(robot_ip)
    await player.connect()
    
    asyncio.create_task(player.player_loop())
    
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
                await player.queue.put(path)
        except:
            pass


if __name__ == "__main__":
    try:
        asyncio.run(main())
    except KeyboardInterrupt:
        pass

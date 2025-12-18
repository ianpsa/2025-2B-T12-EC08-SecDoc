#!/usr/bin/env python3
"""
Pipelined Megaphone streamer with queue.
Accepts chunks continuously and plays them back-to-back.
WebRTC connection stays alive, megaphone mode stays active.
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


class PipelinedPlayer:
    def __init__(self, robot_ip: str):
        self.robot_ip = robot_ip
        self.audio_hub = None
        self.queue = asyncio.Queue()
        self.playing = False

    async def connect(self):
        conn = UnitreeWebRTCConnection(
            WebRTCConnectionMethod.LocalSTA, ip=self.robot_ip
        )
        await conn.connect()
        self.audio_hub = WebRTCAudioHub(conn)
        await asyncio.sleep(0.3)
        
        # Enter megaphone mode once - stay in it forever
        await self.audio_hub.enter_megaphone()
        await asyncio.sleep(0.05)

    async def player_loop(self):
        """Continuously plays queued audio back-to-back."""
        while True:
            wav_path = await self.queue.get()
            
            try:
                await self._stream_file(wav_path)
            except Exception:
                pass
            
            print("DONE", flush=True)
            self.queue.task_done()

    async def _stream_file(self, wav_path: str):
        """Stream a single WAV file to megaphone."""
        if not os.path.exists(wav_path):
            return

        with open(wav_path, 'rb') as f:
            data = f.read()
        
        if len(data) < 44:
            return

        # Encode and chunk
        b64 = base64.b64encode(data).decode('utf-8')
        chunk_size = 16384
        chunks = [b64[i:i + chunk_size] for i in range(0, len(b64), chunk_size)]
        total = len(chunks)

        # Send all chunks as fast as possible
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

        # Wait for this chunk to finish playing
        # 16kHz mono 16-bit = 32000 bytes/sec
        audio_bytes = len(data) - 44
        duration = audio_bytes / 32000.0
        
        # Wait slightly less than full duration to overlap with next chunk
        await asyncio.sleep(max(0, duration - 0.05))

    async def enqueue(self, path: str):
        """Add a file to the play queue."""
        await self.queue.put(path)


async def main():
    robot_ip = sys.argv[1] if len(sys.argv) > 1 else DEFAULT_ROBOT_IP
    
    player = PipelinedPlayer(robot_ip)
    await player.connect()
    
    # Start player loop in background
    asyncio.create_task(player.player_loop())
    
    print("READY", flush=True)

    # Read file paths from stdin and enqueue
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
                await player.enqueue(path)
        except:
            pass


if __name__ == "__main__":
    try:
        asyncio.run(main())
    except KeyboardInterrupt:
        pass

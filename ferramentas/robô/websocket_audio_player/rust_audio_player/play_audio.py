#!/usr/bin/env python3
"""
High-performance megaphone streamer.
Optimized for low-latency continuous audio playback.
"""
import asyncio
import json
import sys
import os
import base64
from concurrent.futures import ThreadPoolExecutor

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

# Larger chunks = fewer API calls = faster
CHUNK_SIZE = 32768


class ContinuousPlayer:
    def __init__(self, robot_ip: str):
        self.robot_ip = robot_ip
        self.audio_hub = None
        self._executor = ThreadPoolExecutor(max_workers=2)

    async def connect(self):
        conn = UnitreeWebRTCConnection(
            WebRTCConnectionMethod.LocalSTA, ip=self.robot_ip
        )
        await conn.connect()
        self.audio_hub = WebRTCAudioHub(conn)
        await asyncio.sleep(0.15)
        await self.audio_hub.enter_megaphone()

    def _read_and_encode(self, wav_path: str):
        """Read and encode file in thread pool."""
        try:
            with open(wav_path, 'rb') as f:
                data = f.read()
            if len(data) < 44:
                return None
            return base64.b64encode(data).decode('utf-8')
        except:
            return None

    async def send_wav(self, wav_path: str):
        """Send WAV to megaphone buffer - fire and forget."""
        if not os.path.exists(wav_path):
            print("DONE", flush=True)
            return
        
        # Read file in thread pool to not block event loop
        loop = asyncio.get_event_loop()
        b64 = await loop.run_in_executor(self._executor, self._read_and_encode, wav_path)
        
        if not b64:
            print("DONE", flush=True)
            return

        chunks = [b64[i:i + CHUNK_SIZE] for i in range(0, len(b64), CHUNK_SIZE)]
        total = len(chunks)

        # Send all chunks concurrently using gather
        tasks = []
        for i, chunk in enumerate(chunks, 1):
            task = self.audio_hub.data_channel.pub_sub.publish_request_new(
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
            tasks.append(task)
        
        # Fire all at once
        await asyncio.gather(*tasks, return_exceptions=True)
        print("DONE", flush=True)


async def main():
    robot_ip = sys.argv[1] if len(sys.argv) > 1 else DEFAULT_ROBOT_IP
    
    player = ContinuousPlayer(robot_ip)
    await player.connect()
    
    print("READY", flush=True)

    reader = asyncio.StreamReader()
    protocol = asyncio.StreamReaderProtocol(reader)
    await asyncio.get_event_loop().connect_read_pipe(lambda: protocol, sys.stdin)

    # Process multiple files concurrently
    pending_tasks = set()
    
    while True:
        try:
            line = await reader.readline()
            if not line:
                break
            path = line.decode().strip()
            if path:
                # Don't await - let it run in background
                task = asyncio.create_task(player.send_wav(path))
                pending_tasks.add(task)
                task.add_done_callback(pending_tasks.discard)
        except:
            pass
    
    # Wait for remaining tasks
    if pending_tasks:
        await asyncio.gather(*pending_tasks, return_exceptions=True)


if __name__ == "__main__":
    try:
        asyncio.run(main())
    except KeyboardInterrupt:
        pass

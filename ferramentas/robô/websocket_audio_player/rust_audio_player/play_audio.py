#!/usr/bin/env python3
"""
Continuous megaphone streamer - sequential chunk delivery.
Sends chunks in order without gaps for smooth playback.
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

# Chunk size for base64 splitting
CHUNK_SIZE = 16384


class ContinuousPlayer:
    def __init__(self, robot_ip: str):
        self.robot_ip = robot_ip
        self.audio_hub = None
        self._queue = asyncio.Queue()
        self._sender_task = None

    async def connect(self):
        conn = UnitreeWebRTCConnection(
            WebRTCConnectionMethod.LocalSTA, ip=self.robot_ip
        )
        await conn.connect()
        self.audio_hub = WebRTCAudioHub(conn)
        await asyncio.sleep(0.2)
        await self.audio_hub.enter_megaphone()
        
        # Start background sender
        self._sender_task = asyncio.create_task(self._send_loop())

    async def _send_loop(self):
        """Background task that sends queued audio sequentially."""
        while True:
            try:
                wav_path = await self._queue.get()
                await self._send_wav_internal(wav_path)
                self._queue.task_done()
            except asyncio.CancelledError:
                break
            except Exception as e:
                pass

    async def _send_wav_internal(self, wav_path: str):
        """Send WAV file to megaphone - chunks sent SEQUENTIALLY."""
        if not os.path.exists(wav_path):
            return
        
        try:
            with open(wav_path, 'rb') as f:
                data = f.read()
            
            if len(data) < 44:
                return

            b64 = base64.b64encode(data).decode('utf-8')
            chunks = [b64[i:i + CHUNK_SIZE] for i in range(0, len(b64), CHUNK_SIZE)]
            total = len(chunks)

            # Send chunks SEQUENTIALLY - megaphone API requires order!
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
                # Small delay between chunks to prevent overwhelming the buffer
                if i < total:
                    await asyncio.sleep(0.001)
                    
        except Exception as e:
            pass

    async def queue_wav(self, wav_path: str):
        """Queue a WAV file for playback (non-blocking)."""
        await self._queue.put(wav_path)
        print("DONE", flush=True)


async def main():
    robot_ip = sys.argv[1] if len(sys.argv) > 1 else DEFAULT_ROBOT_IP
    
    player = ContinuousPlayer(robot_ip)
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
                await player.queue_wav(path)
        except:
            pass


if __name__ == "__main__":
    try:
        asyncio.run(main())
    except KeyboardInterrupt:
        pass

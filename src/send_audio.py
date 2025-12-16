#!/usr/bin/env python3
"""
Simple WebSocket Audio Client for Unitree Go2 Robot

Sends audio files to the robot's WebRTC audio streaming service.

Usage:
    python send_audio.py <robot_ip> <audio_file>

Example:
    python send_audio.py 192.168.123.161 hello.mp3
    python send_audio.py 192.168.123.161 alert.wav
"""

import websockets
import asyncio
import sys
import os
from pathlib import Path


async def send_audio(robot_ip: str, audio_file: str, port: int = 8080):
    """
    Send audio file to robot via WebSocket

    Args:
        robot_ip: Robot's IP address (e.g., "192.168.123.161")
        audio_file: Path to audio file
        port: WebSocket port (default: 8080)
    """
    uri = f"ws://{robot_ip}:{port}"

    # Verify file exists
    if not os.path.exists(audio_file):
        print(f"Error: File not found: {audio_file}")
        return False

    file_size = os.path.getsize(audio_file)
    file_name = Path(audio_file).name

    print(f"Connecting to robot at {uri}...")

    try:
        async with websockets.connect(uri) as websocket:
            print(f"✓ Connected!")

            # Read audio file
            with open(audio_file, "rb") as f:
                audio_data = f.read()

            print(f"Sending {file_name} ({file_size:,} bytes)...")

            # Send as binary WebSocket message
            await websocket.send(audio_data)

            # Wait for acknowledgment
            response = await asyncio.wait_for(websocket.recv(), timeout=10.0)

            if response.startswith("OK"):
                print(f"✓ {response}")
                print(f"✓ Audio sent successfully!")
                return True
            else:
                print(f"✗ Server response: {response}")
                return False

    except websockets.exceptions.WebSocketException as e:
        print(f"✗ WebSocket error: {e}")
        print(f"\nTroubleshooting:")
        print(f"  1. Is the service running on the robot?")
        print(f"  2. Can you reach the robot? Try: ping {robot_ip}")
        print(f"  3. Is the port correct? Default: 8080")
        return False
    except asyncio.TimeoutError:
        print(f"✗ Timeout waiting for response from robot")
        return False
    except Exception as e:
        print(f"✗ Error: {e}")
        return False


def main():
    if len(sys.argv) < 3:
        print(__doc__)
        print("\nError: Missing arguments")
        print(f"Usage: {sys.argv[0]} <robot_ip> <audio_file>")
        print(f"\nExample:")
        print(f"  {sys.argv[0]} 192.168.123.161 hello.mp3")
        sys.exit(1)

    robot_ip = sys.argv[1]
    audio_file = sys.argv[2]

    # Optional: custom port
    port = 8080
    if len(sys.argv) > 3:
        try:
            port = int(sys.argv[3])
        except ValueError:
            print(f"Warning: Invalid port '{sys.argv[3]}', using default 8080")

    print("=" * 50)
    print("  Unitree Go2 Audio Client")
    print("=" * 50)
    print()

    success = asyncio.run(send_audio(robot_ip, audio_file, port))

    if success:
        print()
        print("The robot should now be playing your audio!")
        sys.exit(0)
    else:
        print()
        print("Failed to send audio. Check the errors above.")
        sys.exit(1)


if __name__ == "__main__":
    main()

import asyncio
import json
import base64
import sys
import websockets
import os

async def handle_client(websocket, audio_file_path: str, audio_format: str = "mp3"):
    """
    Handle a client connection and send audio file ONCE, then keep connection alive.
    """
    client_addr = websocket.remote_address
    print(f"✅ Client connected from {client_addr}")

    try:
        # Read audio file
        with open(audio_file_path, "rb") as f:
            audio_bytes = f.read()

        # Encode to base64
        audio_b64 = base64.b64encode(audio_bytes).decode("utf-8")

        # Create message
        message = {"audio": audio_b64, "format": audio_format}

        print(f"📤 Sending audio file: {audio_file_path}")
        print(f"   Format: {audio_format}")
        print(f"   Size: {len(audio_bytes):,} bytes")

        # Send message
        await websocket.send(json.dumps(message))
        print("✅ Audio sent successfully!")

        print("⏳ Keeping connection open (Press Ctrl+C to stop server)...")
        
        # FIXED: Wait forever (or until client disconnects) 
        # instead of disconnecting after 10 seconds
        await websocket.wait_closed()

    except FileNotFoundError:
        print(f"❌ Error: Audio file not found: {audio_file_path}")
    except websockets.exceptions.ConnectionClosed:
        print(f"⚠️  Client disconnected unexpectedly")
    except Exception as e:
        print(f"❌ Error sending audio: {e}")
    finally:
        print(f"🔌 Client {client_addr} disconnected session\n")


async def start_server(host: str, port: int, audio_file_path: str, audio_format: str):
    print("\n" + "=" * 70)
    print(f"🚀 Starting WebSocket server on {host}:{port}")
    print(f"📁 Audio file: {audio_file_path} (format: {audio_format})")
    print("=" * 70)
    print("\n⏳ Waiting for client connections...")

    async def client_handler(websocket):
        await handle_client(websocket, audio_file_path, audio_format)

    async with websockets.serve(client_handler, host, port):
        await asyncio.Future()  # Run forever


async def main():
    if len(sys.argv) < 2:
        print("Usage: python test_websocket_client.py <audio_file_path> [format] [host] [port]")
        sys.exit(1)

    audio_file_path = sys.argv[1]
    audio_format = sys.argv[2] if len(sys.argv) > 2 else "mp3"
    host = sys.argv[3] if len(sys.argv) > 3 else "0.0.0.0"
    port = int(sys.argv[4]) if len(sys.argv) > 4 else 8765

    if not os.path.exists(audio_file_path):
        print(f"\n❌ ERROR: Audio file not found: {audio_file_path}")
        sys.exit(1)

    await start_server(host, port, audio_file_path, audio_format)


if __name__ == "__main__":
    try:
        asyncio.run(main())
    except KeyboardInterrupt:
        print("\nServer stopped by user")
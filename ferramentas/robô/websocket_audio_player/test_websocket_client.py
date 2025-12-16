import asyncio
import json
import base64
import sys
import websockets


async def handle_client(websocket, audio_file_path: str, audio_format: str = "mp3"):
    """
    Handle a client connection and send audio file

    Args:
        websocket: WebSocket connection
        audio_file_path: Path to the audio file to send
        audio_format: Audio format (mp3 or wav, default: mp3)
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
        print(f"   Size: {len(audio_bytes):,} bytes ({len(audio_bytes) / 1024:.1f} KB)")

        # Send message
        await websocket.send(json.dumps(message))
        print("✅ Audio sent successfully!")

        # Keep connection open to allow processing
        print("⏳ Waiting for client to process audio...")
        await asyncio.sleep(10)  # Wait 10 seconds

    except FileNotFoundError:
        print(f"❌ Error: Audio file not found: {audio_file_path}")
    except Exception as e:
        print(f"❌ Error sending audio: {e}")
    finally:
        print(f"🔌 Client {client_addr} disconnected\n")


async def start_server(host: str, port: int, audio_file_path: str, audio_format: str):
    """
    Start WebSocket server that sends audio to connecting clients

    Args:
        host: Server host address
        port: Server port
        audio_file_path: Path to audio file to send
        audio_format: Audio format (mp3 or wav)
    """
    print("\n" + "=" * 70)
    print(f"🚀 Starting WebSocket server on {host}:{port}")
    print(f"📁 Audio file: {audio_file_path} (format: {audio_format})")
    print("=" * 70)
    print("\n⏳ Waiting for client connections...")
    print(f"   Connect your receiver with: ws://{host}:{port}\n")

    async def client_handler(websocket):
        await handle_client(websocket, audio_file_path, audio_format)

    async with websockets.serve(client_handler, host, port):
        await asyncio.Future()  # Run forever


async def main():
    if len(sys.argv) < 2:
        print("=" * 70)
        print("WebSocket Audio Server - Sends audio to Unitree robot receiver")
        print("=" * 70)
        print(
            "\nUsage: python test_websocket_client.py <audio_file_path> [format] [host] [port]"
        )
        print("\nArguments:")
        print("  audio_file_path  Path to audio file (required)")
        print("  format           Audio format: 'mp3' or 'wav' (default: mp3)")
        print("  host             Server host (default: 0.0.0.0)")
        print("  port             Server port (default: 8765)")
        print("\nExamples:")
        print("  python test_websocket_client.py teste.mp3")
        print("  python test_websocket_client.py teste.mp3 mp3")
        print("  python test_websocket_client.py teste.mp3 mp3 0.0.0.0 8765")
        print("  python test_websocket_client.py teste_robot.wav wav")
        print("\n" + "=" * 70)
        print("This script acts as a WebSocket SERVER")
        print("The websocket_audio_receiver.py connects as a CLIENT to this server")
        print("=" * 70)
        sys.exit(1)

    audio_file_path = sys.argv[1]
    audio_format = sys.argv[2] if len(sys.argv) > 2 else "mp3"
    host = sys.argv[3] if len(sys.argv) > 3 else "0.0.0.0"
    port = int(sys.argv[4]) if len(sys.argv) > 4 else 8765

    # Validate audio file exists
    import os

    if not os.path.exists(audio_file_path):
        print(f"\n❌ ERROR: Audio file not found: {audio_file_path}")
        print(f"\nCurrent directory: {os.getcwd()}")
        print(f"Available files in current directory:")
        for f in os.listdir("."):
            if f.endswith((".mp3", ".wav")):
                print(f"  - {f}")
        sys.exit(1)

    # Validate format
    if audio_format not in ["mp3", "wav"]:
        print(f"\n❌ ERROR: Invalid format '{audio_format}'. Must be 'mp3' or 'wav'")
        sys.exit(1)

    await start_server(host, port, audio_file_path, audio_format)


if __name__ == "__main__":
    try:
        asyncio.run(main())
    except KeyboardInterrupt:
        print("\nServer stopped by user")

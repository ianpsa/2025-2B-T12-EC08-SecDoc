#!/usr/bin/env python3
"""
Audio streaming client for Unitree Go2 Robot
Streams audio via WebSocket to the robot's WebRTC audio service
Supports WAV and MP3 files, as well as microphone input
"""

import asyncio
import websockets
import wave
import struct
import sys
import os
import argparse
from pathlib import Path

try:
    import pyaudio

    HAS_PYAUDIO = True
except ImportError:
    HAS_PYAUDIO = False
    print("Warning: PyAudio not available. Microphone streaming disabled.")


def get_robot_websocket_url():
    """
    Get robot WebSocket URL from environment or use default.
    Checks ROBOT_IP environment variable (used by the Rust service).
    """
    robot_ip = os.environ.get("ROBOT_IP", "127.0.0.1")
    return f"ws://{robot_ip}:8080"


def pcm_to_simple_wav(pcm_data, sample_rate=16000, channels=1, sample_width=2):
    """
    Convert PCM data to WAV format in memory

    Args:
        pcm_data: Raw PCM bytes
        sample_rate: Sample rate in Hz
        channels: Number of audio channels
        sample_width: Bytes per sample (2 for 16-bit)

    Returns:
        WAV file bytes
    """
    import io

    wav_buffer = io.BytesIO()

    with wave.open(wav_buffer, "wb") as wav_file:
        wav_file.setnchannels(channels)
        wav_file.setsampwidth(sample_width)
        wav_file.setframerate(sample_rate)
        wav_file.writeframes(pcm_data)

    return wav_buffer.getvalue()


def convert_wav_to_pcm(wav_path):
    """
    Read a WAV file and convert to 16kHz mono PCM chunks

    Args:
        wav_path: Path to WAV file

    Yields:
        PCM data chunks
    """
    with wave.open(str(wav_path), "rb") as wav_file:
        # Get audio parameters
        channels = wav_file.getnchannels()
        sample_width = wav_file.getsampwidth()
        framerate = wav_file.getframerate()

        print(f"Input WAV format:")
        print(f"  Channels: {channels}")
        print(f"  Sample width: {sample_width} bytes")
        print(f"  Frame rate: {framerate} Hz")

        # For simplicity, we'll send WAV data as-is if it's already mono 16kHz
        if channels == 1 and framerate == 16000 and sample_width == 2:
            print("  Format is already optimal (mono, 16kHz, 16-bit)")
            # Read in chunks
            chunk_size = 16000  # 1 second of audio
            while True:
                frames = wav_file.readframes(chunk_size)
                if not frames:
                    break
                yield frames
        else:
            # Read all frames and do basic conversion
            print("  Converting to mono 16kHz...")
            frames = wav_file.readframes(wav_file.getnframes())

            # Very basic conversion - just send as is for now
            # In production, you'd want proper resampling
            if channels == 2:
                # Simple stereo to mono conversion (average channels)
                samples = struct.unpack(f"<{len(frames) // 2}h", frames)
                mono_samples = []
                for i in range(0, len(samples), 2):
                    if i + 1 < len(samples):
                        mono_samples.append((samples[i] + samples[i + 1]) // 2)
                frames = struct.pack(f"<{len(mono_samples)}h", *mono_samples)

            # Yield in chunks
            chunk_size = 32000  # bytes (1 second at 16kHz)
            for i in range(0, len(frames), chunk_size):
                yield frames[i : i + chunk_size]


def convert_mp3_to_pcm(mp3_path):
    """
    Read an MP3 file and send it directly (Rust decoder will handle it)

    Args:
        mp3_path: Path to MP3 file

    Yields:
        MP3 data chunks (will be decoded by the Rust service)
    """
    print(f"Reading MP3 file: {mp3_path}")
    print(f"  Note: MP3 will be decoded by the robot's audio service")

    # Read entire MP3 file
    with open(mp3_path, "rb") as f:
        mp3_data = f.read()

    print(f"  File size: {len(mp3_data)} bytes")

    # Send entire MP3 file in one chunk
    # The Rust audio decoder will handle MP3 decoding
    yield mp3_data


async def stream_microphone(websocket_url, chunk_duration_ms=1000, sample_rate=16000):
    """Stream audio from microphone to robot"""

    if not HAS_PYAUDIO:
        print("Error: PyAudio is not installed. Cannot stream from microphone.")
        print("Install with: pip install pyaudio")
        return

    # Import here since we checked HAS_PYAUDIO
    import pyaudio as pa

    chunk_size = int(sample_rate * chunk_duration_ms / 1000)

    # Initialize PyAudio
    audio = pa.PyAudio()

    print(f"Audio Settings:")
    print(f"  Sample rate: {sample_rate} Hz")
    print(f"  Channels: 1 (mono)")
    print(f"  Chunk duration: {chunk_duration_ms} ms")
    print(f"  Format: 16-bit PCM")

    # Open microphone stream
    stream = audio.open(
        format=pa.paInt16,
        channels=1,
        rate=sample_rate,
        input=True,
        frames_per_buffer=chunk_size,
    )

    try:
        async with websockets.connect(websocket_url) as websocket:
            print(f"\nConnected to {websocket_url}")
            print("Streaming audio... (Press Ctrl+C to stop)\n")

            frame_count = 0

            while True:
                # Read audio chunk from microphone
                pcm_data = stream.read(chunk_size, exception_on_overflow=False)

                # Send PCM data directly as binary
                await websocket.send(pcm_data)

                frame_count += 1
                elapsed_time = frame_count * chunk_duration_ms / 1000

                print(
                    f"[{elapsed_time:6.1f}s] Sent {len(pcm_data):6d} bytes (PCM)",
                    end="\r",
                )

    except websockets.exceptions.ConnectionClosed:
        print("\n\nConnection closed by server")
    except KeyboardInterrupt:
        print("\n\nStopping audio stream...")
    except Exception as e:
        print(f"\n\nError: {e}")
        import traceback

        traceback.print_exc()
    finally:
        stream.stop_stream()
        stream.close()
        audio.terminate()
        print("Audio capture stopped")


async def stream_file(websocket_url, audio_file, realtime=True, chunk_duration_ms=1000):
    """Stream audio file (WAV or MP3) to robot via WebSocket"""

    audio_path = Path(audio_file)

    if not audio_path.exists():
        print(f"Error: File not found: {audio_file}")
        return

    file_ext = audio_path.suffix.lower()

    if file_ext not in [".wav", ".mp3"]:
        print(f"Error: Unsupported file format: {file_ext}")
        print(f"Supported formats: WAV, MP3")
        print(f"To convert other formats:")
        print(f"  ffmpeg -i {audio_file} -ar 16000 -ac 1 output.wav")
        return

    try:
        print(f"Loading audio file: {audio_file}")

        # Determine file type and get duration estimate
        if file_ext == ".wav":
            with wave.open(str(audio_path), "rb") as wav_file:
                framerate = wav_file.getframerate()
                nframes = wav_file.getnframes()
                duration = nframes / framerate
        else:  # MP3
            # For MP3, we'll estimate or just show unknown
            file_size = audio_path.stat().st_size
            # Rough estimate: 128kbps MP3 = ~16KB/s = ~1KB per 62ms
            duration = file_size / 16000  # Very rough estimate
            framerate = 16000  # Assume for display purposes

        print(f"Duration: ~{duration:.2f} seconds")
        print(f"Connecting to {websocket_url}...")

        async with websockets.connect(websocket_url) as websocket:
            print("Connected! Streaming audio to robot...\n")

            chunk_count = 0
            total_bytes = 0

            # Choose converter based on file type
            if file_ext == ".wav":
                converter = convert_wav_to_pcm(audio_path)
            else:  # MP3
                converter = convert_mp3_to_pcm(audio_path)

            for audio_chunk in converter:
                # Send audio data as binary
                await websocket.send(audio_chunk)

                chunk_count += 1
                total_bytes += len(audio_chunk)
                elapsed = (
                    total_bytes / 2 / framerate
                )  # 2 bytes per sample (rough estimate)
                progress = (elapsed / duration) * 100 if duration > 0 else 100

                print(
                    f"[{elapsed:6.1f}s / {duration:6.1f}s] Progress: {progress:5.1f}% | Sent {len(audio_chunk):6d} bytes",
                    end="\r",
                )

                # Simulate real-time streaming
                if realtime and file_ext == ".wav":
                    await asyncio.sleep(chunk_duration_ms / 1000)

            print(f"\n\nFinished streaming {chunk_count} chunks (~{duration:.2f}s)")
            print("Audio sent to robot for playback!")

    except Exception as e:
        print(f"\nError: {e}")
        import traceback

        traceback.print_exc()


def main():
    parser = argparse.ArgumentParser(
        description="Stream audio to Unitree Go2 Robot via WebSocket",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Examples:
  # Stream from microphone to robot (runs ON robot, connects to localhost)
  python3 audio_stream.py
  
  # Stream from laptop to robot at specific IP (robot's IP on network)
  python3 audio_stream.py --host 192.168.12.1 --file audio.mp3
  
  # Or specify full URL
  python3 audio_stream.py --url ws://192.168.12.1:8080 --file audio.mp3
  
  # Stream WAV file
  python3 audio_stream.py --host 192.168.12.1 --file audio.wav
  
  # Stream MP3 file
  python3 audio_stream.py --host 192.168.123.161 --file song.mp3
  
  # Stream file as fast as possible
  python3 audio_stream.py --host 192.168.12.1 --file audio.wav --no-realtime

Supported formats:
  - WAV files (will be converted to optimal format if needed)
  - MP3 files (decoded by robot's audio service)
  - Microphone input (16kHz mono PCM)

Note: The Rust audio service must be running ON the robot:
  ssh unitree@192.168.12.1
  cd ~/rust_websocket_tts
  cargo run --release
        """,
    )

    # Get default URL from environment
    default_url = get_robot_websocket_url()

    parser.add_argument(
        "--url",
        help=f"WebSocket server URL (default: {default_url}, use ROBOT_IP env var to change)",
    )
    parser.add_argument(
        "--host",
        help="Robot IP address (shorthand for --url ws://HOST:8080)",
    )
    parser.add_argument(
        "--file",
        help="Audio file to stream (WAV or MP3). If not provided, uses microphone",
    )
    parser.add_argument(
        "--chunk-duration",
        type=int,
        default=1000,
        help="Audio chunk duration in milliseconds (default: 1000)",
    )
    parser.add_argument(
        "--sample-rate",
        type=int,
        default=16000,
        help="Microphone sample rate in Hz (default: 16000)",
    )
    parser.add_argument(
        "--no-realtime",
        action="store_true",
        help="Stream file as fast as possible (file mode only)",
    )

    args = parser.parse_args()

    # Handle --host parameter as shorthand
    websocket_url = args.url if args.url else default_url
    if args.host:
        websocket_url = f"ws://{args.host}:8080"

    print("=" * 70)
    print("Audio Streaming Client for Unitree Go2 Robot")
    print("=" * 70)
    print(f"Target: {websocket_url}")
    print()

    try:
        if args.file:
            print(f"Mode: File streaming")
            print(f"File: {args.file}")
            print(f"Realtime: {'Yes' if not args.no_realtime else 'No'}")
            print()
            asyncio.run(
                stream_file(
                    websocket_url=websocket_url,
                    audio_file=args.file,
                    realtime=not args.no_realtime,
                    chunk_duration_ms=args.chunk_duration,
                )
            )
        else:
            print(f"Mode: Microphone streaming")
            print()
            asyncio.run(
                stream_microphone(
                    websocket_url=websocket_url,
                    chunk_duration_ms=args.chunk_duration,
                    sample_rate=args.sample_rate,
                )
            )
    except KeyboardInterrupt:
        print("\nInterrupted by user")
        sys.exit(0)
    except Exception as e:
        print(f"\nFatal error: {e}")
        import traceback

        traceback.print_exc()
        sys.exit(1)


if __name__ == "__main__":
    main()

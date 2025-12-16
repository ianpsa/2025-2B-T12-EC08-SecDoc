#!/usr/bin/env python3
"""
Audio format converter for Unitree robot compatibility
Converts audio files to robot-compatible format
"""

import sys
import os
from pydub import AudioSegment


def convert_to_robot_format(input_file, output_file=None):
    """
    Convert audio file to robot-compatible format

    Target specs:
    - Format: WAV
    - Sample rate: 16000 Hz
    - Channels: Mono
    - Bit depth: 16-bit
    """
    if output_file is None:
        base = os.path.splitext(input_file)[0]
        output_file = f"{base}_robot.wav"

    print(f"Loading audio from: {input_file}")

    # Load audio file (supports many formats)
    audio = AudioSegment.from_file(input_file)

    print(f"Original format:")
    print(f"  Sample rate: {audio.frame_rate} Hz")
    print(f"  Channels: {audio.channels}")
    print(f"  Sample width: {audio.sample_width} bytes")
    print(f"  Duration: {len(audio) / 1000:.2f} seconds")

    # Convert to robot format
    print("\nConverting to robot format...")
    audio = audio.set_frame_rate(16000)  # 16kHz
    audio = audio.set_channels(1)  # Mono
    audio = audio.set_sample_width(2)  # 16-bit

    # Export as WAV
    print(f"Saving to: {output_file}")
    audio.export(output_file, format="wav")

    # Verify output
    converted = AudioSegment.from_wav(output_file)
    print(f"\nConverted format:")
    print(f"  Sample rate: {converted.frame_rate} Hz")
    print(f"  Channels: {converted.channels}")
    print(f"  Sample width: {converted.sample_width} bytes")
    print(f"  Duration: {len(converted) / 1000:.2f} seconds")
    print(f"  File size: {os.path.getsize(output_file)} bytes")

    print(f"\n✓ Conversion complete!")
    print(f"  Output: {output_file}")
    return output_file


def main():
    if len(sys.argv) < 2:
        print("Usage: python convert_audio_format.py <input_file> [output_file]")
        print("\nExample:")
        print("  python convert_audio_format.py my_audio.mp3")
        print("  python convert_audio_format.py my_audio.mp3 robot_audio.wav")
        print("\nSupported input formats: mp3, wav, ogg, flac, m4a, etc.")
        sys.exit(1)

    input_file = sys.argv[1]
    output_file = sys.argv[2] if len(sys.argv) > 2 else None

    if not os.path.exists(input_file):
        print(f"Error: File not found: {input_file}")
        sys.exit(1)

    try:
        result = convert_to_robot_format(input_file, output_file)
        print(f"\nYou can now send this file to the robot:")
        print(
            f"  python test_websocket_client.py ws://192.168.123.161:8765 {result} wav"
        )
    except Exception as e:
        print(f"Error: {e}")
        sys.exit(1)


if __name__ == "__main__":
    main()

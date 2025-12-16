#!/usr/bin/env python3
"""
Quick diagnostic script to test robot audio playback
Run this to verify the robot's speaker is working
"""

import asyncio
import sys
from unitree_webrtc_connect.webrtc_driver import (
    UnitreeWebRTCConnection,
    WebRTCConnectionMethod,
)
from unitree_webrtc_connect.webrtc_audiohub import WebRTCAudioHub
import logging
import json

logging.basicConfig(level=logging.INFO)
logger = logging.getLogger(__name__)


async def test_audio_playback(robot_ip="192.168.123.161"):
    """
    Test audio playback on the robot
    """
    try:
        logger.info(f"Connecting to robot at {robot_ip}")
        conn = UnitreeWebRTCConnection(WebRTCConnectionMethod.LocalSTA, ip=robot_ip)
        await conn.connect()
        logger.info("Connected!")

        audio_hub = WebRTCAudioHub(conn, logger)
        logger.info("Audio hub initialized")

        # Get audio list
        logger.info("Fetching audio list from robot...")
        response = await audio_hub.get_audio_list()

        if response and isinstance(response, dict):
            data_str = response.get("data", {}).get("data", "{}")
            audio_list = json.loads(data_str).get("audio_list", [])

            logger.info(f"\nFound {len(audio_list)} audio files on robot:")
            for audio in audio_list:
                logger.info(f"  - {audio['CUSTOM_NAME']} (UUID: {audio['UNIQUE_ID']})")

            if audio_list:
                # Try playing the first audio file
                first_audio = audio_list[0]
                uuid = first_audio["UNIQUE_ID"]
                name = first_audio["CUSTOM_NAME"]

                logger.info(f"\nAttempting to play '{name}' (UUID: {uuid})")
                logger.info("*** LISTEN FOR AUDIO NOW ***")

                await audio_hub.play_by_uuid(uuid)
                logger.info("Playback command sent successfully")

                # Wait to allow audio to play
                logger.info("Waiting 5 seconds... (listen for audio)")
                await asyncio.sleep(5)

                logger.info("\nTest complete!")
                logger.info("\nDid you hear audio?")
                logger.info(
                    "  YES -> Robot speaker is working, check your uploaded audio format"
                )
                logger.info(
                    "  NO  -> Check robot volume in Unitree app or speaker hardware"
                )
            else:
                logger.warning("No audio files found on robot!")
                logger.info("Try uploading an audio file first")
        else:
            logger.error("Failed to get audio list from robot")

    except Exception as e:
        logger.error(f"Error: {e}", exc_info=True)


if __name__ == "__main__":
    robot_ip = sys.argv[1] if len(sys.argv) > 1 else "192.168.123.161"

    print("=" * 60)
    print("Robot Audio Diagnostic Test")
    print("=" * 60)
    print(f"Robot IP: {robot_ip}")
    print("\nThis script will:")
    print("1. Connect to the robot")
    print("2. List available audio files")
    print("3. Play the first audio file")
    print("\nMake sure you can hear the robot's speaker!")
    print("=" * 60)
    print()

    try:
        asyncio.run(test_audio_playback(robot_ip))
    except KeyboardInterrupt:
        print("\n\nTest interrupted by user")

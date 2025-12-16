import logging
import asyncio
import json
import base64
import tempfile
import os
import sys
import shutil
import websockets
from pydub import AudioSegment
from unitree_webrtc_connect.webrtc_driver import (
    UnitreeWebRTCConnection,
    WebRTCConnectionMethod,
)
from unitree_webrtc_connect.webrtc_audiohub import WebRTCAudioHub

# Configure logging
logging.basicConfig(level=logging.INFO)
logger = logging.getLogger(__name__)


def check_ffmpeg():
    """Check if ffmpeg is installed"""
    ffmpeg_path = shutil.which("ffmpeg")
    ffprobe_path = shutil.which("ffprobe")

    if not ffmpeg_path or not ffprobe_path:
        logger.error("=" * 70)
        logger.error("❌ FFMPEG NOT FOUND")
        logger.error("=" * 70)
        logger.error("FFmpeg is required to convert MP3 audio to WAV format.")
        logger.error("\nTo install FFmpeg:")
        logger.error("  Ubuntu/Debian: sudo apt-get install ffmpeg")
        logger.error("  CentOS/RHEL:   sudo yum install ffmpeg")
        logger.error("  macOS:         brew install ffmpeg")
        logger.error("  Windows:       Download from https://ffmpeg.org/download.html")
        logger.error("=" * 70)
        return False

    logger.info(f"✅ FFmpeg found: {ffmpeg_path}")
    logger.info(f"✅ FFprobe found: {ffprobe_path}")
    return True


class WebSocketAudioStreamer:
    def __init__(self, robot_ip: str, websocket_url: str, retry_interval: float = 5.0):
        """
        Initialize WebSocket Audio Streamer

        Args:
            robot_ip: IP address of the Unitree robot
            websocket_url: WebSocket server URL to connect to (e.g., ws://server:8765)
            retry_interval: Seconds to wait between reconnection attempts (default: 5.0)
        """
        self.robot_ip = robot_ip
        self.websocket_url = websocket_url
        self.retry_interval = retry_interval
        self.webrtc_conn = None
        self.audio_hub = None
        self.temp_dir = tempfile.mkdtemp()
        self.is_playing = False
        self.should_stop = False
        logger.info(f"Temporary directory created: {self.temp_dir}")

    async def initialize_robot_connection(self):
        """Establish WebRTC connection with the robot"""
        try:
            logger.info(f"Connecting to robot at {self.robot_ip}")
            self.webrtc_conn = UnitreeWebRTCConnection(
                WebRTCConnectionMethod.LocalSTA, ip=self.robot_ip
            )
            await self.webrtc_conn.connect()
            logger.info("WebRTC connection established")

            # Initialize audio hub
            self.audio_hub = WebRTCAudioHub(self.webrtc_conn, logger)
            logger.info("Audio hub initialized")
        except Exception as e:
            logger.error(f"Failed to connect to robot: {e}")
            raise

    async def convert_and_play_audio(
        self, audio_data_b64: str, audio_format: str = "mp3"
    ):
        """
        Convert MP3/WAV to robot-compatible format and play

        Args:
            audio_data_b64: Base64 encoded audio data
            audio_format: Audio format (mp3 or wav, default: mp3)
        """
        if self.is_playing:
            logger.warning("Audio is already playing, skipping this request")
            return

        self.is_playing = True
        try:
            # Decode base64 audio
            logger.info(f"Decoding base64 {audio_format} audio data")
            audio_bytes = base64.b64decode(audio_data_b64)
            logger.info(f"Decoded audio size: {len(audio_bytes)} bytes")

            # Create temporary file for input
            input_filename = (
                f"input_{int(asyncio.get_event_loop().time())}.{audio_format}"
            )
            input_filepath = os.path.join(self.temp_dir, input_filename)

            # Write audio to temporary file
            with open(input_filepath, "wb") as f:
                f.write(audio_bytes)
            logger.info(f"Audio saved to temporary file: {input_filepath}")

            # Convert to robot-compatible format (16kHz, 16-bit, mono WAV)
            output_filename = f"audio_{int(asyncio.get_event_loop().time())}.wav"
            output_filepath = os.path.join(self.temp_dir, output_filename)

            # Try to convert if pydub/ffmpeg is available
            try:
                logger.info(
                    "Converting audio to robot-compatible format (16kHz mono WAV)"
                )

                if audio_format.lower() == "mp3":
                    audio = AudioSegment.from_mp3(input_filepath)
                else:
                    audio = AudioSegment.from_wav(input_filepath)

                # Convert to 16kHz, mono, 16-bit
                audio = audio.set_frame_rate(16000)
                audio = audio.set_channels(1)
                audio = audio.set_sample_width(2)  # 16-bit

                # Export to WAV
                audio.export(
                    output_filepath,
                    format="wav",
                    parameters=["-ar", "16000", "-ac", "1"],
                )
                logger.info(f"✅ Converted audio saved to: {output_filepath}")

            except FileNotFoundError as e:
                # FFmpeg not found - use input file directly if it's WAV
                logger.warning(f"⚠️  FFmpeg not found: {e}")
                if audio_format.lower() == "wav":
                    logger.info(
                        "📋 Using input WAV file directly (assuming it's already 16kHz mono)"
                    )
                    output_filepath = input_filepath
                    output_filename = input_filename
                else:
                    logger.error("❌ Cannot convert MP3 without FFmpeg!")
                    logger.error(
                        "   Please install FFmpeg: sudo apt-get install ffmpeg"
                    )
                    logger.error("   OR send pre-converted 16kHz mono WAV files")
                    raise Exception("FFmpeg required for MP3 conversion")

            except Exception as e:
                logger.error(f"❌ Error converting audio: {e}")
                # Try using the file directly if it's WAV
                if audio_format.lower() == "wav":
                    logger.warning(
                        "⚠️  Conversion failed, trying to use WAV file directly"
                    )
                    output_filepath = input_filepath
                    output_filename = input_filename
                else:
                    raise

            # Upload and play audio
            logger.info("Uploading audio to robot...")
            await self.audio_hub.upload_audio_file(output_filepath)
            logger.info("Audio uploaded successfully")

            # Wait for upload to settle
            logger.info("Waiting for upload to settle...")
            await asyncio.sleep(1.0)

            # Get the UUID of the uploaded file
            logger.info("Fetching audio list from robot...")
            response = await self.audio_hub.get_audio_list()

            if response and isinstance(response, dict):
                data_str = response.get("data", {}).get("data", "{}")
                audio_list = json.loads(data_str).get("audio_list", [])
                logger.info(f"Found {len(audio_list)} audio files on robot")

                # Get the filename without extension
                filename = os.path.splitext(output_filename)[0]
                logger.info(f"Looking for audio with name: {filename}")

                # Find the uploaded audio
                existing_audio = next(
                    (audio for audio in audio_list if audio["CUSTOM_NAME"] == filename),
                    None,
                )

                if existing_audio:
                    uuid = existing_audio["UNIQUE_ID"]
                    logger.info(f"Found audio with UUID: {uuid}")
                    logger.info("Starting audio playback...")

                    # Play the audio
                    response = await self.audio_hub.play_by_uuid(uuid)
                    logger.info(f"Playback command sent, response: {response}")

                    # Wait for playback to start
                    await asyncio.sleep(0.5)
                    logger.info("Audio playback started")

                    # Note: Check robot volume in Unitree app if no sound is heard
                    # Settings → Volume should be >= 50%
                else:
                    error_msg = f"Could not find uploaded audio '{filename}' in list"
                    logger.error(error_msg)
                    logger.info(
                        f"Available audio files: {[a['CUSTOM_NAME'] for a in audio_list]}"
                    )
                    raise Exception(error_msg)
            else:
                error_msg = "Failed to get audio list from robot"
                logger.error(error_msg)
                raise Exception(error_msg)

            # Cleanup temporary files
            try:
                os.remove(input_filepath)
                os.remove(output_filepath)
                logger.info("Temporary files cleaned up")
            except Exception as e:
                logger.warning(f"Failed to remove temporary files: {e}")

        except Exception as e:
            logger.error(f"Error playing audio: {e}")
            raise
        finally:
            self.is_playing = False

    async def connect_and_listen(self):
        """Connect to WebSocket server and listen for audio data with auto-reconnect"""
        attempt = 0

        while not self.should_stop:
            attempt += 1
            try:
                logger.info(
                    f"[Attempt {attempt}] Connecting to WebSocket server: {self.websocket_url}"
                )

                async with websockets.connect(self.websocket_url) as websocket:
                    logger.info("✅ Connected to WebSocket server")
                    logger.info("⏳ Waiting for audio data...")
                    attempt = 0  # Reset attempt counter on successful connection

                    async for message in websocket:
                        try:
                            # Parse JSON message
                            data = json.loads(message)

                            # Extract audio data and format
                            audio_b64 = data.get("audio")
                            audio_format = data.get("format", "mp3")

                            if not audio_b64:
                                error_msg = "Missing 'audio' field in message"
                                logger.error(error_msg)
                                continue

                            logger.info(
                                f"📥 Received audio data (format: {audio_format})"
                            )

                            # Convert and play the audio
                            await self.convert_and_play_audio(audio_b64, audio_format)

                            logger.info("✅ Audio playback completed")

                        except json.JSONDecodeError as e:
                            logger.error(f"Invalid JSON: {e}")
                        except Exception as e:
                            logger.error(f"Error processing audio: {e}")

            except websockets.exceptions.ConnectionClosed:
                logger.warning(f"⚠️  WebSocket connection closed")
            except websockets.exceptions.WebSocketException as e:
                logger.warning(f"⚠️  WebSocket error: {e}")
            except ConnectionRefusedError:
                logger.warning(f"⚠️  Connection refused to {self.websocket_url}")
            except OSError as e:
                logger.warning(f"⚠️  Network error: {e}")
            except Exception as e:
                logger.error(f"❌ Unexpected error: {e}")

            # Only retry if not stopped
            if not self.should_stop:
                logger.info(f"🔄 Reconnecting in {self.retry_interval} seconds...")
                await asyncio.sleep(self.retry_interval)

    async def run(self):
        """Main entry point"""
        try:
            # Initialize robot connection
            await self.initialize_robot_connection()

            # Connect to WebSocket server and listen
            await self.connect_and_listen()

        except KeyboardInterrupt:
            logger.info("Stopped by user")
        except Exception as e:
            logger.error(f"Error: {e}")
        finally:
            # Cleanup
            try:
                import shutil

                shutil.rmtree(self.temp_dir)
                logger.info(f"Cleaned up temporary directory: {self.temp_dir}")
            except Exception as e:
                logger.warning(f"Failed to clean up temporary directory: {e}")


async def main():
    # Configuration - can be overridden via command line arguments
    if len(sys.argv) < 3:
        print("Usage: python websocket_audio_receiver.py <robot_ip> <websocket_url>")
        print(
            "Example: python websocket_audio_receiver.py 192.168.123.161 ws://server:8765"
        )
        sys.exit(1)

    ROBOT_IP = sys.argv[1]
    WEBSOCKET_URL = sys.argv[2]

    logger.info(f"Configuration: Robot IP={ROBOT_IP}, WebSocket URL={WEBSOCKET_URL}")

    # Create and run the streamer
    streamer = WebSocketAudioStreamer(ROBOT_IP, WEBSOCKET_URL)
    await streamer.run()


if __name__ == "__main__":
    try:
        asyncio.run(main())
    except KeyboardInterrupt:
        logger.info("\nProgram interrupted by user")

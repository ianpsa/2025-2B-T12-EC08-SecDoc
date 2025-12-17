# Rust Audio Player

Fast WebSocket audio receiver for Unitree robot.

## Build

```bash
cargo build --release
```

## Run

```bash
./target/release/rust_audio_player <robot_ip> <websocket_url>
```

Example:
```bash
./target/release/rust_audio_player 192.168.123.161 ws://10.8.250.17:8765
```

## Requirements

- FFmpeg installed
- Python 3 with `unitree_webrtc_connect` and `pydub`
- `play_audio.py` in current directory

## Architecture

```
WebSocket (remote) -> Rust (decode + convert) -> Python (play on robot)
```




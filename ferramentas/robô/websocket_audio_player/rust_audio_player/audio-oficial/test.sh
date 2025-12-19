#!/bin/bash
# Convert all audio files to WAV format (16kHz mono) for robot playback
# Skips files that are already WAV

AUDIO_DIR="$(dirname "$0")"
cd "$AUDIO_DIR" || exit 1

echo "🎵 Converting audio files to WAV (16kHz mono)..."
echo "   Directory: $AUDIO_DIR"
echo ""

converted=0
skipped=0
failed=0

for file in *; do
    # Skip if not a file
    [ -f "$file" ] || continue
    
    # Get extension
    ext="${file##*.}"
    name="${file%.*}"
    
    # Skip if already WAV
    if [ "$ext" = "wav" ]; then
        echo "⏭️  Skipping (already WAV): $file"
        ((skipped++))
        continue
    fi
    
    # Skip non-audio files
    case "$ext" in
        mp3|ogg|m4a|flac|aac|wma|opus)
            ;;
        *)
            continue
            ;;
    esac
    
    output="${name}.wav"
    
    # Skip if WAV version already exists
    if [ -f "$output" ]; then
        echo "⏭️  Skipping (WAV exists): $file -> $output"
        ((skipped++))
        continue
    fi
    
    echo "🔄 Converting: $file -> $output"
    
    if ffmpeg -y -hide_banner -loglevel error \
        -i "$file" \
        -ar 16000 \
        -ac 1 \
        -acodec pcm_s16le \
        "$output" 2>/dev/null; then
        echo "   ✅ Done"
        ((converted++))
    else
        echo "   ❌ Failed"
        ((failed++))
    fi
done

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "✅ Converted: $converted"
echo "⏭️  Skipped:   $skipped"
echo "❌ Failed:    $failed"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"


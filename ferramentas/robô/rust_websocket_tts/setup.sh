#!/bin/bash
# Setup script for the Rust WebSocket Audio Player

set -e

echo "=== Rust WebSocket Audio Player Setup ==="
echo ""

# Check for required system dependencies
echo "Checking system dependencies..."

if [[ "$OSTYPE" == "linux-gnu"* ]]; then
    echo "Detected Linux system"
    
    # Check for ALSA development libraries
    if ! pkg-config --exists alsa; then
        echo "⚠ ALSA development libraries not found"
        echo ""
        echo "Please install them first:"
        echo "  Ubuntu/Debian: sudo apt-get install libasound2-dev pkg-config"
        echo "  Fedora: sudo dnf install alsa-lib-devel"
        echo ""
        read -p "Continue anyway? (y/N) " -n 1 -r
        echo
        if [[ ! $REPLY =~ ^[Yy]$ ]]; then
            exit 1
        fi
    else
        echo "✓ ALSA libraries found"
    fi
elif [[ "$OSTYPE" == "darwin"* ]]; then
    echo "✓ Detected macOS - no additional dependencies needed"
fi

echo ""

# Install Python dependencies
echo "Installing Python client dependencies..."
if command -v pip3 &> /dev/null; then
    pip3 install -r requirements.txt
    echo "✓ Python dependencies installed"
else
    echo "⚠ pip3 not found, skipping Python dependencies"
    echo "  Install manually: pip3 install -r requirements.txt"
fi
echo ""

# Build the Rust project
echo "Building Rust WebSocket Audio Player..."
cargo build --release

if [ $? -eq 0 ]; then
    echo ""
    echo "=== Build Successful! ==="
    echo ""
    echo "To run the server:"
    echo "  cargo run --release"
    echo ""
    echo "Or run the binary directly:"
    echo "  ./target/release/rust_websocket_tts"
    echo ""
    echo "To test with the example client:"
    echo "  python3 client_example.py --test"
    echo ""
    echo "Configuration (optional environment variable):"
    echo "  WS_ADDRESS=0.0.0.0:8080        # WebSocket listen address"
    echo ""
else
    echo ""
    echo "=== Build Failed ==="
    echo "Please check the errors above and fix them."
    exit 1
fi

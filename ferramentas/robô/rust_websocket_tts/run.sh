#!/bin/bash
# Run script that sources ROS2 before running
# Usage: ./run.sh

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

echo "========================================"
echo "Rust WebSocket → ROS2 Audio Bridge"
echo "Run Script"
echo "========================================"
echo ""

# Detect ROS2
if [ ! -d "/opt/ros" ]; then
    echo -e "${RED}ERROR: ROS2 not found${NC}"
    exit 1
fi

# Find ROS2 distro
if [ -f "/opt/ros/humble/setup.bash" ]; then
    ROS_DISTRO="humble"
elif [ -f "/opt/ros/iron/setup.bash" ]; then
    ROS_DISTRO="iron"
elif [ -f "/opt/ros/foxy/setup.bash" ]; then
    ROS_DISTRO="foxy"
else
    ROS_DISTRO=$(ls /opt/ros | head -1)
fi

echo -e "${GREEN}✓ Using ROS2 ${ROS_DISTRO}${NC}"

# Source ROS2
source "/opt/ros/${ROS_DISTRO}/setup.bash"

# Check for WebSocket URL
if [ -z "$WS_URL" ]; then
    echo -e "${YELLOW}⚠️  WS_URL not set, using default: ws://localhost:8080/v1/audio${NC}"
    echo "   To change: export WS_URL=ws://your-server:port/path"
    echo ""
fi

# Run
echo "Starting application..."
echo ""

cargo run --release "$@"

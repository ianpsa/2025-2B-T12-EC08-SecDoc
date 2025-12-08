#!/bin/bash
# Build script that sources ROS2 before building
# Usage: ./build.sh

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo "========================================"
echo "Rust WebSocket → ROS2 Audio Bridge"
echo "Build Script"
echo "========================================"
echo ""

# Check if ROS2 is installed
if [ ! -d "/opt/ros" ]; then
    echo -e "${RED}ERROR: ROS2 not found in /opt/ros${NC}"
    echo "Please install ROS2 first:"
    echo "  - ROS2 Humble: https://docs.ros.org/en/humble/Installation.html"
    echo "  - ROS2 Iron: https://docs.ros.org/en/iron/Installation.html"
    exit 1
fi

# Detect ROS2 distribution
echo "Detecting ROS2 distribution..."
if [ -f "/opt/ros/humble/setup.bash" ]; then
    ROS_DISTRO="humble"
elif [ -f "/opt/ros/iron/setup.bash" ]; then
    ROS_DISTRO="iron"
elif [ -f "/opt/ros/foxy/setup.bash" ]; then
    ROS_DISTRO="foxy"
else
    # Try to find any ROS2 distro
    ROS_DISTRO=$(ls /opt/ros | head -1)
    if [ -z "$ROS_DISTRO" ]; then
        echo -e "${RED}ERROR: Could not detect ROS2 distribution${NC}"
        exit 1
    fi
fi

echo -e "${GREEN}✓ Found ROS2 ${ROS_DISTRO}${NC}"
echo ""

# Source ROS2
echo "Sourcing ROS2 environment..."
source "/opt/ros/${ROS_DISTRO}/setup.bash"

if [ -z "$ROS_DISTRO" ]; then
    echo -e "${RED}ERROR: Failed to source ROS2${NC}"
    exit 1
fi

echo -e "${GREEN}✓ ROS2 environment sourced${NC}"
echo "  ROS_DISTRO: $ROS_DISTRO"
echo "  AMENT_PREFIX_PATH: $AMENT_PREFIX_PATH"
echo ""

# Build the project
echo "Building Rust project..."
echo ""

cargo build "$@"

BUILD_STATUS=$?

echo ""
if [ $BUILD_STATUS -eq 0 ]; then
    echo -e "${GREEN}========================================"
    echo -e "✓ Build successful!${NC}"
    echo "========================================"
    echo ""
    echo "To run:"
    echo "  1. Source ROS2: source /opt/ros/${ROS_DISTRO}/setup.bash"
    echo "  2. Set WebSocket URL (optional): export WS_URL=ws://your-server:8080/v1/audio"
    echo "  3. Run: cargo run"
    echo ""
    echo "Or use the run script:"
    echo "  ./run.sh"
else
    echo -e "${RED}========================================"
    echo -e "✗ Build failed${NC}"
    echo "========================================"
    echo ""
    echo "Check the errors above and:"
    echo "  1. Make sure ROS2 is properly installed"
    echo "  2. Run: source /opt/ros/${ROS_DISTRO}/setup.bash"
    echo "  3. Try again: ./build.sh"
fi

exit $BUILD_STATUS

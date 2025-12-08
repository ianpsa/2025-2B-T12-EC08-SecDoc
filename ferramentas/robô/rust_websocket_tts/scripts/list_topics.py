#!/usr/bin/env python3
"""
ROS2 Topic Discovery Script for Unitree GO2

Purpose: Discover all ROS2 topics on the Unitree GO2, especially audio-related ones.

Usage:
    # On your development machine (with ROS2 installed):
    python3 list_topics.py

    # Or on the Unitree GO2:
    chmod +x list_topics.py
    ./list_topics.py

Requirements:
    - ROS2 installed (Humble, Iron, or compatible)
    - rclpy Python package: pip3 install rclpy
    - Source ROS2 setup: source /opt/ros/<distro>/setup.bash
"""

import sys

try:
    import rclpy
    from rclpy.node import Node
except ImportError:
    print("ERROR: rclpy not found!")
    print("Install with: pip3 install rclpy")
    print("Or source ROS2: source /opt/ros/<distro>/setup.bash")
    sys.exit(1)


def main():
    """Discover and list all ROS2 topics with focus on audio topics."""

    # Initialize ROS2
    try:
        rclpy.init()
    except Exception as e:
        print(f"ERROR: Failed to initialize ROS2: {e}")
        print("Make sure ROS2 is installed and sourced.")
        sys.exit(1)

    # Create a node
    node = Node("topic_lister")

    print("=" * 60)
    print("ROS2 Topic Discovery for Unitree GO2")
    print("=" * 60)
    print()

    # Get all topics
    try:
        topics = node.get_topic_names_and_types()
    except Exception as e:
        print(f"ERROR: Failed to get topics: {e}")
        node.destroy_node()
        rclpy.shutdown()
        sys.exit(1)

    # Categorize topics
    audio_topics = []
    unitree_topics = []
    other_topics = []

    for topic_name, topic_types in topics:
        # Check if it's audio-related
        if "audio" in topic_name.lower():
            audio_topics.append((topic_name, topic_types))
        # Check if it's unitree-related
        elif "unitree" in topic_name.lower() or topic_name.startswith("/"):
            unitree_topics.append((topic_name, topic_types))
        else:
            other_topics.append((topic_name, topic_types))

    # Display results
    total_topics = len(topics)
    print(f"Total topics found: {total_topics}")
    print()

    # Audio topics (most important!)
    if audio_topics:
        print("=" * 60)
        print("🔊 AUDIO TOPICS FOUND (Use these!)")
        print("=" * 60)
        for topic, types in audio_topics:
            print(f"  ✓ Topic: {topic}")
            for t in types:
                print(f"    └─ Type: {t}")
        print()
    else:
        print("=" * 60)
        print("⚠️  NO AUDIO TOPICS FOUND")
        print("=" * 60)
        print("Possible reasons:")
        print("  1. Audio system not running on the robot")
        print("  2. Different topic naming convention")
        print("  3. Need to check Unitree documentation")
        print()
        print("Suggested topic names to try:")
        print("  - audiodata")
        print("  - audio")
        print("  - /audio_play")
        print("  - /unitree/audio")
        print()

    # Unitree topics
    if unitree_topics:
        print("=" * 60)
        print("🤖 UNITREE TOPICS")
        print("=" * 60)
        for topic, types in unitree_topics[:10]:  # Show first 10
            print(f"  📡 {topic}")
            for t in types:
                print(f"     └─ {t}")
        if len(unitree_topics) > 10:
            print(f"  ... and {len(unitree_topics) - 10} more")
        print()

    # Other topics (condensed)
    if other_topics:
        print(f"Other topics: {len(other_topics)} found")
        print()

    # Summary and recommendations
    print("=" * 60)
    print("📝 RECOMMENDATIONS")
    print("=" * 60)

    if audio_topics:
        print(f"✓ Use topic: '{audio_topics[0][0]}' in your Rust code")
        print(f"  Update this line in websocket_handler.rs:")
        print(f'    "audiodata"  →  "{audio_topics[0][0]}"')
    else:
        print("⚠️  No audio topics detected.")
        print("   Try these troubleshooting steps:")
        print("   1. Check if audio node is running: ros2 node list")
        print("   2. Check Unitree GO2 documentation for audio topic name")
        print("   3. Try publishing to common names and see what works:")
        print("      - ros2 topic pub /audiodata ...")
        print("      - ros2 topic pub /audio ...")

    print()
    print("=" * 60)
    print("To see live data on a topic:")
    print("  ros2 topic echo /topic_name")
    print()
    print("To publish test data:")
    print("  ros2 topic pub /topic_name std_msgs/msg/ByteMultiArray ...")
    print("=" * 60)

    # Cleanup
    node.destroy_node()
    rclpy.shutdown()


if __name__ == "__main__":
    main()

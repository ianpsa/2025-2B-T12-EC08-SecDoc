#!/usr/bin/env python3
"""
Test robot connectivity and find the correct IP address
"""

import socket
import requests
import json
import argparse
from concurrent.futures import ThreadPoolExecutor, as_completed

# Common IPs for Unitree Go2 robot
COMMON_IPS = [
    "192.168.12.1",  # LocalAP mode (direct WiFi connection)
    "192.168.123.161",  # LocalSTA mode (default network)
    "192.168.0.189",  # Your current setting
    "192.168.1.1",  # Common router IP
]


def test_ping(ip):
    """Test if IP is reachable via ping"""
    import subprocess

    try:
        result = subprocess.run(
            ["ping", "-c", "1", "-W", "1", ip],
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
            timeout=2,
        )
        return result.returncode == 0
    except:
        return False


def test_port(ip, port, timeout=2.0):
    """Test if port is open"""
    try:
        sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        sock.settimeout(float(timeout))
        result = sock.connect_ex((ip, port))
        sock.close()
        return result == 0
    except:
        return False


def test_webrtc_endpoint(ip, timeout=3):
    """Test if WebRTC endpoint responds"""
    try:
        # Test old method endpoint
        url = f"http://{ip}:8081/offer"
        response = requests.post(
            url,
            json={"id": "", "sdp": "test", "type": "offer", "token": ""},
            timeout=timeout,
        )
        return {
            "endpoint": "/offer",
            "status": response.status_code,
            "success": True,
            "body": response.text[:100] if response.text else "",
        }
    except requests.exceptions.Timeout:
        return {"endpoint": "/offer", "success": False, "error": "Timeout"}
    except requests.exceptions.ConnectionError as e:
        return {
            "endpoint": "/offer",
            "success": False,
            "error": f"Connection error: {e}",
        }
    except Exception as e:
        return {"endpoint": "/offer", "success": False, "error": str(e)}


def test_new_method_endpoint(ip, timeout=3):
    """Test if new method endpoint (port 9991) responds"""
    try:
        url = f"http://{ip}:9991/con_notify"
        response = requests.get(url, timeout=timeout)
        return {
            "endpoint": "/con_notify",
            "port": 9991,
            "status": response.status_code,
            "success": True,
            "body": response.text[:100] if response.text else "",
        }
    except:
        return {"endpoint": "/con_notify", "port": 9991, "success": False}


def comprehensive_test(ip):
    """Run all tests for a given IP"""
    print(f"\n{'=' * 60}")
    print(f"Testing IP: {ip}")
    print(f"{'=' * 60}")

    results = {
        "ip": ip,
        "reachable": False,
        "port_8081": False,
        "port_9991": False,
        "webrtc_old": None,
        "webrtc_new": None,
    }

    # Test 1: Ping
    print(f"[1/5] Testing ping...")
    results["reachable"] = test_ping(ip)
    if results["reachable"]:
        print(f"      ✓ IP is reachable")
    else:
        print(f"      ✗ IP is NOT reachable")
        return results

    # Test 2: Port 8081
    print(f"[2/5] Testing port 8081...")
    results["port_8081"] = test_port(ip, 8081)
    if results["port_8081"]:
        print(f"      ✓ Port 8081 is open")
    else:
        print(f"      ✗ Port 8081 is closed/filtered")

    # Test 3: Port 9991
    print(f"[3/5] Testing port 9991...")
    results["port_9991"] = test_port(ip, 9991)
    if results["port_9991"]:
        print(f"      ✓ Port 9991 is open")
    else:
        print(f"      ✗ Port 9991 is closed/filtered")

    # Test 4: WebRTC old method
    if results["port_8081"]:
        print(f"[4/5] Testing WebRTC endpoint (old method: /offer)...")
        results["webrtc_old"] = test_webrtc_endpoint(ip)
        if results["webrtc_old"]["success"]:
            print(f"      ✓ Endpoint responds (HTTP {results['webrtc_old']['status']})")
            if results["webrtc_old"]["body"]:
                print(f"      Response: {results['webrtc_old']['body']}")
        else:
            print(
                f"      ✗ Endpoint error: {results['webrtc_old'].get('error', 'Unknown')}"
            )
    else:
        print(f"[4/5] Skipping WebRTC test (port 8081 closed)")

    # Test 5: WebRTC new method
    if results["port_9991"]:
        print(f"[5/5] Testing WebRTC endpoint (new method: /con_notify)...")
        results["webrtc_new"] = test_new_method_endpoint(ip)
        if results["webrtc_new"]["success"]:
            print(f"      ✓ Endpoint responds (HTTP {results['webrtc_new']['status']})")
        else:
            print(f"      ✗ Endpoint not accessible")
    else:
        print(f"[5/5] Skipping new method test (port 9991 closed)")

    return results


def scan_subnet(base_ip):
    """Scan a subnet for responsive hosts on port 8081"""
    print(f"\nScanning subnet {base_ip}.0/24 for port 8081...")
    print("This may take a minute...")

    found = []

    def check_host(i):
        ip = f"{base_ip}.{i}"
        if test_port(ip, 8081, timeout=0.5):
            return ip
        return None

    with ThreadPoolExecutor(max_workers=50) as executor:
        futures = [executor.submit(check_host, i) for i in range(1, 255)]
        for future in as_completed(futures):
            result = future.result()
            if result:
                found.append(result)
                print(f"  Found: {result}")

    return found


def main():
    parser = argparse.ArgumentParser(description="Test Unitree Go2 robot connectivity")
    parser.add_argument("--ip", help="Specific IP to test")
    parser.add_argument(
        "--scan", help="Scan subnet (e.g., 192.168.0)", action="store_true"
    )
    parser.add_argument(
        "--subnet", help="Subnet to scan (e.g., 192.168.0)", default="192.168.0"
    )
    args = parser.parse_args()

    print("=" * 60)
    print("Unitree Go2 Robot Connection Tester")
    print("=" * 60)

    if args.ip:
        # Test specific IP
        result = comprehensive_test(args.ip)

    elif args.scan:
        # Scan subnet
        found_ips = scan_subnet(args.subnet)
        if found_ips:
            print(f"\nFound {len(found_ips)} host(s) with port 8081 open")
            print("\nTesting found hosts in detail...")
            for ip in found_ips:
                comprehensive_test(ip)
        else:
            print("\nNo hosts found with port 8081 open")
    else:
        # Test common IPs
        print("\nTesting common robot IP addresses...\n")

        results = []
        for ip in COMMON_IPS:
            result = comprehensive_test(ip)
            results.append(result)

        # Summary
        print(f"\n{'=' * 60}")
        print("SUMMARY")
        print(f"{'=' * 60}")

        working_ips = []
        for result in results:
            if result["webrtc_old"] and result["webrtc_old"]["success"]:
                working_ips.append(result["ip"])
                print(f"✓ {result['ip']} - WebRTC accessible (old method)")
            elif result["webrtc_new"] and result["webrtc_new"]["success"]:
                working_ips.append(result["ip"])
                print(f"✓ {result['ip']} - WebRTC accessible (new method)")
            elif result["reachable"]:
                print(f"⚠ {result['ip']} - Reachable but WebRTC not responding")
            else:
                print(f"✗ {result['ip']} - Not reachable")

        if working_ips:
            print(f"\n{'=' * 60}")
            print("RECOMMENDED ACTION:")
            print(f"{'=' * 60}")
            print(f"Use this IP address: {working_ips[0]}")
            print(f"\nRun the service with:")
            print(f"  ROBOT_IP={working_ips[0]} cargo run")
        else:
            print(f"\n{'=' * 60}")
            print("NO WORKING CONNECTION FOUND")
            print(f"{'=' * 60}")
            print("\nPossible issues:")
            print("  1. Robot is not powered on")
            print("  2. Robot is not connected to the network")
            print("  3. Wrong network - try connecting to robot's WiFi hotspot")
            print("  4. Firewall blocking connection")
            print("\nTry:")
            print("  • Connect to robot's WiFi hotspot (usually 'Unitree_Go2XXXXX')")
            print("  • Then run: python3 test_robot_connection.py --ip 192.168.12.1")
            print(
                "  • Or scan your network: python3 test_robot_connection.py --scan --subnet 192.168.1"
            )


if __name__ == "__main__":
    main()

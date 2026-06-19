#!/usr/bin/env python3
"""
Minimal mDNS responder for agri-server.local.
Listens on 224.0.0.251:5353, responds to A queries for agri-server.local.
Designed to work in containers without CAP_NET_RAW.
"""
import socket
import struct
import time

MDNS_ADDR = "224.0.0.251"
MDNS_PORT = 5353
LOCAL_IP = "172.20.10.2"

def get_wlan_ip():
    s = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
    try:
        s.connect(("224.0.0.251", 5353))
        return s.getsockname()[0]
    except:
        pass
    finally:
        s.close()
    return LOCAL_IP

local_ip = get_wlan_ip()

def build_dns_name(name: str) -> bytes:
    parts = name.rstrip(".").split(".")
    result = b""
    for part in parts:
        result += bytes([len(part)]) + part.encode()
    return result + b"\x00"

AGRI_SERVER_QNAME = build_dns_name("agri-server.local")

def make_response(data: bytes) -> bytes | None:
    """Parse query, return A record response if it matches agri-server.local."""
    if len(data) < 12:
        return None

    tid = struct.unpack("!H", data[0:2])[0]
    qdcount = struct.unpack("!H", data[4:6])[0]
    if qdcount == 0:
        return None

    # Check the question section
    pos = 12
    for _ in range(qdcount):
        # Parse QNAME
        qname_parts = []
        while pos < len(data):
            length = data[pos]
            if length == 0:
                pos += 1
                break
            if length & 0xC0:  # compression
                pos += 2
                break
            pos += 1
            qname_parts.append(data[pos:pos + length])
            pos += length

        if pos + 4 > len(data):
            return None
        qtype, qclass = struct.unpack("!HH", data[pos:pos + 4])
        pos += 4

        # Check if query is for agri-server.local A record
        qname = b"".join(bytes([len(p)]) + p for p in qname_parts) + b"\x00"
        if qname == AGRI_SERVER_QNAME and qtype in (1, 255):  # A or ANY
            # Build response
            flags = 0x8400  # response + authoritative
            header = struct.pack("!HHHHHH", tid, flags, qdcount, 1, 0, 0)
            # Echo back the question
            question = data[12:pos]
            # Answer: A record for agri-server.local
            answer = AGRI_SERVER_QNAME
            answer += struct.pack("!HHIH", 1, 1, 120, 4)  # A, IN, TTL 120, rdlength 4
            answer += socket.inet_aton(local_ip)
            return header + question + answer

    return None

# Create listen socket
sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM, socket.IPPROTO_UDP)
sock.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
sock.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEPORT, 1)
sock.bind(("", MDNS_PORT))
sock.setsockopt(socket.IPPROTO_IP, socket.IP_MULTICAST_LOOP, 1)
sock.setsockopt(socket.IPPROTO_IP, socket.IP_MULTICAST_TTL, 2)

# Join multicast group
mreq = struct.pack("4s4s", socket.inet_aton(MDNS_ADDR), socket.inet_aton(local_ip))
sock.setsockopt(socket.IPPROTO_IP, socket.IP_ADD_MEMBERSHIP, mreq)

# Create send socket with proper multicast interface
send_sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM, socket.IPPROTO_UDP)
send_sock.setsockopt(socket.IPPROTO_IP, socket.IP_MULTICAST_TTL, 2)
send_sock.setsockopt(socket.IPPROTO_IP, socket.IP_MULTICAST_LOOP, 1)
send_sock.setsockopt(socket.IPPROTO_IP, socket.IP_MULTICAST_IF, socket.inet_aton(local_ip))

sock.settimeout(1.0)
print(f"mDNS responder: agri-server.local → {local_ip}:3001", flush=True)

while True:
    try:
        data, addr = sock.recvfrom(1024)
    except socket.timeout:
        continue
    except:
        break

    resp = make_response(data)
    if resp:
        # Send response to multicast (ESP32 listens on multicast)
        send_sock.sendto(resp, (MDNS_ADDR, MDNS_PORT))
        # Also send unicast response directly to querier
        send_sock.sendto(resp, (addr[0], MDNS_PORT))

"""Raw packet recorder for AION 2 research (Windows, needs Npcap + Administrator).

Records every TCP payload of the game connection (both directions) into a
JSONL file, plus free-text markers you type while playing, so a later pass can
line packets up with in-game events ("opened boss list", "boss spawned", ...).

    python capture.py [--port 13328] [--out captures]

Type a line + Enter at any time to drop a marker with the current timestamp.
Ctrl+C (or typing `quit`) stops and closes the file.
"""
import argparse
import ctypes
import json
import os
import struct
import sys
import threading
import time
from ctypes import (POINTER, Structure, byref, c_char, c_char_p, c_int, c_long,
                    c_uint, c_ushort, c_ubyte, c_void_p, cast)

NPCAP_DIR = os.path.join(os.environ.get("SystemRoot", r"C:\Windows"), "System32", "Npcap")


class SockAddr(Structure):
    _fields_ = [("sa_family", c_ushort), ("sa_data", c_ubyte * 14)]


class PcapAddr(Structure):
    pass


PcapAddr._fields_ = [("next", POINTER(PcapAddr)), ("addr", POINTER(SockAddr)),
                     ("netmask", POINTER(SockAddr)), ("broadaddr", POINTER(SockAddr)),
                     ("dstaddr", POINTER(SockAddr))]


class PcapIf(Structure):
    pass


PcapIf._fields_ = [("next", POINTER(PcapIf)), ("name", c_char_p), ("description", c_char_p),
                   ("addresses", POINTER(PcapAddr)), ("flags", c_uint)]


class PktHdr(Structure):
    # struct timeval is two C longs: 32-bit on Windows.
    _fields_ = [("ts_sec", c_long), ("ts_usec", c_long), ("caplen", c_uint), ("len", c_uint)]


def load_pcap():
    os.add_dll_directory(NPCAP_DIR)
    lib = ctypes.WinDLL(os.path.join(NPCAP_DIR, "wpcap.dll"))
    lib.pcap_findalldevs.argtypes = [POINTER(POINTER(PcapIf)), c_char_p]
    lib.pcap_freealldevs.argtypes = [POINTER(PcapIf)]
    lib.pcap_open_live.argtypes = [c_char_p, c_int, c_int, c_int, c_char_p]
    lib.pcap_open_live.restype = c_void_p
    lib.pcap_next_ex.argtypes = [c_void_p, POINTER(POINTER(PktHdr)), POINTER(POINTER(c_ubyte))]
    lib.pcap_datalink.argtypes = [c_void_p]
    lib.pcap_close.argtypes = [c_void_p]
    return lib


def list_devices(lib):
    """(name, description, [ipv4...], is_loopback) for every device."""
    errbuf = ctypes.create_string_buffer(256)
    head = POINTER(PcapIf)()
    if lib.pcap_findalldevs(byref(head), errbuf) != 0:
        raise RuntimeError(errbuf.value.decode(errors="replace"))
    out = []
    dev = head
    while dev:
        d = dev.contents
        ips = []
        a = d.addresses
        while a:
            ad = a.contents
            if ad.addr and ad.addr.contents.sa_family == 2:
                ips.append(".".join(str(b) for b in ad.addr.contents.sa_data[2:6]))
            a = ad.next
        out.append((d.name, (d.description or b"").decode(errors="replace"), ips, bool(d.flags & 1)))
        dev = d.next
    lib.pcap_freealldevs(head)
    return out


def ip_offset(link, frame):
    if link == 1:  # Ethernet, up to two VLAN tags
        at = 12
        for _ in range(2):
            if frame[at:at + 2] in (b"\x81\x00", b"\x88\xa8"):
                at += 4
        return at + 2 if frame[at:at + 2] == b"\x08\x00" else None
    if link == 0:  # Npcap loopback
        return 4 if frame[:4] == b"\x02\x00\x00\x00" else None
    if link in (12, 14, 101, 228):
        return 0
    return None


def parse(frame, link):
    ip = ip_offset(link, frame)
    if ip is None or len(frame) < ip + 20 or frame[ip] >> 4 != 4 or frame[ip + 9] != 6:
        return None
    ihl = (frame[ip] & 15) * 4
    tcp = ip + ihl
    if len(frame) < tcp + 20:
        return None
    sport, dport, seq, ack = struct.unpack(">HHII", frame[tcp:tcp + 12])
    thl = (frame[tcp + 12] >> 4) * 4
    total = struct.unpack(">H", frame[ip + 2:ip + 4])[0]
    end = ip + total if total >= ihl + 20 and ip + total <= len(frame) else len(frame)
    payload = frame[tcp + thl:end]
    if not payload:
        return None
    src = ".".join(str(b) for b in frame[ip + 12:ip + 16])
    dst = ".".join(str(b) for b in frame[ip + 16:ip + 20])
    return src, dst, sport, dport, seq, ack, payload


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--port", type=int, default=13328, help="game server TCP port")
    ap.add_argument("--out", default="captures")
    args = ap.parse_args()

    lib = load_pcap()
    devices = [d for d in list_devices(lib) if d[2] and not d[3]
               and not any(ip.startswith("169.254.") for ip in d[2])]
    if not devices:
        sys.exit("no capture devices with an IPv4 address")

    os.makedirs(args.out, exist_ok=True)
    path = os.path.join(args.out, time.strftime("cap_%Y%m%d_%H%M%S.jsonl"))
    f = open(path, "w", encoding="utf-8")
    lock = threading.Lock()
    running = threading.Event()
    running.set()
    seen = set()
    counts = {"packets": 0, "bytes": 0}

    def emit(rec):
        with lock:
            f.write(json.dumps(rec, separators=(",", ":")) + "\n")
            f.flush()

    def worker(name, desc):
        errbuf = ctypes.create_string_buffer(256)
        h = lib.pcap_open_live(name, 65535, 0, 100, errbuf)
        if not h:
            print(f"skip {desc}: {errbuf.value.decode(errors='replace')}")
            return
        link = lib.pcap_datalink(h)
        hdr = POINTER(PktHdr)()
        data = POINTER(c_ubyte)()
        while running.is_set():
            r = lib.pcap_next_ex(h, byref(hdr), byref(data))
            if r == 0:
                continue
            if r != 1:
                break
            n = hdr.contents.caplen
            frame = bytes(data[:n])
            p = parse(frame, link)
            if not p:
                continue
            src, dst, sport, dport, seq, ack, payload = p
            if args.port not in (sport, dport):
                continue
            key = (src, dst, sport, dport, seq, len(payload))
            with lock:
                if key in seen:
                    continue
                seen.add(key)
                counts["packets"] += 1
                counts["bytes"] += len(payload)
            ts = hdr.contents.ts_sec * 1000 + hdr.contents.ts_usec // 1000
            emit({"t": ts, "dir": "S2C" if sport == args.port else "C2S",
                  "src": f"{src}:{sport}", "dst": f"{dst}:{dport}",
                  "seq": seq, "ack": ack, "hex": payload.hex()})
        lib.pcap_close(h)

    for name, desc, ips, _ in devices:
        print(f"capturing on {desc} {ips}")
        threading.Thread(target=worker, args=(name, desc), daemon=True).start()

    print(f"\nrecording port {args.port} -> {path}")
    print("type a marker + Enter (e.g. 'opened boss list'); 'quit' to stop\n")
    try:
        for line in sys.stdin:
            line = line.strip()
            if line.lower() == "quit":
                break
            if line:
                emit({"t": int(time.time() * 1000), "dir": "MARK", "text": line})
                print(f"  marker saved ({counts['packets']} packets, {counts['bytes']} bytes so far)")
    except KeyboardInterrupt:
        pass
    running.clear()
    time.sleep(0.3)
    f.close()
    print(f"done: {counts['packets']} packets, {counts['bytes']} bytes -> {path}")


if __name__ == "__main__":
    main()

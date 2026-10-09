"""Analyse a capture.py recording: rebuild the server stream, split it into game
packets (same framing as A2Tools), and show which opcodes appear right after
each marker compared with the rest of the session.

    python analyze.py captures/cap_X.jsonl [--window 4]
"""
import argparse
import bisect
import collections
import json
import sys


def read_varint(buf, at):
    """Protobuf-style LEB128. Returns (value, width) or (None, 0)."""
    val = shift = 0
    for i in range(5):
        if at + i >= len(buf):
            return None, 0
        b = buf[at + i]
        val |= (b & 0x7F) << shift
        if not b & 0x80:
            return val, i + 1
        shift += 7
    return None, 0


def lz4_block(src, size):
    out = bytearray()
    i = 0
    n = len(src)
    while i < n:
        tok = src[i]
        i += 1
        lit = tok >> 4
        if lit == 15:
            while True:
                b = src[i]
                i += 1
                lit += b
                if b != 255:
                    break
        out += src[i:i + lit]
        i += lit
        if i >= n:
            break
        off = src[i] | (src[i + 1] << 8)
        i += 2
        ml = tok & 15
        if ml == 15:
            while True:
                b = src[i]
                i += 1
                ml += b
                if b != 255:
                    break
        ml += 4
        if off == 0 or off > len(out):
            raise ValueError("bad lz4 offset")
        for _ in range(ml):
            out.append(out[-off])
        if len(out) > size:
            break
    if len(out) != size:
        raise ValueError("lz4 size mismatch")
    return bytes(out)


def tls_size(b):
    if len(b) < 5 or not 0x14 <= b[0] <= 0x17 or b[1] != 3 or not 1 <= b[2] <= 4:
        return None
    n = (b[3] << 8) | b[4]
    return 5 + n if 0 < n <= 16384 + 256 else None


def walk(buf, inner=False):
    """Yield (start, end, payload_start, is_bundle). Returns consumed via list."""
    frames = []
    off = 0
    while off < len(buf):
        if buf[off] == 0:
            off += 1
            continue
        if not inner:
            t = tls_size(buf[off:off + 5])
            if t:
                if off + t > len(buf):
                    break
                off += t
                continue
        val, w = read_varint(buf, off)
        if not val or w <= 0:
            if inner:
                break
            if off + 5 > len(buf):
                break
            off += 1
            continue
        size = val - 4 + w
        if size < w or size <= 0 or size > 65535:
            off += 1
            continue
        if off + size > len(buf):
            if inner or size > 16384:
                if inner:
                    break
                off += 1
                continue
            break
        bundle = w + 1 < size and buf[off + w] == 0xFF and buf[off + w + 1] == 0xFF
        frames.append((off, off + size, w, bundle))
        off += size
    return frames, off


def explode(payload, depth=1):
    """Packets (body bytes after the length varint) inside a bundle payload."""
    if depth > 4 or len(payload) < 7:
        return
    size = int.from_bytes(payload[2:6], "little")
    if not 0 < size <= 1_000_000:
        return
    try:
        data = lz4_block(payload[6:], size)
    except Exception:
        return
    frames, _ = walk(data, inner=True)
    for s, e, w, b in frames:
        if b:
            yield from explode(data[s + w:e], depth + 1)
        else:
            yield data[s + w:e]


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("file")
    ap.add_argument("--window", type=float, default=4.0, help="seconds after a marker")
    ap.add_argument("--dump", type=int, default=0, help="hex-dump N packets per rare opcode")
    args = ap.parse_args()

    segs = collections.defaultdict(dict)  # connection -> seq -> (ts, bytes)
    marks = []
    for line in open(args.file, encoding="utf-8"):
        r = json.loads(line)
        if r["dir"] == "MARK":
            marks.append((r["t"], r["text"]))
        elif r["dir"] == "S2C":
            segs[(r["src"], r["dst"])].setdefault(r["seq"], (r["t"], bytes.fromhex(r["hex"])))
    if not segs:
        sys.exit("no server->client data")

    packets = []  # (ts, opcode, body)
    for conn, d in segs.items():
        seqs = sorted(d)
        stream = bytearray()
        starts = []  # (stream offset, ts)
        nxt = None
        for sq in seqs:
            ts, data = d[sq]
            if nxt is not None and sq < nxt:  # retransmission / overlap
                skip = nxt - sq
                if skip >= len(data):
                    continue
                data = data[skip:]
                sq = nxt
            elif nxt is not None and sq > nxt:  # gap: lost segment, resync later
                stream += b"\x00"
            starts.append((len(stream), ts))
            stream += data
            nxt = sq + len(data)
        offs = [s for s, _ in starts]
        frames, _ = walk(bytes(stream))
        print(f"{conn[0]}: {len(stream)} bytes, {len(frames)} frames")
        for s, e, w, b in frames:
            ts = starts[max(bisect.bisect_right(offs, s) - 1, 0)][1]
            if b:
                for body in explode(bytes(stream[s + w:e])):
                    packets.append((ts, body[:2].hex(), body))
            else:
                body = bytes(stream[s + w:e])
                packets.append((ts, body[:2].hex(), body))

    packets.sort(key=lambda p: p[0])
    total = collections.Counter(p[1] for p in packets)
    t0, t1 = packets[0][0], packets[-1][0]
    span = max((t1 - t0) / 1000, 1)
    print(f"\n{len(packets)} packets over {span:.0f}s, {len(total)} distinct opcodes")
    print("most common:", ", ".join(f"{k}x{v}" for k, v in total.most_common(12)))

    for mt, text in marks:
        win = [p for p in packets if mt - 500 <= p[0] <= mt + args.window * 1000]
        c = collections.Counter(p[1] for p in win)
        print(f"\n== marker '{text}' @ +{(mt - t0) / 1000:.1f}s: {len(win)} packets in window")
        rows = []
        for op, n in c.items():
            base = total[op] / span * args.window  # expected count in an average window
            rows.append((n / (base + 0.5), op, n, total[op]))
        for score, op, n, tot in sorted(rows, reverse=True)[:12]:
            print(f"  op {op}: {n} in window, {tot} in session (x{score:.1f} vs average)")

    if args.dump:
        for mt, text in marks:
            win = [p for p in packets if mt - 500 <= p[0] <= mt + args.window * 1000
                   and total[p[1]] <= 40]
            print(f"\n== rare packets near '{text}'")
            for ts, op, body in win[:args.dump]:
                print(f"  +{(ts - mt) / 1000:.2f}s op {op} len {len(body)}: {body[:96].hex()}")


if __name__ == "__main__":
    main()

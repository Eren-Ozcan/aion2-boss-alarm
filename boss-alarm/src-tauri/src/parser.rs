//! Wire framing and the boss-list packet of the AION 2 server stream.
//!
//! The server-to-client TCP stream is a sequence of frames:
//!
//! ```text
//! 00 ...                                padding
//! 17 03 03 <u16 len> ...                a TLS record from another connection
//! <varint len> <body>                   a packet; len counts the body plus 4
//! <varint len> FF FF <u32 size> <lz4>   a bundle of further packets
//! ```
//!
//! The boss list is the packet whose body starts `01 91` (see
//! `docs/boss-list-packet.md` at the repository root for how it was found).

const MAX_PACKET_BYTES: usize = 65_535;
const MAX_FRAGMENT_WAIT_BYTES: usize = 16_384;
const MAX_DECOMPRESSED_BYTES: usize = 1_000_000;
const MAX_BUNDLE_DEPTH: usize = 4;

/// Protobuf-style LEB128. Returns `(value, width)`.
pub fn read_varint(buf: &[u8], at: usize) -> Option<(u32, usize)> {
    let mut value: u32 = 0;
    for i in 0..5 {
        let b = *buf.get(at + i)?;
        value |= ((b & 0x7F) as u32) << (7 * i);
        if b & 0x80 == 0 {
            return Some((value, i + 1));
        }
    }
    None
}

fn tls_record_size(b: &[u8]) -> Option<usize> {
    if b.len() < 5 || !(0x14..=0x17).contains(&b[0]) || b[1] != 0x03 || !(0x01..=0x04).contains(&b[2]) {
        return None;
    }
    let len = u16::from_be_bytes([b[3], b[4]]) as usize;
    (len > 0 && len <= 16_384 + 256).then_some(5 + len)
}

struct Frame {
    start: usize,
    end: usize,
    payload_start: usize,
    bundle: bool,
}

/// Walk `buf` into frames. `inner` is true for the decompressed contents of a
/// bundle, which the game framed itself, so a bad length ends the walk instead
/// of resynchronising. Returns the frames and how many bytes were consumed.
fn walk(buf: &[u8], inner: bool) -> (Vec<Frame>, usize) {
    let mut frames = Vec::new();
    let mut off = 0usize;
    while off < buf.len() {
        if buf[off] == 0 {
            off += 1;
            continue;
        }
        if !inner {
            if let Some(size) = tls_record_size(&buf[off..]) {
                if off + size > buf.len() {
                    break;
                }
                off += size;
                continue;
            }
        }
        let Some((value, width)) = read_varint(buf, off).filter(|&(v, _)| v > 0) else {
            if inner || off + 5 > buf.len() {
                break;
            }
            off += 1;
            continue;
        };
        let size = value as i64 - 4 + width as i64;
        if size < width as i64 || size <= 0 || size as usize > MAX_PACKET_BYTES {
            off += 1;
            continue;
        }
        let size = size as usize;
        if off + size > buf.len() {
            if inner {
                break;
            }
            if size > MAX_FRAGMENT_WAIT_BYTES {
                off += 1;
                continue;
            }
            break; // a TCP fragment: wait for the rest
        }
        let bundle = width + 1 < size && buf[off + width] == 0xFF && buf[off + width + 1] == 0xFF;
        frames.push(Frame { start: off, end: off + size, payload_start: width, bundle });
        off += size;
    }
    (frames, off)
}

fn explode(payload: &[u8], depth: usize, out: &mut Vec<Vec<u8>>) {
    if depth > MAX_BUNDLE_DEPTH || payload.len() < 7 {
        return;
    }
    let size = u32::from_le_bytes([payload[2], payload[3], payload[4], payload[5]]) as usize;
    if size == 0 || size > MAX_DECOMPRESSED_BYTES {
        return;
    }
    let Ok(data) = lz4_flex::block::decompress(&payload[6..], size) else { return };
    let (frames, _) = walk(&data, true);
    for f in frames {
        let payload = &data[f.start + f.payload_start..f.end];
        if f.bundle {
            explode(payload, depth + 1, out);
        } else {
            out.push(payload.to_vec());
        }
    }
}

/// Split the front of `buf` into packet bodies (bundles opened). Returns the
/// bodies and the number of bytes consumed; the rest is an unfinished packet.
pub fn split_packets(buf: &[u8]) -> (Vec<Vec<u8>>, usize) {
    let (frames, consumed) = walk(buf, false);
    let mut out = Vec::new();
    for f in frames {
        let payload = &buf[f.start + f.payload_start..f.end];
        if f.bundle {
            explode(payload, 1, &mut out);
        } else {
            out.push(payload.to_vec());
        }
    }
    (out, consumed)
}

/// The server clock in unix milliseconds, from a `00 36` packet (sent many
/// times a second), if `body` is one.
pub fn server_time_ms(body: &[u8]) -> Option<u64> {
    if body.len() == 10 && body[0] == 0x00 && body[1] == 0x36 {
        let v = u64::from_le_bytes(body[2..10].try_into().ok()?);
        // Roughly 2020 .. 2033; anything else is not a clock.
        (1_600_000_000_000..2_000_000_000_000).contains(&v).then_some(v)
    } else {
        None
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ListEntry {
    pub id: u32,
    /// The boss is alive on the map: its record carries a position.
    pub spawned: bool,
    /// Server time in ms of the next respawn; `None` when the record carries
    /// none (spawned, or a timer the packet does not hold).
    pub respawn_ms: Option<u64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BossList {
    /// Which list this is: the field map list or the abyss list.
    pub list_id: u16,
    pub entries: Vec<ListEntry>,
}

/// How far from the server clock a timestamp may sit and still count as one.
const WINDOW_MS: u64 = 120 * 86_400_000;

fn u48(b: &[u8], at: usize) -> Option<u64> {
    let s = b.get(at..at + 6)?;
    Some(u64::from_le_bytes([s[0], s[1], s[2], s[3], s[4], s[5], 0, 0]))
}

fn plausible(ts: u64, now_ms: u64) -> bool {
    ts + WINDOW_MS > now_ms && ts < now_ms + WINDOW_MS
}

/// The id range of the list: the first entry's id fixes the block of a hundred
/// that every other id of the list falls in.
fn id_block(first: u32) -> (u32, u32) {
    let base = first / 100 * 100;
    (base, base + 100)
}

/// Parse a `01 91` boss-list packet body. `now_ms` is the server clock.
///
/// Layout (see docs/boss-list-packet.md):
/// `01 91 | 00 00 | list_id u16 | 00 00 | count | 1 byte | entries`, each entry
/// `id varint | [x y z f32 when spawned] | [1 odd byte] | u48 ms | 3-5 tail bytes`.
/// Entry ends are not length-prefixed, so the next entry is found by looking for
/// the next valid id just after the timestamp.
pub fn parse_boss_list(body: &[u8], now_ms: u64) -> Option<BossList> {
    if body.len() < 12 || body[0] != 0x01 || body[1] != 0x91 {
        return None;
    }
    let list_id = u16::from_le_bytes([body[4], body[5]]);
    let (count, cw) = read_varint(body, 8)?;
    if count == 0 || count > 200 {
        return None;
    }
    let mut at = 8 + cw + 1;
    let (first, _) = read_varint(body, at)?;
    let (lo, hi) = id_block(first);
    let valid_id = |p: usize| read_varint(body, p).filter(|&(v, _)| v >= lo && v < hi);

    let mut entries: Vec<ListEntry> = Vec::new();
    for _ in 0..count {
        let Some((id, w)) = valid_id(at) else { break };
        if entries.iter().any(|e| e.id == id) {
            break;
        }
        let rest = at + w;

        // Timed: the timestamp right after the id, or after one odd byte.
        let mut entry = None;
        for skip in [0usize, 1] {
            if let Some(ts) = u48(body, rest + skip).filter(|&t| plausible(t, now_ms)) {
                entry = Some((ListEntry { id, spawned: false, respawn_ms: Some(ts) }, rest + skip + 6));
                break;
            }
        }
        // A record with no timer at all: six zero bytes where the time would be.
        if entry.is_none() && u48(body, rest) == Some(0) {
            entry = Some((ListEntry { id, spawned: false, respawn_ms: None }, rest + 6));
        }
        // Spawned: a position (three finite floats) comes first.
        if entry.is_none() {
            let floats_ok = body.get(rest..rest + 12).is_some_and(|s| {
                s.chunks(4).all(|c| {
                    let f = f32::from_le_bytes([c[0], c[1], c[2], c[3]]);
                    f.is_finite() && f.abs() < 1.0e7
                })
            });
            if floats_ok {
                entry = Some((ListEntry { id, spawned: true, respawn_ms: None }, rest + 12 + 6));
            }
        }
        let Some((e, ts_end)) = entry else { break };
        entries.push(e);

        // The next id starts a few tail bytes after the timestamp.
        match (3..=6usize).map(|d| ts_end + d).find(|&p| {
            valid_id(p).is_some_and(|(v, _)| !entries.iter().any(|x| x.id == v))
        }) {
            Some(p) => at = p,
            None => break,
        }
    }
    (!entries.is_empty()).then_some(BossList { list_id, entries })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(name: &str) -> Vec<u8> {
        let path = format!("{}/tests/fixtures/{name}", env!("CARGO_MANIFEST_DIR"));
        let hex = std::fs::read_to_string(path).unwrap();
        let hex = hex.trim();
        (0..hex.len()).step_by(2).map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap()).collect()
    }

    /// Server clock of the capture moment: 2026-10-09 ~20:07 local.
    const NOW: u64 = 1_791_563_199_268;

    fn rem(l: &BossList, id: u32) -> Option<i64> {
        l.entries.iter().find(|e| e.id == id)?.respawn_ms.map(|t| (t as i64 - NOW as i64) / 1000)
    }

    #[test]
    fn field_list_matches_what_the_game_showed() {
        let l = parse_boss_list(&fixture("field_list.hex"), NOW).expect("parses");
        assert_eq!(l.list_id, 0x0456);
        assert_eq!(l.entries.len(), 24);
        // Alive on the map in the in-game list: Danar, Aed, the two others.
        let mut spawned: Vec<u32> = l.entries.iter().filter(|e| e.spawned).map(|e| e.id).collect();
        spawned.sort();
        assert_eq!(spawned, vec![111001, 111002, 111011, 111014]);
        // Times read off the screen about 7 s after the packet.
        for (id, shown) in [(111003, 324), (111004, 2693), (111005, 6565), (111006, 311), (111010, 10292)] {
            let r = rem(&l, id).unwrap();
            assert!((r - shown).abs() <= 12, "id {id}: packet {r}, screen {shown}");
        }
    }

    #[test]
    fn abyss_list() {
        let l = parse_boss_list(&fixture("abyss_list.hex"), NOW + 750_000).expect("parses");
        assert_eq!(l.list_id, 0x0014);
        assert_eq!(l.entries.len(), 8);
        assert!(l.entries.iter().find(|e| e.id == 2001).unwrap().spawned);
        // Three Nahma share a respawn, three executors share another.
        let nahma: Vec<_> = [2003, 2004, 2005].iter().map(|&i| rem(&l, i).unwrap()).collect();
        assert!(nahma.iter().all(|&r| (r - nahma[0]).abs() <= 1), "{nahma:?}");
        let exec: Vec<_> = [2006, 2007, 2008].iter().map(|&i| rem(&l, i).unwrap()).collect();
        assert!(exec.iter().all(|&r| (r - exec[0]).abs() <= 1), "{exec:?}");
    }

    #[test]
    fn garbage_is_not_a_list() {
        assert!(parse_boss_list(&[0x01, 0x91], NOW).is_none());
        assert!(parse_boss_list(&[0u8; 40], NOW).is_none());
    }

    fn varint(mut v: u32) -> Vec<u8> {
        let mut out = Vec::new();
        loop {
            let b = (v & 0x7F) as u8;
            v >>= 7;
            if v == 0 {
                out.push(b);
                return out;
            }
            out.push(b | 0x80);
        }
    }

    fn packet(body: &[u8]) -> Vec<u8> {
        let mut v = varint(body.len() as u32 + 4);
        v.extend_from_slice(body);
        v
    }

    #[test]
    fn framing_reassembles_across_fragments_and_bundles() {
        let list = fixture("field_list.hex");
        let mut ping = vec![0x00, 0x36];
        ping.extend_from_slice(&NOW.to_le_bytes());
        let inner: Vec<u8> = [packet(&ping), packet(&list)].concat();
        let mut bundle_payload = vec![0xFF, 0xFF];
        bundle_payload.extend_from_slice(&(inner.len() as u32).to_le_bytes());
        bundle_payload.extend_from_slice(&lz4_flex::block::compress(&inner));
        let stream: Vec<u8> = [packet(&list), packet(&bundle_payload), vec![0, 0]].concat();

        let (all, used) = split_packets(&stream);
        assert_eq!(used, stream.len());
        assert_eq!(all.len(), 3);
        assert_eq!(server_time_ms(&all[1]), Some(NOW));

        // Fed in 7-byte pieces, keeping the unconsumed tail.
        let mut pending = Vec::new();
        let mut got = 0;
        for chunk in stream.chunks(7) {
            pending.extend_from_slice(chunk);
            let (bodies, n) = split_packets(&pending);
            got += bodies.len();
            pending.drain(..n);
        }
        assert_eq!(got, 3);
    }
}

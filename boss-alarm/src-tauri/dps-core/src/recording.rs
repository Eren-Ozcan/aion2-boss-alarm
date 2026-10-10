//! The on-disk format of a recorded game stream (`stream_<time>.bin`).
//!
//! `A2S1`, then frames of `flow: u64 LE`, `len: u32 LE`, `len` bytes. The flow
//! says which TCP connection the bytes came from; replaying must keep them
//! apart, as the live engine does. A file without the magic is the older
//! format, one undivided stream, and reads as a single flow.

pub const MAGIC: &[u8; 4] = b"A2S1";

pub fn write_frame(out: &mut impl std::io::Write, flow: u64, data: &[u8]) -> std::io::Result<()> {
    out.write_all(&flow.to_le_bytes())?;
    out.write_all(&(data.len() as u32).to_le_bytes())?;
    out.write_all(data)
}

/// All frames of a recording, in order. Stops quietly at a truncated tail.
pub fn read_frames(bytes: &[u8]) -> Vec<(u64, &[u8])> {
    let Some(mut rest) = bytes.strip_prefix(MAGIC.as_slice()) else {
        return vec![(0, bytes)];
    };
    let mut out = Vec::new();
    while rest.len() >= 12 {
        let flow = u64::from_le_bytes(rest[..8].try_into().unwrap());
        let len = u32::from_le_bytes(rest[8..12].try_into().unwrap()) as usize;
        let Some(data) = rest.get(12..12 + len) else { break };
        out.push((flow, data));
        rest = &rest[12 + len..];
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frames_round_trip_and_old_files_read_as_one_flow() {
        let mut buf = MAGIC.to_vec();
        write_frame(&mut buf, 7, b"abc").unwrap();
        write_frame(&mut buf, 9, b"").unwrap();
        write_frame(&mut buf, 7, b"de").unwrap();
        assert_eq!(read_frames(&buf), vec![(7, &b"abc"[..]), (9, &b""[..]), (7, &b"de"[..])]);
        buf.truncate(buf.len() - 1); // a cut tail is dropped, not misread
        assert_eq!(read_frames(&buf).len(), 2);
        assert_eq!(read_frames(b"raw bytes"), vec![(0, &b"raw bytes"[..])]);
    }
}

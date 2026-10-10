//! For every `04 38` packet in a recording, reads the first record's header
//! (target, switch, actor) and counts hits per (target, actor). Run the engine's
//! own total next to it with the `replay` example to see how many hits the
//! parser keeps.
//!
//!     cargo run -p dps-core --example hit_headers -- path/to/stream.bin

use std::collections::HashMap;

use dps_core::capture::framing::{decompress_bundle, walk, walk_inner, FrameKind};
use dps_core::capture::stream_processor::read_varint;

fn header(packet: &[u8]) -> Option<(i32, i32, i32)> {
    let len = read_varint(packet, 0);
    if len.length <= 0 {
        return None;
    }
    let mut o = len.length as usize;
    if packet.get(o)? != &0x04 || packet.get(o + 1)? != &0x38 {
        return None;
    }
    o += 2;
    let t = read_varint(packet, o);
    if t.length <= 0 {
        return None;
    }
    o += t.length as usize;
    let sw = read_varint(packet, o);
    if sw.length <= 0 {
        return None;
    }
    o += sw.length as usize;
    let flag = read_varint(packet, o);
    if flag.length <= 0 {
        return None;
    }
    o += flag.length as usize;
    let a = read_varint(packet, o);
    if a.length <= 0 {
        return None;
    }
    Some((t.value, sw.value, a.value))
}

fn main() {
    let path = std::env::args().nth(1).expect("usage: hit_headers <stream.bin>");
    let data = std::fs::read(&path).expect("cannot read stream file");
    let mut flows: HashMap<u64, Vec<u8>> = HashMap::new();
    for (flow, bytes) in dps_core::recording::read_frames(&data) {
        flows.entry(flow).or_default().extend_from_slice(bytes);
    }
    let mut pairs: HashMap<(i32, i32), usize> = HashMap::new();
    let mut embedded: HashMap<(i32, i32), usize> = HashMap::new();
    let mut total = 0usize;
    let mut unreadable = 0usize;
    for buf in flows.values() {
        let mut packets: Vec<Vec<u8>> = Vec::new();
        for f in &walk(buf).frames {
            match f.kind {
                FrameKind::Packet => packets.push(f.bytes(buf).to_vec()),
                FrameKind::Bundle => {
                    if let Some(inner) = decompress_bundle(f.payload(buf)) {
                        for g in &walk_inner(&inner).frames {
                            packets.push(g.bytes(&inner).to_vec());
                        }
                    }
                }
            }
        }
        for p in &packets {
            // Embedded: a `04 38` anywhere inside a packet whose first opcode is something else.
            let first = read_varint(p, 0).length.max(0) as usize;
            if !(p.get(first) == Some(&0x04) && p.get(first + 1) == Some(&0x38)) {
                for i in first..p.len().saturating_sub(2) {
                    if p[i] == 0x04 && p[i + 1] == 0x38 {
                        let mut tmp = vec![0x01u8];
                        tmp.extend_from_slice(&p[i..]);
                        if let Some((t, sw, a)) = header(&tmp) {
                            if (100..200_000).contains(&t) && (100..200_000).contains(&a) && matches!(sw & 0x0F, 0 | 2 | 4..=7) {
                                *embedded.entry((t, a)).or_default() += 1;
                            }
                        }
                    }
                }
            }
            let len = read_varint(p, 0);
            let o = len.length.max(0) as usize;
            if p.get(o) == Some(&0x04) && p.get(o + 1) == Some(&0x38) {
                total += 1;
                match header(p) {
                    Some((t, _sw, a)) => *pairs.entry((t, a)).or_default() += 1,
                    None => unreadable += 1,
                }
            }
        }
    }
    if let Some(t) = std::env::var("BOSS").ok().and_then(|v| v.parse::<i32>().ok()) {
        let mut a: Vec<_> = embedded.iter().filter(|((tt, _), _)| *tt == t).map(|((_, a), n)| (*a, *n)).collect();
        a.sort_by(|x, y| y.1.cmp(&x.1));
        println!("embedded 04 38 records on target {t}: {} actors", a.len());
        for (act, n) in a.iter().take(15) {
            println!("   actor {act}: {n}");
        }
    }
    {
        let mut per: HashMap<i32, (usize, usize)> = HashMap::new();
        let mut both = pairs.clone();
        for (k, n) in &embedded {
            *both.entry(*k).or_default() += n;
        }
        println!("pairs: head {} embedded {} (embedded hits {})", pairs.len(), embedded.len(), embedded.values().sum::<usize>());
        for (&(t, _), n) in &both {
            let e = per.entry(t).or_default();
            e.0 += n;
            e.1 += 1;
        }
        let mut v: Vec<_> = per.into_iter().collect();
        v.sort_by(|a, b| b.1 .1.cmp(&a.1 .1));
        println!("targets by number of distinct actors hitting them:");
        for (t, (n, k)) in v.iter().take(6) {
            println!("   target {t}: {k} actors, {n} hits");
        }
    }
    println!("04 38 packets: {total}, header unreadable: {unreadable}");
    let mut by_target: HashMap<i32, usize> = HashMap::new();
    for (&(t, _), n) in &pairs {
        *by_target.entry(t).or_default() += n;
    }
    // With ME=<actor id>, the targets that actor hit most are the ones listed.
    let me: Option<i32> = std::env::var("ME").ok().and_then(|v| v.parse().ok());
    let mut targets: Vec<_> = match me {
        Some(me) => {
            let mut v: HashMap<i32, usize> = HashMap::new();
            for (&(t, a), n) in &pairs {
                if a == me && t != me {
                    *v.entry(t).or_default() += n;
                }
            }
            v.into_iter().collect()
        }
        None => by_target.into_iter().collect(),
    };
    targets.sort_by(|a, b| b.1.cmp(&a.1));
    for (t, n) in targets.iter().take(5) {
        println!("target {t}: {n} hits received");
    }
    if let Some(&(t, _)) = targets.first() {
        let mut actors: Vec<_> = pairs.iter().filter(|((tt, _), _)| *tt == t).map(|((_, a), n)| (*a, *n)).collect();
        actors.sort_by(|a, b| b.1.cmp(&a.1));
        println!("hits on target {t}, by actor id (top 12 of {}):", actors.len());
        for (a, n) in actors.iter().take(12) {
            println!("   actor {a}: {n}");
        }
    }
}

# AION 2 field boss list packet (opcode `01 91`)

Found by capturing the server-to-client stream (`tools/capture/capture.py`) while the in-game
boss list was open, then matching the on-screen "Time Left" values against packet fields
(`tools/capture/match.py`: 8/8 bosses matched to within 1 s). Game build/date: 2026-10-09.

## Transport
- Server `193.202.112.126:13328`, plain TCP, server-to-client is **not encrypted**.
- Frame: `<varint len> <payload>`, `len = payload + 4`; `FF FF <u32 size> <lz4>` is a bundle
  of further frames (same framing as A2Tools-DPS-Meter `capture/framing.rs`).
- Client-to-server traffic is encrypted/obfuscated (random-looking bytes); the client polls
  the boss list about every 1.1 s while the list window is open (7-byte `0a ..` packets).
  The server answers with `01 91` (+ a 9-byte `0c 91` that looks like an end marker).
  Nothing is sent while the list is closed, so a passive reader only sees timers when the
  game has the list open.

## `01 91` layout (351 or 363 bytes, 24 entries)
```
01 91 | 00 00 56 04 00 00 | 18 (count=24) | entries...
entry = <id varint, 3 bytes, e.g. 99 e3 06 = 111001>
        [x f32][y f32][z f32]      only while the boss is currently SPAWNED (12 bytes)
        <u48 LE server time ms>    next spawn time (timed) / spawn time (spawned, in the past)
        00 00 <flag u8>            flag meaning unknown (0/1)
```
(Some entries carry one extra byte before the timestamp; the first entry of a packet and a
few later ones are 13-14 bytes. The 6-byte timestamp is always `.. a1 01` = ~1.79e12 ms.)

- Remaining time = `timestamp - server_now`; `server_now` can be estimated from the packet
  capture time (server clock was ~10 s off the PC clock in the sample) or from `00 36`
  packets, which carry the server time in ms.
- "Spawned" bosses are the entries that carry the 12 position bytes (the in-game list shows the
  distance from the player to that position).

## Boss ids seen (list order = spawned first, then by id)
| id | name | state in sample |
|---|---|---|
| 111001 | Melted Danar | spawned |
| 111002 | Black Warrior Aed | spawned |
| 111003 | Faithful Rajit | timed |
| 111004 | Berserker Vargor | timed |
| 111005 | Predator Garsan | timed |
| 111006 | Blood Warrior Lannar | timed |
| 111007 | Deceiver Trid | timed |
| 111008 | Blue Wave Kelpina | timed |
| 111009 | High Overseer Nutah | timed |
| 111010 | Advisor Resana | timed |
| 111011 | Special Operations Leader (truncated in UI) | spawned (order by id, 111011/111014 not directly verified) |
| 111012 | Desecrator Newbold | timed (verified) |
| 111013 | Specter Archon Axios | timed (verified) |
| 111014 | Addicted Hardirun | spawned (see 111011) |
| 111015 | Executioner Barthien | timed (verified) |
| 111016 | Drakan Battalion Weapo... (truncated in UI) | timed (verified) |
| 111017 | Veteran Shujakan | timed (verified) |
| 111018 | Visionary Karuka | timed (verified) |
| 111019 | Dark Shadow Vishwada | timed (verified) |
| 111020 | Sharp Shylak | timed (verified) |
| 111021 | Immortal Gartua | timed (verified) |
| 111022 | High Commander Lagta | timed (verified) |
| 111023 | Soul Ruler Kashapa | timed (verified) |
| 111024 | Silent Dartan | timed (verified) |

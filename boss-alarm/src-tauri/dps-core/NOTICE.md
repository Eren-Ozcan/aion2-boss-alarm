# Notice

The code in this crate is derived from the parser core of
[A2Tools DPS Meter](https://github.com/taengu/A2Tools-DPS-Meter) by taengu,
licensed under GPL-3.0 (see `LICENSE-A2Tools`), taken at commit
`82e53c1008ac4c2974446cc703703f473bbbed81`.

Taken: `capture/` (framing, packet_accumulator, stream_assembler,
stream_processor, captured_payload), `combat/` (data_storage, dps_calculator,
ping_tracker), `entity/`, `i18n/`, `clock.rs`, `supporters.rs` and a few data
tables under `src/data/`.

Left out: the Tauri app, pcap capture, uploads and accounts, Discord presence,
stream overlay, Evidence Slice builder, packaging and the log service.

Changes: `include_str!` data paths point at `src/data/`; only English skill and
NPC names are bundled. This crate is part of AION2 Boss Alarm and is released
under GPL-3.0-or-later like the rest of the project.

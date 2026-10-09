# AION2 Boss Alarm

A small Windows tray app that plays a sound before AION 2 field and abyss bosses respawn.

It reads the respawn timers the game server already sends to your PC (the same data the
in-game boss list shows) by watching your own network traffic with Npcap. It does not touch
the game process or its memory and never sends anything to the game server or anywhere else.

## How it works
- Open the in-game map or boss list once. The server answers with every boss timer as an
  absolute time, so the app can keep counting down and alarm with the map closed.
- When a boss is killed, open the map again to pick up its new respawn time.
- Per boss: switch the alarm on, choose how many minutes before the respawn it should ring
  (`5, 0` means 5 minutes before and at the respawn).
- Pick your own alarm sound (up to 3 MB) or use the built-in one. Closing the window keeps the
  app running in the tray.

## Requirements
- Windows 10/11
- [Npcap](https://npcap.com), installed with **"Install Npcap in WinPcap API-compatible Mode"**
  ticked. If the app cannot capture, run it as administrator.

## Build
```
npm install
npx tauri build      # MSI in src-tauri/target/release/bundle/msi
cd src-tauri && cargo test
```

## Notes
- How the boss list packet was decoded: [docs/boss-list-packet.md](../docs/boss-list-packet.md).
- Unofficial fan tool, not affiliated with NCSOFT. Whether passive network reading is
  acceptable under the game's terms is for you to judge; use at your own risk.
- Not yet known: the timer of "Watcher Kaira" in the abyss list (the packet holds none for it).

## License
GPL-3.0-or-later. See [LICENSE](LICENSE).

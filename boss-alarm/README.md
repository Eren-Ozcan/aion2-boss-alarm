# AION2 Boss Alarm

A small Windows app for AION 2: sound alarms before field and abyss bosses respawn, and a
DPS meter in a see-through overlay.

It reads data the game server already sends to your PC (the boss list timers, and combat
packets for what is around you) by watching your own network traffic with Npcap. It does not
touch the game process or its memory and sends nothing to the game server. The only requests
it makes are for skill icon images, to NCSOFT's public game data CDN (file names only, nothing
about you or your fights).

## How it works
- Open the in-game map or boss list once. The server answers with every boss timer as an
  absolute time, so the app can keep counting down and alarm with the map closed.
- When a boss is killed, open the map again to pick up its new respawn time.
- Per boss: switch the alarm on, choose how many minutes before the respawn it should ring
  (`5, 0` means 5 minutes before and at the respawn).
- Pick your own alarm sound (up to 3 MB) or use the built-in one. Closing the window keeps the
  app running in the tray.

## DPS meter
- A transparent, always-on-top overlay that opens at startup: per-player damage and DPS,
  class icons, boss HP and estimated fight length, ping. You are always shown, at your true
  rank, even outside the top rows. Rank colors: gold, silver, bronze, then two quiet greens;
  yours is always blue.
- Click a player (in the main window) for a per-skill breakdown with crit, back, double and
  parry rates and skill icons; a DPS timeline chart; boss fights are saved to a history.
- Target modes (Boss, Last hit, All, Training dummy), figures (DPS, damage, both), sort order,
  opacity, text size and row count are set from the overlay itself. The window position is
  remembered.
- Type your character name once (main window, DPS tab): the game names you only at login or
  zone change, so an app started later would show you as a bare id.
- "Record fight" in the DPS tab saves the raw game stream locally, for checking the numbers
  against another meter. It can contain chat, so it is never uploaded or committed.
- Fights, history and recordings stay on your PC. Battle uploads, accounts, Discord status and
  stream overlays of other meters are deliberately not included.

The packet parser and combat maths in `src-tauri/dps-core` are derived from
[A2Tools DPS Meter](https://github.com/taengu/A2Tools-DPS-Meter) (GPL-3.0); see
`src-tauri/dps-core/NOTICE.md` for what was taken and changed.

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

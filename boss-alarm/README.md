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
- Scheduled events (Shugo Festival at :00, Dimensional Invasion at :30, Spacetime Rift every
  three hours from 02:00, Abyss Siege on Mon, Thu and Sat at 21:00) have their own cards with a
  countdown and an alarm, 15 minutes ahead by default. The game does not send these times, so
  they come from the community timer sites and are in server time, set under "Server time UTC"
  (the default is UTC+2, the European server clock: the siege opens at 22:00 in Turkey). Check
  them against the in-game schedule.
- Bosses and events ring different sounds (a double beep and a rising chime by default), and
  each can be replaced with your own file.
- A boss that came up shows "Spawned 12 min ago" (when its timer ran out while the app was
  watching), in the main window and in the overlay. Bosses that are up also show as chips at
  the top of the DPS overlay.
- First run: a short notice must be accepted before anything is read from the network. If
  Npcap is missing, a step-by-step guide shows what to install.

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
- Boss is the default mode. A boss whose spawn the app never saw (it was started late) counts
  once eight players hit it, and its HP bar shows from the HP the game reports. Other players
  arrive mostly as damage-over-time ticks (the server sends your own hits, little of theirs), so
  their figures are partial; A2Tools reads the same data. "Show players with partial data" in the
  overlay settings hides players who have only DOT ticks.
- The overlay has a lock (clicks pass through to the game), a mute button, auto-hide when idle
  and options for ping, TTK, totals and pinning you at the top. Global shortcuts: Ctrl+Alt+K
  locks, Ctrl+Alt+J shows or hides, Ctrl+Alt+M mutes. The Settings tab has start with Windows,
  language and the same controls.
- Click a row in the overlay for a small card with what the game sent about that player:
  class, level, gear score and combat power (party members and you only), server, and the
  fight's damage, DPS and share. Fields the game did not send are left out.
- History: export the open fight as JSON (with skills) or CSV (per player). Files go to the
  app's `exports` folder, which opens after saving.
- Type your character name once (main window, DPS tab): the game names you only at login or
  zone change, so an app started later would show you as a bare id.
- "Record fight" in the DPS tab saves the raw game stream locally, for checking the numbers
  against another meter. It can contain chat, so it is never uploaded or committed.
- Fights, history and recordings stay on your PC. Battle uploads, accounts, Discord status and
  stream overlays of other meters are deliberately not included.

The packet parser and combat maths in `src-tauri/dps-core` are derived from
[A2Tools DPS Meter](https://github.com/taengu/A2Tools-DPS-Meter) (GPL-3.0); see
`src-tauri/dps-core/NOTICE.md` for what was taken and changed.

## Support
The footer of the main window has an "Open log folder" button. `app.log` holds what the app
did (start, capture status, exports), never chat, names or packet data.

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
- Not yet known: the timer of "Watcher Kaira" in the abyss list (the packet holds none for it).

## Windows SmartScreen
The installer is not code-signed yet, so Windows may show "Windows protected your PC". Choose
**More info**, then **Run anyway**. The in-app updates are signed separately and are not affected.

## Disclaimer
- Unofficial fan project. Not affiliated with, endorsed by or approved by NCSOFT or any
  AION 2 publisher. AION and all related names belong to their owners.
- The app only reads network packets passively (Npcap). It does not send packets, read or
  modify game memory, or inject into the game. This is a description of how it works, not a
  guarantee that the game's rules allow it.
- No permission from NCSOFT exists. Whether third-party tools like this are allowed under the
  game's terms of service or fair-play policy is unknown and may change at any time. Using it
  could lead to warnings, restrictions or account bans.
- You use this software entirely at your own risk. The authors accept no responsibility for
  account actions, lost data, or any other damage. It is provided "as is", without warranty
  of any kind (see the GPL-3.0 license).
- The class icons and the combat power icon are NCSOFT's artwork, used here as-is. They are
  not covered by the GPL-3.0 license of this project's code.
- If NCSOFT asks for this project to be taken down or changed, it will be.

## License
GPL-3.0-or-later. See [LICENSE](LICENSE).

## Author
[erenozcan.dev](https://erenozcan.dev) · [GitHub](https://github.com/Eren-Ozcan)

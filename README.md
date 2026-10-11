# BellMeter

A small Windows app for AION 2: sound alarms before field and abyss bosses respawn, and a
DPS meter in a see-through overlay. It only reads your own network traffic (Npcap) and never
touches the game process.

## Install
1. Npcap (free) is needed to read network traffic. The app offers an **Install Npcap** button
   on first start, or install it yourself from [npcap.com](https://npcap.com).
2. Download **`BellMeter_x.y.z_x64-setup.exe`** from the
   [latest release](https://github.com/Eren-Ozcan/bellmeter/releases/latest) and run it.
   It installs for your user only (no admin rights). Windows 10/11, 64-bit.
3. The installer is not code-signed yet, so Windows may warn about an unknown publisher:
   choose **More info**, then **Run anyway**.

The app updates itself when a new version is out.

## Repository layout
- [`boss-alarm/`](boss-alarm/) - the app (Tauri 2, Rust + plain JS). Full documentation:
  [boss-alarm/README.md](boss-alarm/README.md).
- [`docs/`](docs/) - protocol notes, e.g. [how the boss list packet was decoded](docs/boss-list-packet.md).
- [`tools/capture/`](tools/capture/) - capture and analysis helpers.

## Build
```
cd boss-alarm
npm install
npx tauri build      # setup.exe and .msi in src-tauri/target/release/bundle/
```
Releases are built by GitHub Actions when a `v*` tag is pushed.

## Disclaimer
Unofficial fan project, not affiliated with NCSOFT. Third-party tools may not be allowed by
the game's rules; use at your own risk. See the disclaimer in
[boss-alarm/README.md](boss-alarm/README.md#disclaimer).

## License
GPL-3.0-or-later. See [boss-alarm/LICENSE](boss-alarm/LICENSE).

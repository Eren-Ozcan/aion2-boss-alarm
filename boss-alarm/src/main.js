"use strict";

const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;

// ---------- text ----------
const TEXT = {
  en: {
    appName: "BellMeter",
    tabBosses: "Bosses",
    tabDps: "DPS",
    charName: "Character name",
    save: "Save",
    overlay: "Toggle overlay",
    tabSettings: "Settings",
    setStartup: "Startup",
    setAutostart: "Start with Windows",
    setOverlayStart: "Open the overlay at startup",
    setAlarms: "Alarms",
    setMute: "Mute all alarms",
    setMuteHint: "Timers keep running; nothing rings while muted.",
    setLanguage: "Language",
    langAuto: "Automatic (system)",
    setOverlay: "Overlay",
    setLock: "Lock / unlock overlay",
    setResetPos: "Reset overlay position",
    keyLock: "Lock the overlay, clicks pass through to the game",
    keyShow: "Show or hide the overlay",
    keyMute: "Mute or unmute alarms",
    soundCap: "Boss alarm sound",
    eventSoundCap: "Event alarm sound",
    events: "Events",
    serverTime: "Server time",
    every: "Every hour",
    nextAt: "Next",
    approaching: "Approaching",
    waiting: "Waiting",
    alarmOn: (m) => `ALARM · ${m} MIN`,
    alarmOff: "ALARM OFF",
    titles: {
      bosses: ["Bosses", "Timers sync when you open the in-game map or boss list. Alarms keep working with the map closed."],
      dps: ["DPS meter", "Click a player for a per-skill breakdown with crit, back and double rates."],
      history: ["Fight history", "Boss fights are saved automatically shortly after they end. Records stay on this PC."],
      settings: ["Settings", "Startup, alarms, language and the overlay. The overlay\u2019s own options are under its gear icon."],
    },
    timeline: "DPS timeline",
    tabHistory: "History",
    histEmpty: "No saved fights yet. Boss fights are saved automatically a short while after they end.",
    histBack: "Back",
    histDelete: "Delete this fight",
    dpsTotal: "Total",
    targetMode: "Target",
    modeBoss: "Boss",
    modeLast: "Last hit",
    modeAll: "All targets",
    modeTrain: "Training dummy",
    skill: "Skill",
    hits: "Hits",
    damage: "Damage",
    crit: "Crit",
    back: "Back",
    double: "Double",
    parry: "Parry",
    dotTag: "DoT",
    hpLeft: "HP",
    ttk: "Est. fight length",
    recStart: "Record fight (for checking numbers)",
    recStop: "Stop recording",
    recSaved: (p) => `Saved: ${p}. Local only, it can contain chat. Send it only to the developer.`,
    recOn: "Recording…",
    dpsEmpty: "No fight yet. Hit a boss or a training dummy and the numbers show up here.",
    dpsTarget: (n, t) => `${n || "Target"} · ${t}`,
    field: "Field bosses",
    abyss: "Abyss bosses",
    enableAll: "Enable all",
    disableAll: "Disable all",
    chooseSound: "Choose alarm sound…",
    defaultSound: "Default sound",
    test: "Test",
    volume: "Volume",
    soundDefault: "Using the default alarm sound",
    soundCustom: (n) => `Alarm sound: ${n}`,
    soundTooBig: "That file is too large (max 3 MB).",
    spawned: "Spawned",
    unknown: "Time unknown",
    spawnedAgo: (a) => `Spawned ${a} ago (now unknown)`,
    spawnedUp: (a) => `Spawned ${a} ago`,
    spawnedStale: (a) => `Spawned (seen ${a} ago, now unknown)`,
    never: "Not seen yet",
    minutesTip: "Minutes before respawn, e.g. 5, 0 (0 = at respawn)",
    alarmAt: (m) => (m === 0 ? "respawned" : `${m} min left`),
    st_Starting: "Starting…",
    st_WaitingForGame: "Waiting for AION 2…",
    st_Connected: "Connected, open the map to sync",
    st_Synced: "Synced",
    st_NpcapMissing: "Npcap is missing",
    st_CaptureFailed: "Capture failed",
    hintWaiting: "Start AION 2. Timers sync when you open the in-game map or boss list.",
    hintOpen: "Open the in-game map or boss list to sync timers. Alarms keep running with the map closed.",
    hintSynced: (ago) => `Last sync ${ago} ago. Open the map again after a boss is killed to get the new timer.`,
    npcap: "BellMeter reads game network traffic with Npcap. Install it from",
    npcapOpts: "and tick “Install Npcap in WinPcap API-compatible Mode”, then press Retry.",
    retry: "Retry",
    failed: "Capture could not start. Try running the app as administrator.",
    ago: (s) => (s < 60 ? `${s}s` : s < 3600 ? `${Math.floor(s / 60)} min` : `${Math.floor(s / 3600)} h ${Math.floor((s % 3600) / 60)} min`),
  },
  tr: {
    appName: "BellMeter",
    tabBosses: "Bosslar",
    tabDps: "DPS",
    charName: "Karakter adı",
    save: "Kaydet",
    overlay: "Overlay aç/kapat",
    tabSettings: "Ayarlar",
    setStartup: "Başlangıç",
    setAutostart: "Windows ile başlat",
    setOverlayStart: "Açılışta overlay'i aç",
    setAlarms: "Alarmlar",
    setMute: "Tüm alarmları sustur",
    setMuteHint: "Süreler işlemeye devam eder; sessizken hiçbir şey çalmaz.",
    setLanguage: "Dil",
    langAuto: "Otomatik (sistem)",
    setOverlay: "Overlay",
    setLock: "Overlay'i kilitle / aç",
    setResetPos: "Overlay konumunu sıfırla",
    keyLock: "Overlay'i kilitle, tıklamalar oyuna geçer",
    keyShow: "Overlay'i göster veya gizle",
    keyMute: "Alarmları sustur veya aç",
    soundCap: "Boss alarm sesi",
    eventSoundCap: "Etkinlik alarm sesi",
    events: "Etkinlikler",
    serverTime: "Sunucu saati",
    every: "Her saat",
    nextAt: "Sonraki",
    approaching: "Yaklaşıyor",
    waiting: "Bekliyor",
    alarmOn: (m) => `ALARM · ${m} DK`,
    alarmOff: "ALARM KAPALI",
    titles: {
      bosses: ["Bosslar", "Süreler oyundaki haritayı veya boss listesini açtığında senkronlanır. Harita kapalıyken de alarmlar çalışır."],
      dps: ["DPS ölçer", "Bir oyuncuya tıkla: beceri dökümü, kritik, arka ve çift vuruş oranları."],
      history: ["Savaş geçmişi", "Boss savaşları bittikten kısa süre sonra otomatik kaydedilir. Kayıtlar bu bilgisayarda kalır."],
      settings: ["Ayarlar", "Başlangıç, alarmlar, dil ve overlay. Overlay'in kendi seçenekleri üstündeki dişli simgesinde."],
    },
    timeline: "DPS zaman çizelgesi",
    tabHistory: "Geçmiş",
    histEmpty: "Kayıtlı savaş yok. Boss savaşları bittikten kısa süre sonra otomatik kaydedilir.",
    histBack: "Geri",
    histDelete: "Bu savaşı sil",
    dpsTotal: "Toplam",
    targetMode: "Hedef",
    modeBoss: "Boss",
    modeLast: "Son vuruş",
    modeAll: "Tüm hedefler",
    modeTrain: "Antrenman kuklası",
    skill: "Beceri",
    hits: "Vuruş",
    damage: "Hasar",
    crit: "Kritik",
    back: "Arkadan",
    double: "Çift",
    parry: "Savuşturma",
    dotTag: "DoT",
    hpLeft: "Can",
    ttk: "Tahmini savaş süresi",
    recStart: "Savaşı kaydet (sayıları kontrol için)",
    recStop: "Kaydı durdur",
    recSaved: (p) => `Kaydedildi: ${p}. Sadece yerelde, sohbet içerebilir. Sadece geliştiriciye gönder.`,
    recOn: "Kaydediliyor…",
    dpsEmpty: "Henüz savaş yok. Bir bosla ya da antrenman kuklasıyla dövüşünce sayılar burada görünür.",
    dpsTarget: (n, t) => `${n || "Hedef"} · ${t}`,
    field: "Saha bossları",
    abyss: "Abyss bossları",
    enableAll: "Hepsini aç",
    disableAll: "Hepsini kapat",
    chooseSound: "Alarm sesi seç…",
    defaultSound: "Varsayılan ses",
    test: "Dene",
    volume: "Ses",
    soundDefault: "Varsayılan alarm sesi kullanılıyor",
    soundCustom: (n) => `Alarm sesi: ${n}`,
    soundTooBig: "Dosya çok büyük (en fazla 3 MB).",
    spawned: "Doğdu",
    unknown: "Süre bilinmiyor",
    spawnedAgo: (a) => `${a} önce doğdu (şu an bilinmiyor)`,
    spawnedUp: (a) => `${a} önce doğdu`,
    spawnedStale: (a) => `Doğmuştu (${a} önce görüldü, şu an bilinmiyor)`,
    never: "Henüz görülmedi",
    minutesTip: "Doğmadan kaç dakika önce, örn. 5, 0 (0 = doğduğu an)",
    alarmAt: (m) => (m === 0 ? "doğdu" : `${m} dk kaldı`),
    st_Starting: "Başlıyor…",
    st_WaitingForGame: "AION 2 bekleniyor…",
    st_Connected: "Bağlı, eşitlemek için haritayı aç",
    st_Synced: "Eşitlendi",
    st_NpcapMissing: "Npcap yok",
    st_CaptureFailed: "Yakalama başlamadı",
    hintWaiting: "AION 2'yi aç. Süreler, oyun içi haritayı veya boss listesini açınca eşitlenir.",
    hintOpen: "Süreleri eşitlemek için oyun içi haritayı veya boss listesini aç. Harita kapalıyken de alarmlar çalışır.",
    hintSynced: (ago) => `Son eşitleme ${ago} önce. Bir boss kesilince yeni süreyi almak için haritayı tekrar aç.`,
    npcap: "BellMeter oyun ağ trafiğini Npcap ile okur. Şuradan kur:",
    npcapOpts: "ve kurulumda “Install Npcap in WinPcap API-compatible Mode” kutusunu işaretle, sonra Yeniden dene'ye bas.",
    retry: "Yeniden dene",
    failed: "Yakalama başlatılamadı. Uygulamayı yönetici olarak çalıştırmayı dene.",
    ago: (s) => (s < 60 ? `${s} sn` : s < 3600 ? `${Math.floor(s / 60)} dk` : `${Math.floor(s / 3600)} sa ${Math.floor((s % 3600) / 60)} dk`),
  },
};
const chosenLang = (() => { try { return localStorage.getItem("lang"); } catch { return null; } })();
const lang = chosenLang === "tr" || chosenLang === "en" ? chosenLang : (navigator.language || "en").toLowerCase().startsWith("tr") ? "tr" : "en";
const T = TEXT[lang];
document.documentElement.lang = lang;
for (const el of document.querySelectorAll("[data-i18n]")) el.textContent = T[el.dataset.i18n];

// ---------- storage (per-viewer conveniences only) ----------
const store = {
  get(k) { try { return localStorage.getItem(k); } catch { return null; } },
  set(k, v) { try { localStorage.setItem(k, v); } catch { /* storage unavailable */ } },
  del(k) { try { localStorage.removeItem(k); } catch { /* ignore */ } },
};

// ---------- sound ----------
let audioCtx = null;
const customAudio = { boss: null, event: null }; // HTMLAudioElement per kind
const SOUND_KEYS = { boss: ["soundData", "soundName", "soundName"], event: ["eventSoundData", "eventSoundName", "eventSoundName"] };
const volume = () => Math.min(1, Math.max(0, Number(store.get("volume") ?? 80) / 100));

function beep(ctx, at, freq, len, vol) {
  const o = ctx.createOscillator();
  const g = ctx.createGain();
  o.type = "sine";
  o.frequency.value = freq;
  g.gain.setValueAtTime(0.0001, at);
  g.gain.exponentialRampToValueAtTime(Math.max(vol, 0.0002), at + 0.02);
  g.gain.exponentialRampToValueAtTime(0.0001, at + len);
  o.connect(g).connect(ctx.destination);
  o.start(at);
  o.stop(at + len + 0.05);
}

// Bosses ring a two-tone double beep; events a rising chime, so they can be told apart by ear.
function playDefault(kind = "boss") {
  audioCtx ??= new AudioContext();
  const ctx = audioCtx;
  if (ctx.state === "suspended") ctx.resume();
  const t = ctx.currentTime + 0.02;
  const v = volume() * 0.5;
  if (kind === "event") [523, 659, 784, 1047].forEach((f, i) => beep(ctx, t + i * 0.16, f, 0.34, v));
  else [880, 1175, 880, 1175].forEach((f, i) => beep(ctx, t + i * 0.28, f, 0.22, v));
}

function playAlarm(kind = "boss") {
  const audio = customAudio[kind];
  if (audio) {
    audio.volume = volume();
    audio.currentTime = 0;
    audio.play().catch(() => playDefault(kind));
  } else {
    playDefault(kind);
  }
}

function loadCustomSound() {
  for (const kind of ["boss", "event"]) {
    const [dataKey, nameKey] = SOUND_KEYS[kind];
    const data = store.get(dataKey);
    const name = store.get(nameKey);
    const label = $(kind === "boss" ? "soundName" : "eventSoundName");
    if (data) {
      customAudio[kind] = new Audio(data);
      label.textContent = T.soundCustom(name || "custom");
    } else {
      customAudio[kind] = null;
      label.textContent = T.soundDefault;
    }
  }
}

// ---------- state ----------
const $ = (id) => document.getElementById(id);
let snap = null;
let editing = false;

function toast(text) {
  const el = document.createElement("div");
  el.className = "toast";
  el.textContent = text;
  $("toasts").append(el);
  setTimeout(() => el.remove(), 8000);
}

function fmt(ms) {
  const s = Math.max(0, Math.floor(ms / 1000));
  const h = Math.floor(s / 3600);
  const m = Math.floor((s % 3600) / 60);
  const sec = s % 60;
  const p = (n) => String(n).padStart(2, "0");
  return h >= 24
    ? `${Math.floor(h / 24)}d ${p(h % 24)}:${p(m)}:${p(sec)}`
    : `${p(h)}:${p(m)}:${p(sec)}`;
}

function parseMinutes(text) {
  const out = [...new Set(text.split(/[\s,;]+/).filter(Boolean).map(Number))]
    .filter((n) => Number.isInteger(n) && n >= 0 && n <= 1440);
  return out.sort((a, b) => b - a);
}

// What a boss card shows: the big figure, the line under it, a colour class and the progress bar.
function bossState(b, now) {
  const secs = (ms) => Math.max(0, Math.round(ms / 1000));
  const since = (t) => T.ago(secs(now - t));
  if (b.spawned) {
    // Up when last seen; after a while we cannot know whether it was killed.
    const age = b.seen_local_ms ? now - b.seen_local_ms : 0;
    const spawnedAgo = b.spawned_at_local_ms != null ? since(b.spawned_at_local_ms) : null;
    if (age > 60_000) {
      return { time: T.spawned, status: spawnedAgo ? T.spawnedAgo(spawnedAgo) : T.spawnedStale(T.ago(secs(age))), cls: "up-unknown", prog: 100 };
    }
    return { time: T.spawned, status: spawnedAgo ? T.spawnedUp(spawnedAgo) : T.spawned, cls: "spawned", prog: 100 };
  }
  if (b.respawn_local_ms == null) return { time: "--:--", status: b.seen_local_ms ? T.unknown : T.never, cls: "unknown", prog: 0 };
  const left = b.respawn_local_ms - now;
  // The respawn time passed while we were not watching: it did spawn, but
  // whether it is still up is unknown until the map is opened again.
  if (left <= 0) return { time: T.spawned, status: T.spawnedAgo(since(b.respawn_local_ms)), cls: "up-unknown", prog: 100 };
  const soon = left < 10 * 60_000;
  return { time: fmt(left), status: soon ? T.approaching : T.waiting, cls: left < 5 * 60_000 ? "soon" : "", prog: Math.max(2, 100 - (left / 21_600_000) * 100) };
}

function buildRow(b) {
  const row = document.createElement("div");
  row.className = "boss";
  row.dataset.id = b.id;

  const top = document.createElement("div");
  top.className = "b-top";
  const name = document.createElement("span");
  name.className = "name";
  name.textContent = b.name;
  name.title = b.name;

  const sw = document.createElement("label");
  sw.className = "switch";
  const cb = document.createElement("input");
  cb.type = "checkbox";
  cb.setAttribute("aria-label", b.name);
  const track = document.createElement("span");
  sw.append(cb, track);
  top.append(name, sw);

  const mid = document.createElement("div");
  mid.className = "b-mid";
  const state = document.createElement("span");
  state.className = "state";
  const mins = document.createElement("input");
  mins.className = "minutes";
  mins.type = "text";
  mins.inputMode = "numeric";
  mins.title = T.minutesTip;
  mins.setAttribute("aria-label", `${b.name}: ${T.minutesTip}`);
  mid.append(state, mins);

  const bar = document.createElement("div");
  bar.className = "b-bar";
  bar.append(document.createElement("i"));
  const status = document.createElement("div");
  status.className = "b-status";

  const save = () => {
    const minutes = parseMinutes(mins.value);
    invoke("set_alarm", { id: b.id, enabled: cb.checked, minutes: minutes.length ? minutes : [0] });
  };
  cb.addEventListener("change", save);
  mins.addEventListener("focus", () => (editing = true));
  mins.addEventListener("blur", () => { editing = false; save(); });
  mins.addEventListener("keydown", (e) => { if (e.key === "Enter") mins.blur(); });

  row.append(top, mid, bar, status);
  return row;
}

function render() {
  if (!snap) return;
  const now = Date.now();
  for (const b of snap.bosses) {
    const list = $(b.group);
    let row = list.querySelector(`[data-id="${b.id}"]`);
    if (!row) { row = buildRow(b); list.append(row); }
    const cb = row.querySelector("input[type=checkbox]");
    const mins = row.querySelector(".minutes");
    cb.checked = b.alarm.enabled;
    mins.disabled = !b.alarm.enabled;
    if (document.activeElement !== mins) mins.value = b.alarm.minutes.join(", ");
    row.classList.toggle("on", b.alarm.enabled);
    row.style.order = b.alarm.enabled ? 0 : 1;
    const st = bossState(b, now);
    const el = row.querySelector(".state");
    el.textContent = st.time;
    el.className = "state " + (st.cls === "soon" && !b.alarm.enabled ? "" : st.cls);
    row.classList.toggle("is-up", st.cls === "spawned" || st.cls === "up-unknown");
    row.classList.toggle("is-soon", st.cls === "soon" && b.alarm.enabled);
    row.querySelector(".b-bar i").style.width = `${st.prog}%`;
    row.querySelector(".b-status").textContent = b.alarm.enabled ? `${st.status} · ${T.alarmOn(b.alarm.minutes.join(", "))}` : `${st.status} · ${T.alarmOff}`;
  }
  // Scheduled events: counted down to the next start, and alarmed like a boss.
  const evBox = $("events");
  const clock = (ms) => new Date(ms).toLocaleTimeString(lang === "tr" ? "tr-TR" : undefined, { hour: "2-digit", minute: "2-digit" });
  for (const e of snap.events ?? []) {
    let row = evBox.querySelector(`[data-id="${e.id}"]`);
    if (!row) { row = buildRow(e); row.classList.add("event"); evBox.append(row); }
    const cb = row.querySelector("input[type=checkbox]");
    const mins = row.querySelector(".minutes");
    cb.checked = e.alarm.enabled;
    mins.disabled = !e.alarm.enabled;
    if (document.activeElement !== mins) mins.value = e.alarm.minutes.join(", ");
    row.classList.toggle("on", e.alarm.enabled);
    row.style.order = e.alarm.enabled ? 0 : 1;
    const left = e.next_local_ms - now;
    const soon = left < 5 * 60_000;
    const el = row.querySelector(".state");
    el.textContent = fmt(left);
    el.className = "state " + (soon && e.alarm.enabled ? "soon" : "") + (el.textContent.length > 9 ? " small" : "");
    row.classList.toggle("is-soon", soon && e.alarm.enabled);
    row.querySelector(".b-bar i").style.width = `${Math.max(2, 100 - (left / e.period_ms) * 100)}%`;
    const when = e.rule.startsWith(":") ? `${T.every} ${e.rule}` : e.rule;
    row.querySelector(".b-status").textContent = `${when} · ${T.nextAt} ${clock(e.next_local_ms)}` + (e.alarm.enabled ? ` · ${T.alarmOn(e.alarm.minutes.join(", "))}` : ` · ${T.alarmOff}`);
  }
  if (document.activeElement !== $("serverOffset")) $("serverOffset").value = snap.server_offset_h ?? 9;

  if (document.activeElement !== $("muteAll")) $("muteAll").checked = !!snap.muted;
  if (document.activeElement !== $("overlayStart")) $("overlayStart").checked = snap.overlay_at_startup !== false;
  const on = snap.bosses.filter((x) => x.alarm.enabled).length;
  $("metaBosses").textContent = String(on);
  renderStatus(now);
}

function renderStatus(now) {
  const s = snap.status;
  const chip = $("status");
  const setup = $("setup");
  setup.hidden = true;
  let cls = "";
  let text = T["st_" + s.kind] ?? s.kind;
  if (s.kind === "Connected" && snap.last_list_local_ms) { text = T.st_Synced; cls = "ok"; }
  else if (s.kind === "Connected") cls = "warn";
  else if (s.kind === "NpcapMissing" || s.kind === "CaptureFailed") cls = "bad";
  else cls = "warn";
  chip.textContent = text;
  chip.className = "chip " + cls;

  setup.classList.remove("guide");
  if (s.kind === "NpcapMissing") {
    setup.hidden = false;
    // extras.js loads after this file; the next render (1 s) draws the guide.
    if (typeof renderNpcapGuide === "function") renderNpcapGuide(setup, () => invoke("retry_capture"));
  } else if (s.kind === "CaptureFailed") {
    setup.hidden = false;
    setup.replaceChildren();
    const p = document.createElement("div");
    p.textContent = `${T.failed} (${s.detail})`;
    const btn = document.createElement("button");
    btn.className = "btn";
    btn.textContent = T.retry;
    btn.onclick = () => invoke("retry_capture");
    setup.append(p, btn);
  }

  const hint = $("hint");
  if (s.kind === "WaitingForGame" || s.kind === "Starting") hint.textContent = T.hintWaiting;
  else if (snap.last_list_local_ms) hint.textContent = T.hintSynced(T.ago(Math.max(0, Math.floor((now - snap.last_list_local_ms) / 1000))));
  else hint.textContent = T.hintOpen;
}

// ---------- wiring ----------
$("enableAll").onclick = () => invoke("set_all_enabled", { enabled: true });
$("disableAll").onclick = () => invoke("set_all_enabled", { enabled: false });
$("volume").value = store.get("volume") ?? 80;
$("volume").oninput = (e) => store.set("volume", e.target.value);
function wireSound(kind, ids) {
  const [dataKey, nameKey] = SOUND_KEYS[kind];
  $(ids.test).onclick = () => playAlarm(kind);
  $(ids.reset).onclick = () => { store.del(dataKey); store.del(nameKey); loadCustomSound(); };
  $(ids.file).onchange = (e) => {
    const file = e.target.files[0];
    if (!file) return;
    if (file.size > 3 * 1024 * 1024) { toast(T.soundTooBig); return; }
    const reader = new FileReader();
    reader.onload = () => {
      store.set(dataKey, reader.result);
      store.set(nameKey, file.name);
      loadCustomSound();
      playAlarm(kind);
    };
    reader.readAsDataURL(file);
  };
}
wireSound("boss", { test: "testSound", reset: "resetSound", file: "soundFile" });
wireSound("event", { test: "eventTestSound", reset: "eventResetSound", file: "eventSoundFile" });
$("muteAll").onchange = (e) => invoke("set_muted", { muted: e.target.checked });
$("overlayStart").onchange = (e) => invoke("set_overlay_at_startup", { on: e.target.checked });
$("autostart").onchange = (e) => invoke("set_autostart", { on: e.target.checked }).catch(() => { e.target.checked = !e.target.checked; });
invoke("get_autostart").then((on) => { $("autostart").checked = !!on; }).catch(() => {});
$("langSel").value = chosenLang === "tr" || chosenLang === "en" ? chosenLang : "auto";
$("langSel").onchange = (e) => {
  if (e.target.value === "auto") store.del("lang"); else store.set("lang", e.target.value);
  location.reload();
};
$("setLock").onclick = () => invoke("toggle_overlay_lock").catch(() => {});
$("setResetPos").onclick = () => invoke("reset_overlay_position").catch(() => {});
$("serverOffset").onchange = (e) => {
  const h = Math.max(-12, Math.min(14, Math.round(Number(e.target.value))));
  if (Number.isFinite(h)) invoke("set_server_offset", { hours: h });
};

listen("snapshot", (e) => { snap = e.payload; render(); });
listen("alarm", (e) => {
  const a = e.payload;
  playAlarm(a.kind === "event" ? "event" : "boss");
  toast(`${a.name}: ${T.alarmAt(a.minutes)}`);
});

loadCustomSound();
invoke("get_snapshot").then((s) => { snap = s; render(); });
setInterval(() => { if (!editing) render(); }, 1000);


// ---------- DPS tab ----------
const views = { bosses: document.getElementById("viewBosses"), dps: document.getElementById("viewDps"), history: document.getElementById("viewHistory"), settings: document.getElementById("viewSettings") };
const tabs = { bosses: document.getElementById("tabBosses"), dps: document.getElementById("tabDps"), history: document.getElementById("tabHistory"), settings: document.getElementById("tabSettings") };
let dpsTimer = null;

function showTab(name) {
  $("tabTitle").textContent = T.titles[name][0];
  $("tabHint").textContent = T.titles[name][1];
  for (const k of Object.keys(views)) {
    views[k].hidden = k !== name;
    tabs[k].classList.toggle("active", k === name);
    tabs[k].setAttribute("aria-selected", String(k === name));
  }
  clearInterval(dpsTimer);
  dpsTimer = null;
  if (name === "dps") {
    refreshDps();
    dpsTimer = setInterval(refreshDps, 1000);
  } else if (name === "history") {
    refreshHistory();
  }
}
tabs.bosses.addEventListener("click", () => showTab("bosses"));
tabs.dps.addEventListener("click", () => showTab("dps"));
tabs.history.addEventListener("click", () => showTab("history"));
tabs.settings.addEventListener("click", () => showTab("settings"));

const nf = new Intl.NumberFormat();

// The game names classes in Korean; map them to an icon file and a readable name.
const JOBS = {
  "검성": ["gladiator", "Gladiator"], "수호성": ["templar", "Templar"],
  "살성": ["assassin", "Assassin"], "궁성": ["ranger", "Ranger"],
  "마도성": ["sorcerer", "Sorcerer"], "정령성": ["elementalist", "Elementalist"],
  "치유성": ["cleric", "Cleric"], "호법성": ["chanter", "Chanter"],
  "권성": ["fighter", "Fighter"],
};

// Players whose name the game never sent show as their numeric id.
const displayName = (p) => (/^\d+$/.test(p.nickname) ? `#${p.nickname}` : p.nickname || "?");

const mmss = (ms) => {
  const s = Math.max(0, Math.round(ms / 1000));
  return `${Math.floor(s / 60)}:${String(s % 60).padStart(2, "0")}`;
};

// Time-to-kill estimate, after the estimator in A2Tools DPS Meter (GPL-3.0,
// https://github.com/taengu/A2Tools-DPS-Meter): the rate comes from the boss's
// own HP over the last 20 s, smoothed so a burst does not cut the estimate short.
function createTtk({ windowMs = 20000, riseTau = 30000, fallTau = 10000, minElapsed = 5000, maxTotal = 99 * 60000 } = {}) {
  let id = null, samples = [], rate = null, lastT = null;
  const reset = (n = null) => { id = n; samples = []; rate = null; lastT = null; };
  const update = (tid, t, hp, max) => {
    if (!(max > 0) || !(hp >= 0) || !Number.isFinite(t)) return null;
    if (tid !== id || (lastT !== null && t < lastT)) reset(tid);
    if (hp <= 0) return null;
    const last = samples[samples.length - 1];
    if (last && hp > last.hp) samples = [];
    if (last && samples.length && t === last.t) last.hp = hp; else samples.push({ t, hp });
    while (samples.length > 2 && samples[1].t <= t - windowMs) samples.shift();
    if (t < minElapsed) { lastT = t; return null; }
    const first = samples[0];
    const span = t - first.t;
    const avg = (max - hp) / t;
    const observed = span >= windowMs / 2 ? (first.hp - hp) / span : avg;
    if (rate === null) {
      const seed = avg > 0 ? avg : observed;
      if (seed > 0) rate = seed;
    } else if (observed >= 0 && lastT !== null && t > lastT) {
      const tau = observed > rate ? riseTau : fallTau;
      rate += (1 - Math.exp(-(t - lastT) / tau)) * (observed - rate);
    }
    lastT = t;
    if (!(rate > 0)) return null;
    const total = t + hp / rate;
    return total <= maxTotal ? total : null;
  };
  return { update };
}
const ttk = createTtk();

const expanded = new Set(); // actor ids whose skill table is open

const modeSel = document.getElementById("targetMode");
modeSel.value = store.get("targetMode") || "bossTargets";
const applyMode = () => { store.set("targetMode", modeSel.value); invoke("set_target_mode", { mode: modeSel.value }); };
modeSel.addEventListener("change", () => { applyMode(); refreshDps(); });
applyMode();

const pct = (n, d) => (d > 0 ? `${((n / d) * 100).toFixed(0)}%` : "-");

function cell(text, cls) {
  const c = document.createElement("div");
  if (cls) c.className = cls;
  c.textContent = text;
  return c;
}

// Rebuilding rows every second would drop keyboard focus; put it back on the
// same player's row afterwards.
function replaceKeepingFocus(box, nodes) {
  const focused = box.contains(document.activeElement) ? document.activeElement.dataset?.actor : undefined;
  box.replaceChildren(...nodes);
  if (focused !== undefined) box.querySelector(`[data-actor="${CSS.escape(focused)}"]`)?.focus({ preventScroll: true });
}

function skillTable(res, actorId, actorTotal) {
  const box = document.createElement("div");
  box.className = "skills";
  const head = ["skill", "hits", "damage", "", "crit", "back", "double", "parry"];
  const addRow = (cells, cls, skill) => {
    const r = document.createElement("div");
    r.className = `skill-row ${cls ?? ""}`;
    cells.forEach((c, i) => {
      if (i !== 0) return r.append(cell(c, "num"));
      const first = document.createElement("div");
      first.className = "sk-name";
      if (skill) first.append(skillIconImg(skill));
      const label = document.createElement("span");
      label.textContent = c;
      first.append(label);
      r.append(first);
    });
    box.append(r);
  };
  addRow(head.map((k) => (k ? T[k] : "%")), "skill-head");
  const list = (res.skills ?? [])
    .filter((s) => s.actorId === actorId && (s.dmg > 0 || s.time > 0))
    .sort((a, b) => b.dmg - a.dmg);
  for (const s of list) {
    addRow([
      s.isDot ? `${s.name} (${T.dotTag})` : s.name || `#${s.code}`,
      nf.format(s.time), nf.format(s.dmg), pct(s.dmg, actorTotal),
      pct(s.crit, s.time), pct(s.back, s.time), pct(s.double, s.time), pct(s.parry, s.time),
    ], "", s);
  }
  return box;
}

const chartBox = document.getElementById("chartBox");
let chartTick = 0;
async function refreshChart(d) {
  if (!chartBox.open || chartTick++ % 2) return; // every other tick: the rows are cheap, this is not
  let res;
  try { res = await invoke("get_all_skills", { targetId: d.targetId }); } catch { return; }
  const names = new Map(Object.entries(d.map ?? {}).map(([id, p]) => [Number(id), displayName(p)]));
  const node = timelineChart(res, (id) => names.get(id) ?? `#${id}`);
  document.getElementById("chart").replaceChildren(...(node ? [node] : []));
}
chartBox.addEventListener("toggle", () => { chartTick = 0; });

let refreshing = false;
async function refreshDps() {
  if (refreshing) return;
  refreshing = true;
  try {
    const d = await invoke("get_dps");
    const entries = Object.entries(d.map ?? {})
      .map(([id, p]) => ({ id: Number(id), p }))
      .sort((a, b) => b.p.amount - a.p.amount);
    const details = new Map();
    for (const id of expanded) {
      try { details.set(id, await invoke("get_skills", { targetId: d.targetId, actorId: id })); } catch { /* skip */ }
    }
    renderDps(d, entries, details);
    await refreshChart(d);
  } catch { /* backend not ready */ } finally { refreshing = false; }
}

function renderDps(d, entries, details) {
  // Other players arrive mostly as damage-over-time ticks; they are left out unless the overlay setting shows them.
  const anyDirect = entries.some((e) => e.p.direct !== false);
  if (store.get("showPartial") === "0" && anyDirect) entries = entries.filter((e) => e.p.direct !== false || e.id === d.localPlayerId);
  const shown = entries.reduce((sum, e) => sum + e.p.amount, 0) || 1;
  for (const e of entries) e.p.damageContribution = (e.p.amount / shown) * 100;
  const box = document.getElementById("dpsRows");
  document.getElementById("dpsEmpty").hidden = entries.length > 0;
  const total = entries.reduce((s, e) => s + e.p.amount, 0);

  // HP bar and estimated fight length, for a boss with a known HP pool.
  const hpBox = document.getElementById("hp");
  let tail = "";
  if (entries.length && d.targetIsBoss && d.targetMaxHp > 0) {
    const left = Math.max(0, d.targetCurrentHp >= 0 ? d.targetCurrentHp : d.targetMaxHp - d.targetTotalDamage);
    hpBox.hidden = false;
    document.getElementById("hpFill").style.width = `${Math.min(100, (left / d.targetMaxHp) * 100)}%`;
    const est = ttk.update(d.targetId, d.battleTime, left, d.targetMaxHp);
    tail = ` · ${T.hpLeft} ${((left / d.targetMaxHp) * 100).toFixed(0)}%` + (est ? ` · ${T.ttk} ${mmss(est)}` : "");
  } else {
    hpBox.hidden = true;
  }
  document.getElementById("dpsTarget").textContent = entries.length
    ? `${d.targetName || (d.targetId > 0 ? `MOB #${d.targetId}` : "?")} · ${mmss(d.battleTime)} · ${T.dpsTotal} ${nf.format(Math.round(total))}${tail}`
    : "";

  const top = entries[0]?.p.amount || 1;
  const nodes = [];
  for (const { id, p } of entries) {
    const row = document.createElement("div");
    row.className = "dps-row";
    row.tabIndex = 0;
    row.dataset.actor = String(id);
    row.setAttribute("role", "button");
    row.setAttribute("aria-expanded", String(expanded.has(id)));
    const bar = document.createElement("div");
    bar.className = "bar";
    bar.style.width = `${Math.max(2, (p.amount / top) * 100)}%`;
    const who = document.createElement("div");
    who.className = "who";
    const job = JOBS[p.job];
    if (job) {
      const img = document.createElement("img");
      img.className = "job-icon";
      img.src = `assets/classes/${job[0]}.png`;
      img.alt = job[1];
      img.title = job[1];
      who.append(img);
    }
    who.append(cell(displayName(p)));
    row.append(bar, who, cell(nf.format(Math.round(p.amount)), "num total"),
      cell(nf.format(Math.round(p.dps)), "num"), cell(`${p.damageContribution.toFixed(1)}%`, "share"));
    const toggle = () => {
      if (expanded.has(id)) expanded.delete(id); else expanded.add(id);
      refreshDps();
    };
    row.addEventListener("click", toggle);
    row.addEventListener("keydown", (e) => { if (e.key === "Enter" || e.key === " ") { e.preventDefault(); toggle(); } });
    nodes.push(row);
    if (expanded.has(id) && details.has(id)) nodes.push(skillTable(details.get(id), id, p.amount));
  }
  replaceKeepingFocus(box, nodes);
}


// ---------- History tab ----------
const histList = document.getElementById("histList");
const histDetail = document.getElementById("histDetail");
const histEmpty = document.getElementById("histEmpty");
let openFight = null; // the record being viewed
const openActors = new Set();

const when = (ms) => new Date(ms).toLocaleString(lang === "tr" ? "tr-TR" : undefined, { dateStyle: "short", timeStyle: "short" });

function jobIcon(job) {
  const j = JOBS[job];
  if (!j) return null;
  const img = document.createElement("img");
  img.className = "job-icon";
  img.src = `assets/classes/${j[0]}.png`;
  img.alt = j[1];
  img.title = j[1];
  return img;
}

async function refreshHistory() {
  if (openFight) return;
  let list = [];
  try { list = await invoke("list_fights"); } catch { /* none yet */ }
  list.sort((a, b) => b.startTimeMs - a.startTimeMs);
  histEmpty.hidden = list.length > 0;
  histList.replaceChildren(...list.map((f) => {
    const row = document.createElement("div");
    row.className = "hist-row";
    row.tabIndex = 0;
    row.setAttribute("role", "button");
    const icons = document.createElement("div");
    icons.className = "hist-icons";
    for (const job of f.memberJobs?.length ? f.memberJobs : f.jobs ?? []) {
      const img = jobIcon(job);
      if (img) icons.append(img);
    }
    const name = document.createElement("div");
    name.className = "hist-name";
    name.append(cell(f.bossName || "?"), cell(when(f.startTimeMs), "muted"));
    const open = async () => {
      try { openFight = await invoke("load_fight", { id: f.id }); } catch { return; }
      openActors.clear();
      showFight();
    };
    row.append(name, icons, cell(mmss(f.durationMs), "num"), cell(nf.format(f.totalDamage), "num total"));
    row.addEventListener("click", open);
    row.addEventListener("keydown", (e) => { if (e.key === "Enter" || e.key === " ") { e.preventDefault(); open(); } });
    return row;
  }));
}

// Per-player totals from a saved fight's skill rows.
function fightPlayers(rec) {
  const damage = new Map();
  for (const s of rec.details?.skills ?? []) damage.set(s.actorId, (damage.get(s.actorId) ?? 0) + s.dmg);
  const total = [...damage.values()].reduce((a, b) => a + b, 0);
  const secs = Math.max(1, (rec.durationMs || rec.details?.battleTime || 1000) / 1000);
  return [...damage.entries()]
    .map(([id, dmg]) => {
      const a = (rec.actors ?? []).find((x) => x.actorId === id) ?? {};
      return { id, dmg, total, dps: dmg / secs, nickname: a.nickname ?? String(id), job: a.job ?? "" };
    })
    .sort((x, y) => y.dmg - x.dmg);
}

function showFight() {
  const rec = openFight;
  document.getElementById("histListBox").hidden = true;
  histDetail.hidden = false;
  document.getElementById("histTitle").textContent =
    `${rec.bossName || "?"} · ${when(rec.startTimeMs)} · ${mmss(rec.durationMs)} · ${T.dpsTotal} ${nf.format(rec.totalDamage)}`;
  const players = fightPlayers(rec);
  const top = players[0]?.dmg || 1;
  const nodes = [];
  for (const p of players) {
    const row = document.createElement("div");
    row.className = "dps-row";
    row.tabIndex = 0;
    row.setAttribute("role", "button");
    const bar = document.createElement("div");
    bar.className = "bar";
    bar.style.width = `${Math.max(2, (p.dmg / top) * 100)}%`;
    const who = document.createElement("div");
    who.className = "who";
    const img = jobIcon(p.job);
    if (img) who.append(img);
    who.append(cell(displayName(p)));
    row.append(bar, who, cell(nf.format(p.dmg), "num total"), cell(nf.format(Math.round(p.dps)), "num"),
      cell(`${((p.dmg / (p.total || 1)) * 100).toFixed(1)}%`, "share"));
    const toggle = () => { if (openActors.has(p.id)) openActors.delete(p.id); else openActors.add(p.id); showFight(); };
    row.addEventListener("click", toggle);
    row.addEventListener("keydown", (e) => { if (e.key === "Enter" || e.key === " ") { e.preventDefault(); toggle(); } });
    nodes.push(row);
    if (openActors.has(p.id)) nodes.push(skillTable(rec.details, p.id, p.dmg));
  }
  document.getElementById("histRows").replaceChildren(...nodes);
  const names = new Map((rec.actors ?? []).map((a) => [a.actorId, displayName({ nickname: a.nickname ?? String(a.actorId) })]));
  const chart = timelineChart(rec.details, (id) => names.get(id) ?? `#${id}`);
  document.getElementById("histChart").replaceChildren(...(chart ? [chart] : []));
}

function closeFight() {
  openFight = null;
  histDetail.hidden = true;
  document.getElementById("histListBox").hidden = false;
  refreshHistory();
}
document.getElementById("histBack").addEventListener("click", closeFight);
document.getElementById("histDelete").addEventListener("click", async () => {
  if (!openFight) return;
  try { await invoke("delete_fight", { id: openFight.id }); } catch { return; }
  closeFight();
});

// ---------- recording ----------
const recBtn = document.getElementById("recBtn");
const recInfo = document.getElementById("recInfo");
let recording = false;
recBtn.addEventListener("click", async () => {
  try {
    if (!recording) {
      await invoke("start_recording");
      recording = true;
      recBtn.textContent = T.recStop;
      recInfo.textContent = T.recOn;
    } else {
      const path = await invoke("stop_recording");
      recording = false;
      recBtn.textContent = T.recStart;
      recInfo.textContent = path ? T.recSaved(path) : "";
    }
  } catch (e) {
    recInfo.textContent = String(e);
  }
});

// ---------- overlay ----------
document.getElementById("overlayBtn").addEventListener("click", () => {
  invoke("toggle_overlay").catch((e) => { document.getElementById("recInfo").textContent = String(e); });
});
// The window is always the dark Aether theme; the stored choice of the old selector is dropped.
store.del("theme");

// ---------- character name ----------
const charName = document.getElementById("charName");
invoke("get_character_name").then((n) => { if (n && document.activeElement !== charName) charName.value = n; }).catch(() => {});
const saveName = () => invoke("set_character_name", { name: charName.value }).catch(() => {});
document.getElementById("charSave").addEventListener("click", saveName);
charName.addEventListener("keydown", (e) => { if (e.key === "Enter") saveName(); });

// The overlay owns this setting; the main window only applies the saved value.
invoke("set_every_fight", { on: store.get("everyFight") !== "0" }).catch(() => {});

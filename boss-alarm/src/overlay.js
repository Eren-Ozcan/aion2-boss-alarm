"use strict";

const { invoke } = window.__TAURI__.core;

const lang = (navigator.language || "en").toLowerCase().startsWith("tr") ? "tr" : "en";
const TEXT = {
  en: {
    opacity: "Window opacity", rows: "Rows shown", fit: "Fit height to rows", scale: "Text size", every: "Show bosses I am not fighting", idle: "Waiting for a fight", ttk: "Est.",
    modes: { bossTargets: ["BOSS", "BOSS TARGETS", "Waiting for a boss"], lastHitByMe: ["LAST", "LAST HIT", "Waiting for a hit"], allTargets: ["ALL", "ALL TARGETS", "Waiting for a fight"], trainTargets: ["TRAIN", "TRAIN TARGETS", "Waiting for a dummy"] },
    metrics: { both: "BOTH", dps: "DPS", dmg: "DMG" },
    bosses: "Boss timers", noBosses: "No timers yet. Open the in-game map to sync.",
    spawned: "Spawned", unknown: "Unknown", never: "Not seen",
    spawnedAgo: (a) => `${a} ago (now unknown)`, stale: (a) => `seen ${a} ago`,
    lastSync: (a) => `Last sync ${a} ago`, noSync: "Not synced",
    ago: (s) => (s < 60 ? `${s}s` : s < 3600 ? `${Math.floor(s / 60)} min` : `${Math.floor(s / 3600)} h ${Math.floor((s % 3600) / 60)} min`),
  },
  tr: {
    opacity: "Pencere şeffaflığı", rows: "Gösterilen satır", fit: "Yüksekliği satırlara uydur", scale: "Yazı boyutu", every: "Vurmadığım bossları da göster", idle: "Savaş bekleniyor", ttk: "Tah.",
    modes: { bossTargets: ["BOSS", "BOSS HEDEFİ", "Boss bekleniyor"], lastHitByMe: ["SON", "SON VURUŞ", "Vuruş bekleniyor"], allTargets: ["HEPSİ", "TÜM HEDEFLER", "Savaş bekleniyor"], trainTargets: ["KUKLA", "KUKLA HEDEFİ", "Kukla bekleniyor"] },
    metrics: { both: "İKİSİ", dps: "DPS", dmg: "HASAR" },
    bosses: "Boss süreleri", noBosses: "Henüz süre yok. Oyunda haritayı açıp senkronla.",
    spawned: "Doğdu", unknown: "Bilinmiyor", never: "Görülmedi",
    spawnedAgo: (a) => `${a} önce (şu an bilinmiyor)`, stale: (a) => `${a} önce görüldü`,
    lastSync: (a) => `Son senkron ${a} önce`, noSync: "Senkron yok",
    ago: (s) => (s < 60 ? `${s} sn` : s < 3600 ? `${Math.floor(s / 60)} dk` : `${Math.floor(s / 3600)} sa ${Math.floor((s % 3600) / 60)} dk`),
  },
}[lang];
document.getElementById("lblOpacity").textContent = TEXT.opacity;
document.getElementById("lblRows").textContent = TEXT.rows;
document.getElementById("lblFit").textContent = TEXT.fit;
document.getElementById("lblScale").textContent = TEXT.scale;
document.getElementById("lblEvery").textContent = TEXT.every;

const JOBS = {
  "검성": "gladiator", "수호성": "templar", "살성": "assassin", "궁성": "ranger",
  "마도성": "sorcerer", "정령성": "elementalist", "치유성": "cleric", "호법성": "chanter", "권성": "fighter",
};

// ---- settings (shared with the main window through localStorage) ----
const store = {
  get(k) { try { return localStorage.getItem(k); } catch { return null; } },
  set(k, v) { try { localStorage.setItem(k, v); } catch { /* unavailable */ } },
};
const opacity = document.getElementById("opacity");
const rowCount = document.getElementById("rowCount");
// The overlay always opens see-through; the settings panel changes that.
opacity.value = store.get("overlayOpacity") ?? "40";
rowCount.value = store.get("overlayRows") ?? "6";
const textScale = document.getElementById("textScale");
textScale.value = store.get("overlayScale") ?? "100";
textScale.addEventListener("input", () => { store.set("overlayScale", textScale.value); applyLook(); });
const applyLook = () => {
  document.documentElement.style.setProperty("--scale", String(Number(textScale.value) / 100));
  document.documentElement.style.setProperty("--window-opacity", String(Number(opacity.value) / 100));
};
opacity.addEventListener("input", () => { store.set("overlayOpacity", opacity.value); applyLook(); });
rowCount.addEventListener("input", () => store.set("overlayRows", rowCount.value));
applyLook();

const settings = document.getElementById("settings");
const settingsBtn = document.getElementById("settingsBtn");
settingsBtn.addEventListener("click", () => {
  settings.hidden = !settings.hidden;
  settingsBtn.setAttribute("aria-expanded", String(!settings.hidden));
});
document.getElementById("closeBtn").addEventListener("click", () => window.__TAURI__.window.getCurrentWindow().close());
document.getElementById("resetBtn").addEventListener("click", () => { invoke("reset_dps").catch(() => {}); });
document.getElementById("windowBtn").addEventListener("click", () => { invoke("show_settings").catch(() => {}); });

// Footer controls, as in A2Tools DPS Meter: which target the meter follows,
// which figures the rows show, and the sort direction. All remembered.
const MODES = ["bossTargets", "lastHitByMe", "allTargets", "trainTargets"];
const METRICS = ["both", "dps", "dmg"];
let mode = MODES.includes(store.get("targetMode")) ? store.get("targetMode") : "bossTargets";
let metric = METRICS.includes(store.get("overlayMetric")) ? store.get("overlayMetric") : "both";
let ascending = store.get("overlaySortAsc") === "1";
const modeBtn = document.getElementById("modeBtn");
const metricBtn = document.getElementById("metricBtn");
function paintControls() {
  modeBtn.textContent = TEXT.modes[mode][0];
  modeBtn.title = TEXT.modes[mode][1];
  metricBtn.textContent = TEXT.metrics[metric];
  document.getElementById("sortIcon").setAttribute("d", ascending ? "M12 19V5M6 11l6-6 6 6" : "M12 5v14M6 13l6 6 6-6");
}
function applyMode() { invoke("set_target_mode", { mode }).catch(() => {}); }
modeBtn.addEventListener("click", () => { mode = MODES[(MODES.indexOf(mode) + 1) % MODES.length]; store.set("targetMode", mode); applyMode(); paintControls(); tick(); });
metricBtn.addEventListener("click", () => { metric = METRICS[(METRICS.indexOf(metric) + 1) % METRICS.length]; store.set("overlayMetric", metric); paintControls(); tick(); });
document.getElementById("sortBtn").addEventListener("click", () => { ascending = !ascending; store.set("overlaySortAsc", ascending ? "1" : "0"); paintControls(); tick(); });
document.getElementById("resizeHandle").addEventListener("mousedown", (e) => {
  e.preventDefault();
  // Dragging a size by hand means the player wants that size.
  autoFit.checked = false;
  store.set("overlayAutoFit", "0");
  window.__TAURI__.window.getCurrentWindow().startResizeDragging("SouthEast").catch(() => {});
});
paintControls();
applyMode();

// Show a boss fight whenever a boss is in range, not only when you or your
// party hit it. On by default; off keeps strangers' fights off the meter.
const everyFight = document.getElementById("everyFight");
everyFight.checked = store.get("everyFight") !== "0";
const applyEvery = () => { store.set("everyFight", everyFight.checked ? "1" : "0"); invoke("set_every_fight", { on: everyFight.checked }).catch(() => {}); };
everyFight.addEventListener("change", applyEvery);
applyEvery();

// Two views: the fight (DPS) and the boss timers. The choice is remembered.
let view = store.get("overlayView") === "bosses" ? "bosses" : "dps";
document.getElementById("viewBtn").addEventListener("click", () => {
  view = view === "dps" ? "bosses" : "dps";
  store.set("overlayView", view);
  tick();
});

// Remember where the window was left, so it opens in the same place next time.
let lastGeom = "";
setInterval(() => {
  const g = [window.screenX, window.screenY, window.outerWidth, window.outerHeight].map(Math.round);
  const key = g.join(",");
  if (key === lastGeom || g[2] < 100 || g[3] < 40) return;
  lastGeom = key;
  invoke("save_overlay_geometry", { x: g[0], y: g[1], w: g[2], h: g[3] }).catch(() => {});
}, 1500);

// ---- formatting ----
const num = (n) => {
  if (n >= 1e6) return `${(n / 1e6).toFixed(2)}M`;
  if (n >= 1e3) return `${(n / 1e3).toFixed(2)}k`;
  return String(Math.round(n));
};
const rate = (n) => (n >= 1e3 ? `${(n / 1e3).toFixed(0)}k/s` : `${Math.round(n)}/s`);
const mmss = (ms) => {
  const s = Math.max(0, Math.round(ms / 1000));
  return `${Math.floor(s / 60)}:${String(s % 60).padStart(2, "0")}`;
};
const displayName = (p) => (/^\d+$/.test(p.nickname) ? `#${p.nickname}` : p.nickname || "?");

// A target whose name the game never told us shows as its id, like A2Tools'.
const targetLabel = (d) => d.targetName || (d.targetId > 0 ? `MOB #${d.targetId}` : "?");

function el(tag, cls, text) {
  const e = document.createElement(tag);
  if (cls) e.className = cls;
  if (text !== undefined) e.textContent = text;
  return e;
}

// Time-to-kill, after A2Tools DPS Meter's estimator (GPL-3.0).
function createTtk() {
  const windowMs = 20000, riseTau = 30000, fallTau = 10000, minElapsed = 5000, maxTotal = 99 * 60000;
  let id = null, samples = [], rate_ = null, lastT = null;
  const reset = (n = null) => { id = n; samples = []; rate_ = null; lastT = null; };
  return (tid, t, hp, max) => {
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
    if (rate_ === null) {
      const seed = avg > 0 ? avg : observed;
      if (seed > 0) rate_ = seed;
    } else if (observed >= 0 && lastT !== null && t > lastT) {
      rate_ += (1 - Math.exp(-(t - lastT) / (observed > rate_ ? riseTau : fallTau))) * (observed - rate_);
    }
    lastT = t;
    if (!(rate_ > 0)) return null;
    const total = t + hp / rate_;
    return total <= maxTotal ? total : null;
  };
}
const ttk = createTtk();

let lastDamage = -1, lastChange = 0;

async function tickDps() {
  let d;
  try { d = await invoke("get_dps"); } catch { return; }
  const entries = Object.entries(d.map ?? {})
    .map(([id, p]) => ({ id: Number(id), p }))
    .sort((a, b) => b.p.amount - a.p.amount);
  const total = entries.reduce((s, e) => s + e.p.amount, 0);
  if (total !== lastDamage) { lastDamage = total; lastChange = Date.now(); }
  const fighting = entries.length > 0 && Date.now() - lastChange < 4000;

  const bossName = document.getElementById("bossName");
  bossName.textContent = entries.length ? targetLabel(d) : TEXT.modes[mode][1];
  bossName.classList.toggle("waiting", !entries.length);

  // boss HP band
  const hp = document.getElementById("hp");
  let est = null;
  if (entries.length && d.targetIsBoss && d.targetMaxHp > 0) {
    const left = Math.max(0, d.targetCurrentHp >= 0 ? d.targetCurrentHp : d.targetMaxHp - d.targetTotalDamage);
    const pct = (left / d.targetMaxHp) * 100;
    hp.hidden = false;
    const fill = document.getElementById("hpFill");
    fill.style.width = `${Math.min(100, pct)}%`;
    fill.className = `hp-fill${pct <= 25 ? " low" : pct <= 55 ? " mid" : ""}`;
    document.getElementById("hpPct").textContent = `${pct.toFixed(1)}%`;
    document.getElementById("hpAmt").textContent = num(left);
    est = ttk(d.targetId, d.battleTime, left, d.targetMaxHp);
  } else {
    hp.hidden = true;
  }

  // rows: the top of the list, and always you. With more players than rows,
  // the last row is yours at your true rank, so you never fall off the meter.
  const max = Math.max(2, Math.min(8, Number(store.get("overlayRows") ?? 6)));
  const rankOf = new Map(entries.map((e, i) => [e.id, i + 1]));
  let shown = entries.slice(0, max);
  const me = entries.find((e) => e.id === d.localPlayerId);
  if (me && !shown.includes(me)) shown = [...shown.slice(0, max - 1), me];
  if (ascending) shown = [...shown].reverse();
  const top = entries[0]?.p.amount || 1;
  document.getElementById("list").replaceChildren(...shown.map(({ id, p }) => {
    const rank = rankOf.get(id);
    const isMe = id === d.localPlayerId;
    // Your bar is always the same blue; the others take a colour by rank.
    const item = el("div", `item${isMe ? " user" : ""}${rank <= 5 ? ` rank${rank}` : ""}`);
    const fill = el("div", "fill");
    fill.style.width = `${Math.max(1, (p.amount / top) * 100)}%`;
    item.append(fill, el("span", "rank", String(rank)));
    const icon = JOBS[p.job];
    if (icon) {
      const img = document.createElement("img");
      img.className = "icon";
      img.src = `assets/classes/${icon}.png`;
      img.alt = "";
      item.append(img);
    }
    const fig = el("div", "figures", metric === "dps" ? rate(p.dps) : num(p.amount));
    if (metric === "both") fig.append(el("small", "", `(${rate(p.dps)})`));
    item.append(el("span", "name", displayName(p)), fig, el("span", "pct", `${p.damageContribution.toFixed(1)}%`));
    return item;
  }));

  // footer
  document.getElementById("dot").className = `dot${fighting ? " fighting" : ""}`;
  document.getElementById("time").textContent = mmss(entries.length ? d.battleTime : 0);
  document.getElementById("status").textContent = entries.length ? "" : TEXT.modes[mode][2];
  document.getElementById("ttk").textContent = est ? `${TEXT.ttk} ${mmss(est)}` : "";
}

// ---- boss timers ----
const clockText = (ms) => {
  const s = Math.max(0, Math.round(ms / 1000));
  const p = (n) => String(n).padStart(2, "0");
  const h = Math.floor(s / 3600);
  return h >= 24 ? `${Math.floor(h / 24)}d ${p(h % 24)}:${p(Math.floor((s % 3600) / 60))}` : `${p(h)}:${p(Math.floor((s % 3600) / 60))}:${p(s % 60)}`;
};

// Text and urgency of one boss, from what was last seen. Whatever happened
// while the app or the game was closed is shown as unknown, not guessed.
function bossLine(b, now) {
  const age = b.seen_local_ms ? Math.round((now - b.seen_local_ms) / 1000) : 0;
  if (b.spawned) return age > 60 ? [TEXT.stale(TEXT.ago(age)), "stale", Infinity] : [TEXT.spawned, "up", -1];
  if (b.respawn_local_ms == null) return [b.seen_local_ms ? TEXT.unknown : TEXT.never, "stale", Infinity];
  const left = b.respawn_local_ms - now;
  if (left <= 0) return [TEXT.spawnedAgo(TEXT.ago(Math.round(-left / 1000))), "stale", Infinity];
  return [clockText(left), left < 5 * 60_000 ? "soon" : "", left];
}

async function tickBosses() {
  let snap;
  try { snap = await invoke("get_snapshot"); } catch { return; }
  const now = Date.now();
  const all = snap.bosses.map((b) => ({ b, line: bossLine(b, now) }));
  // The bosses you set alarms for; with none set, everything with a known timer.
  let rows = all.filter((r) => r.b.alarm.enabled);
  if (!rows.length) rows = all.filter((r) => r.line[2] !== Infinity || r.line[1] === "up");
  rows.sort((x, y) => x.line[2] - y.line[2]);
  const max = Math.max(1, Math.min(10, Number(store.get("overlayRows") ?? 6) + 2));
  document.getElementById("bossName").textContent = TEXT.bosses;
  document.getElementById("hp").hidden = true;
  document.getElementById("empty").textContent = rows.length ? "" : TEXT.noBosses;
  document.getElementById("list").replaceChildren(...rows.slice(0, max).map(({ b, line }) => {
    const item = el("div", `item boss ${line[1]}`);
    item.append(el("span", "name", b.name), el("span", "figures", line[0]));
    return item;
  }));
  document.getElementById("dot").className = "dot";
  document.getElementById("time").textContent = "";
  document.getElementById("status").textContent = snap.last_list_local_ms
    ? TEXT.lastSync(TEXT.ago(Math.round((now - snap.last_list_local_ms) / 1000)))
    : TEXT.noSync;
  document.getElementById("ttk").textContent = "";
  document.getElementById("bossName").classList.remove("waiting");
}

// ---- auto height: the window is as tall as its rows, like A2Tools' ----
const autoFit = document.getElementById("autoFit");
autoFit.checked = store.get("overlayAutoFit") !== "0";
autoFit.addEventListener("change", () => { store.set("overlayAutoFit", autoFit.checked ? "1" : "0"); fitHeight(); });
let fitted = 0;
function fitHeight() {
  if (!autoFit.checked) return;
  const meter = document.getElementById("meter");
  let need = 2; // the panel's border
  for (const child of meter.children) {
    if (child.hidden || child.id === "resizeHandle" || getComputedStyle(child).display === "none") continue;
    if (child.id === "list") {
      const items = child.children.length;
      if (items) {
        // Measured, not assumed: row height follows the text size.
        const cs = getComputedStyle(child);
        const rowH = child.firstElementChild.offsetHeight;
        need += items * rowH + (items - 1) * (parseFloat(cs.rowGap) || 0) + parseFloat(cs.paddingTop) + parseFloat(cs.paddingBottom);
      }
    } else {
      need += child.offsetHeight;
    }
  }
  need = Math.max(70, Math.min(700, Math.ceil(need)));
  if (Math.abs(need - fitted) < 2 || Math.abs(need - window.innerHeight) < 2) { fitted = need; return; }
  fitted = need;
  const { LogicalSize } = window.__TAURI__.dpi ?? {};
  if (LogicalSize) window.__TAURI__.window.getCurrentWindow().setSize(new LogicalSize(window.innerWidth, need)).catch(() => {});
}

// ---- ping, from the game's own ping replies ----
let pingTick = 0;
async function refreshPing() {
  if (pingTick++ % 8) return;
  const el_ = document.getElementById("ping");
  let ms = null;
  try { ms = await invoke("get_ping"); } catch { /* none */ }
  el_.textContent = ms == null ? "" : `${ms}ms`;
  el_.className = `ping${ms > 150 ? " high" : ms > 80 ? " mid" : ""}`;
}

async function tick() {
  document.getElementById("empty").textContent = "";
  await (view === "bosses" ? tickBosses() : tickDps());
  refreshPing();
  fitHeight();
}

tick();
setInterval(tick, 250);

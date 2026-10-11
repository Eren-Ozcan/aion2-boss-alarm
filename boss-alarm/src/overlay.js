"use strict";

const { invoke } = window.__TAURI__.core;

const chosenLang = (() => { try { return localStorage.getItem("lang"); } catch { return null; } })();
const lang = chosenLang === "tr" || chosenLang === "en" ? chosenLang : (navigator.language || "en").toLowerCase().startsWith("tr") ? "tr" : "en";
const TEXT = {
  en: {
    opacity: "Opacity", rows: "Rows", scale: "Text size", reset: "Reset every fight", fit: "Auto-fit window", partial: "Show players with partial data (DOT only)",
    partialNote: "Partial data (DOT only)",
    pin: "Always show me at the top", ping: "Show ping", ttk_: "Show time to kill (TTK)", total_: "Show total DPS / DMG", autoHide: "Auto-hide when idle", resetPos: "Reset position", locked: "Locked", lockTip: "Lock overlay (clicks pass through)", unlockTip: "Overlay locked: Ctrl+Alt+K unlocks", muteTip: "Mute alarms", unmuteTip: "Alarms muted: click to unmute",
    ttk: "TTK",
    modes: { bossTargets: ["BOSS", "BOSS TARGETS", "Waiting for a boss"], lastHitByMe: ["LAST", "LAST HIT", "Waiting for a hit"], allTargets: ["ALL", "ALL TARGETS", "Waiting for a fight"], trainTargets: ["TRAIN", "TRAIN TARGETS", "Waiting for a dummy"] },
    metrics: { both: "BOTH", dps: "DPS", dmg: "DMG" },
    timers: "Boss timers", synced: (a) => `Synced ${a} ago`, notSynced: "Not synced · open the map", live: "Up now",
    born: "Spawned", field: "Field", abyss: "Abyss", event: "Event", started: "Started", unknown: "?",
    upTip: "Up", maybeTip: "Respawn time passed, not confirmed",
    level: "Lv.", gear: "Gear", power: "Combat power", server: "Server", damage: "Damage", dps: "DPS", share: "Share", rank: "Rank", close: "Close",
    ago: (s) => (s < 60 ? `${s}s` : s < 3600 ? `${Math.floor(s / 60)} min` : `${Math.floor(s / 3600)} h ${Math.floor((s % 3600) / 60)} min`),
  },
  tr: {
    opacity: "Şeffaflık", rows: "Satır", scale: "Yazı boyutu", reset: "Her savaşta sıfırla", fit: "Pencereyi sığdır", partial: "Eksik verili oyuncuları göster (sadece DOT)",
    partialNote: "Eksik veri (sadece DOT)",
    pin: "Beni hep en üstte göster", ping: "Ping göster", ttk_: "Bitiş süresini (TTK) göster", total_: "Toplam DPS / hasarı göster", autoHide: "Boştayken otomatik gizle", resetPos: "Konumu sıfırla", locked: "Kilitli", lockTip: "Overlay'i kilitle (tıklamalar geçer)", unlockTip: "Overlay kilitli: Ctrl+Alt+K açar", muteTip: "Alarmları sustur", unmuteTip: "Alarmlar sessiz: açmak için tıkla",
    ttk: "TTK",
    modes: { bossTargets: ["BOSS", "BOSS HEDEFİ", "Boss bekleniyor"], lastHitByMe: ["SON", "SON VURUŞ", "Vuruş bekleniyor"], allTargets: ["HEPSİ", "TÜM HEDEFLER", "Savaş bekleniyor"], trainTargets: ["KUKLA", "KUKLA HEDEFİ", "Kukla bekleniyor"] },
    metrics: { both: "İKİSİ", dps: "DPS", dmg: "HASAR" },
    timers: "Boss süreleri", synced: (a) => `${a} önce senkron`, notSynced: "Senkron yok · haritayı aç", live: "Canlı",
    born: "Doğdu", field: "Saha", abyss: "Abyss", event: "Etkinlik", started: "Başladı", unknown: "?",
    upTip: "Canlı", maybeTip: "Doğma zamanı geçti, doğrulanmadı",
    level: "Sv.", gear: "Ekipman", power: "Savaş gücü", server: "Sunucu", damage: "Hasar", dps: "DPS", share: "Pay", rank: "Sıra", close: "Kapat",
    ago: (s) => (s < 60 ? `${s} sn` : s < 3600 ? `${Math.floor(s / 60)} dk` : `${Math.floor(s / 3600)} sa ${Math.floor((s % 3600) / 60)} dk`),
  },
}[lang];
const $ = (id) => document.getElementById(id);
$("lblOpacity").textContent = TEXT.opacity;
$("lblRows").textContent = TEXT.rows;
$("lblScale").textContent = TEXT.scale;
$("lblReset").textContent = TEXT.reset;
$("lblFit").textContent = TEXT.fit;
$("lblPartial").textContent = TEXT.partial;
$("lblPin").textContent = TEXT.pin;
$("lblPing").textContent = TEXT.ping;
$("lblTtk").textContent = TEXT.ttk_;
$("lblTotal").textContent = TEXT.total_;
$("lblAutoHide").textContent = TEXT.autoHide;
$("resetPos").textContent = TEXT.resetPos;
$("lockBadge").textContent = TEXT.locked;
$("tTitle").textContent = TEXT.timers;

const JOBS = {
  "검성": "gladiator", "수호성": "templar", "살성": "assassin", "궁성": "ranger",
  "마도성": "sorcerer", "정령성": "elementalist", "치유성": "cleric", "호법성": "chanter", "권성": "fighter",
};
const CLASS_LABEL = { gladiator: "Gladiator", templar: "Templar", assassin: "Assassin", ranger: "Ranger", sorcerer: "Sorcerer", elementalist: "Elementalist", cleric: "Cleric", chanter: "Chanter", fighter: "Fighter" };

// ---- settings (shared with the main window through localStorage) ----
const store = {
  get(k) { try { return localStorage.getItem(k); } catch { return null; } },
  set(k, v) { try { localStorage.setItem(k, v); } catch { /* unavailable */ } },
};
const opacity = $("opacity");
const rowCount = $("rowCount");
opacity.value = store.get("overlayOpacity") ?? "72";
rowCount.value = store.get("overlayRows") ?? "6";
const textScale = $("textScale");
textScale.value = store.get("overlayScale") ?? "100";
textScale.addEventListener("input", () => { store.set("overlayScale", textScale.value); applyLook(); });
const applyLook = () => {
  document.documentElement.style.setProperty("--scale", String(Number(textScale.value) / 100));
  // The settings panel keeps its size, so the slider does not move away under the cursor while dragging.
  document.documentElement.style.setProperty("--unscale", String(100 / Number(textScale.value)));
  document.documentElement.style.setProperty("--window-opacity", String(Number(opacity.value) / 100));
};
opacity.addEventListener("input", () => { store.set("overlayOpacity", opacity.value); applyLook(); });
rowCount.addEventListener("input", () => store.set("overlayRows", rowCount.value));
applyLook();

const settings = $("settings");
const settingsBtn = $("settingsBtn");
settingsBtn.addEventListener("click", () => {
  settings.hidden = !settings.hidden;
  settingsBtn.setAttribute("aria-expanded", String(!settings.hidden));
});
$("closeBtn").addEventListener("click", () => window.__TAURI__.window.getCurrentWindow().close());
$("resetBtn").addEventListener("click", () => { invoke("reset_dps").catch(() => {}); });
$("windowBtn").addEventListener("click", () => { invoke("show_settings").catch(() => {}); });

// Lock: clicks pass through the overlay to the game. The hotkey (Ctrl+Alt+K) and the tray unlock it again.
const lockBtn = $("lockBtn");
let locked = false;
function paintLock() {
  lockBtn.setAttribute("aria-pressed", String(locked));
  lockBtn.classList.toggle("on", locked);
  lockBtn.title = locked ? TEXT.unlockTip : TEXT.lockTip;
  $("lockBadge").hidden = !locked;
}
lockBtn.addEventListener("click", () => { invoke("toggle_overlay_lock").then((v) => { locked = v; paintLock(); }).catch(() => {}); });
invoke("get_overlay_locked").then((v) => { locked = v; paintLock(); }).catch(() => {});
window.__TAURI__.event.listen("overlay-locked", (e) => { locked = !!e.payload; paintLock(); });
paintLock();

// Mute: no alarm rings; the state lives in the app, so the tray and the window share it.
const muteBtn = $("muteBtn");
let muted = false;
function paintMute() {
  muteBtn.setAttribute("aria-pressed", String(muted));
  muteBtn.classList.toggle("on", muted);
  muteBtn.title = muted ? TEXT.unmuteTip : TEXT.muteTip;
  $("muteSlash").style.display = muted ? "" : "none";
}
muteBtn.addEventListener("click", () => { invoke("set_muted", { muted: !muted }).catch(() => {}); muted = !muted; paintMute(); });
paintMute();

// Footer controls: which target the meter follows, which figures the rows show, and the sort direction. All remembered.
const MODES = ["bossTargets", "lastHitByMe", "allTargets", "trainTargets"];
const METRICS = ["both", "dps", "dmg"];
// The default is BOSS. A boss whose spawn the app never saw (it started late) still counts once
// a crowd hits it, so BOSS shows the boss fight and ALL, which also lists every mob fought nearby, is not needed.
// A one-time switch moves the earlier default of ALL back to BOSS.
if (store.get("modeV3") !== "1") { store.set("targetMode", "bossTargets"); store.set("modeV3", "1"); }
let mode = MODES.includes(store.get("targetMode")) ? store.get("targetMode") : "bossTargets";
let metric = METRICS.includes(store.get("overlayMetric")) ? store.get("overlayMetric") : "both";
let ascending = store.get("overlaySortAsc") === "1";
const modeBtn = $("modeBtn");
const metricBtn = $("metricBtn");
function paintControls() {
  modeBtn.textContent = TEXT.modes[mode][0];
  modeBtn.title = TEXT.modes[mode][1];
  metricBtn.textContent = TEXT.metrics[metric];
  $("sortIcon").setAttribute("d", ascending ? "M12 19V5M6 11l6-6 6 6" : "M12 5v14M6 13l6 6 6-6");
}
function applyMode() { invoke("set_target_mode", { mode }).catch(() => {}); }
modeBtn.addEventListener("click", () => { mode = MODES[(MODES.indexOf(mode) + 1) % MODES.length]; store.set("targetMode", mode); applyMode(); paintControls(); tick(); });
metricBtn.addEventListener("click", () => { metric = METRICS[(METRICS.indexOf(metric) + 1) % METRICS.length]; store.set("overlayMetric", metric); paintControls(); tick(); });
$("sortBtn").addEventListener("click", () => { ascending = !ascending; store.set("overlaySortAsc", ascending ? "1" : "0"); paintControls(); tick(); });
$("resizeHandle").addEventListener("mousedown", (e) => {
  e.preventDefault();
  // Dragging a size by hand means the player wants that size.
  autoFit.checked = false;
  store.set("overlayAutoFit", "0");
  window.__TAURI__.window.getCurrentWindow().startResizeDragging("SouthEast").catch(() => {});
});
paintControls();
applyMode();

// A boss fight is shown whenever a boss is in range, not only when you or your party hit it.
invoke("set_every_fight", { on: store.get("everyFight") !== "0" }).catch(() => {});

// Reset every fight: the meter starts clean when a new fight begins after a pause.
const resetEvery = $("resetEvery");
resetEvery.checked = store.get("resetEvery") === "1";
resetEvery.addEventListener("change", () => store.set("resetEvery", resetEvery.checked ? "1" : "0"));
const FIGHT_GAP_MS = 10_000;

// Display options, remembered. Ping and TTK start on, the rest off.
const OPTIONS = { pinMe: "0", showPing: "1", showTtk: "1", showTotal: "0", autoHide: "0" };
const opt = {};
for (const [id, def] of Object.entries(OPTIONS)) {
  const box = $(id);
  box.checked = (store.get(id) ?? def) === "1";
  opt[id] = () => box.checked;
  box.addEventListener("change", () => { store.set(id, box.checked ? "1" : "0"); if (id === "autoHide" && !box.checked) setAutoHidden(false); });
}
$("resetPos").addEventListener("click", () => { invoke("reset_overlay_position").catch(() => {}); });

// Auto-hide: the window leaves when nothing is going on and returns with a fight or an alert.
let autoHidden = false;
let idleSince = Date.now();
function setAutoHidden(hidden) {
  if (hidden === autoHidden) return;
  autoHidden = hidden;
  invoke("set_overlay_visible", { visible: !hidden }).catch(() => {});
}

// The server sends you your own hits, but from other players mostly damage-over-time ticks.
// Everyone is shown by default, like A2Tools; switching this off leaves out the players known only by DOT ticks.
const showPartial = $("showPartial");
showPartial.checked = (store.get("showPartial") ?? "1") === "1";
showPartial.addEventListener("change", () => store.set("showPartial", showPartial.checked ? "1" : "0"));

// The boss timers can be folded away; the ones about to spawn stay visible either way.
let timersOpen = store.get("overlayTimers") !== "0";
const timersBtn = $("timersBtn");
timersBtn.title = TEXT.timers;
timersBtn.addEventListener("click", () => {
  timersOpen = !timersOpen;
  store.set("overlayTimers", timersOpen ? "1" : "0");
  timersBtn.setAttribute("aria-pressed", String(timersOpen));
  paintTimers();
});
timersBtn.setAttribute("aria-pressed", String(timersOpen));

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
  if (n >= 1e3) return `${(n / 1e3).toFixed(1)}K`;
  return String(Math.round(n));
};
function cpBadge(v) {
  const c = el("span", "cp");
  const img = document.createElement("img");
  img.src = "assets/cp.png";
  img.alt = "";
  c.append(img, document.createTextNode(v.toLocaleString()));
  c.title = TEXT.power;
  return c;
}
const rate = (n) => (n >= 1e3 ? `${(n / 1e3).toFixed(1)}K` : `${Math.round(n)}`);
const mmss = (ms) => {
  const s = Math.max(0, Math.round(ms / 1000));
  return `${String(Math.floor(s / 60)).padStart(2, "0")}:${String(s % 60).padStart(2, "0")}`;
};
const displayName = (p) => (/^\d+$/.test(p.nickname) ? `#${p.nickname}` : p.nickname || "?");
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

// ---- player card: what the game told us about the player you clicked ----
let lastPlayers = {};
let lastLocalId = null;
let lastRanks = new Map();
let cardId = null;
const cardEl = $("card");
$("list").addEventListener("click", (e) => {
  const row = e.target.closest(".item[data-id]");
  if (!row) return;
  cardId = cardId === row.dataset.id ? null : row.dataset.id;
  paintCard();
});
function paintCard() {
  const p = cardId == null ? null : lastPlayers[cardId];
  if (!p) { cardEl.hidden = true; cardId = null; return; }
  const stat = (label, value) => { const s = el("div", "c-stat"); s.append(el("b", "", String(value)), el("span", "", label)); return s; };
  const head = el("div", "c-head");
  const icon = JOBS[p.job];
  if (icon) { const img = document.createElement("img"); img.src = `assets/classes/${icon}.png`; img.alt = ""; head.append(img); }
  const who = el("div", "c-who");
  // Class, level and server, only as far as the game sent them.
  const sub = [CLASS_LABEL[icon] ?? "", p.level > 0 ? `${TEXT.level} ${p.level}` : "", p.serverId > 0 ? `${TEXT.server} ${p.serverId}` : ""].filter(Boolean).join(" · ");
  who.append(el("div", `c-name${Number(cardId) === lastLocalId ? " user" : ""}`, displayName(p)), el("div", "c-sub", sub));
  const x = el("button", "c-x");
  x.type = "button"; x.title = TEXT.close; x.setAttribute("aria-label", TEXT.close);
  x.innerHTML = '<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M6 6l12 12M18 6L6 18"/></svg>';
  x.addEventListener("click", () => { cardId = null; paintCard(); });
  head.append(who, x);
  const grid = el("div", "c-grid");
  grid.append(stat(TEXT.damage, num(p.amount)), stat(TEXT.dps, rate(p.dps)), stat(TEXT.share, `${p.damageContribution.toFixed(1)}%`));
  if (p.gearScore > 0) grid.append(stat(TEXT.gear, p.gearScore.toLocaleString()));
  if (p.combatPower > 0) grid.append(stat(TEXT.power, p.combatPower.toLocaleString()));
  const rank = lastRanks.get(Number(cardId));
  if (rank) grid.append(stat(TEXT.rank, `#${rank}`));
  cardEl.replaceChildren(head, grid);
  cardEl.hidden = false;
}

async function tickDps() {
  let d;
  try { d = await invoke("get_dps"); } catch { return; }
  const all = Object.entries(d.map ?? {}).map(([id, p]) => ({ id: Number(id), p: { ...p } }));
  // Players known only by damage-over-time ticks are left out, unless nobody has a direct hit
  // (a boss you are not fighting): then they are all there is, and are shown as partial data.
  const anyDirect = all.some((e) => e.p.direct !== false);
  const entries = all
    .filter((e) => showPartial.checked || !anyDirect || e.p.direct !== false || e.id === d.localPlayerId)
    .sort((a, b) => b.p.amount - a.p.amount);
  // Shares among the players shown, so a lone you reads 100 %.
  const shownTotal = entries.reduce((s, e) => s + e.p.amount, 0) || 1;
  for (const e of entries) e.p.damageContribution = (e.p.amount / shownTotal) * 100;
  const total = entries.reduce((s, e) => s + e.p.amount, 0);
  if (total !== lastDamage) {
    // A new fight after a pause: start the meter clean (the hit that woke it is lost).
    if (resetEvery.checked && lastDamage > 0 && Date.now() - lastChange > FIGHT_GAP_MS) {
      lastDamage = -1; lastChange = Date.now();
      invoke("reset_dps").catch(() => {});
      return;
    }
    lastDamage = total; lastChange = Date.now();
  }
  const fighting = entries.length > 0 && Date.now() - lastChange < 4000;

  const bossName = $("bossName");
  bossName.textContent = entries.length ? targetLabel(d) : TEXT.modes[mode][1];
  bossName.classList.toggle("waiting", !entries.length);
  $("bossSub").textContent = entries.length ? TEXT.modes[mode][1] : "";

  // boss HP band
  const hp = $("hp");
  let est = null;
  if (entries.length && d.targetIsBoss && d.targetMaxHp > 0) {
    const left = Math.max(0, d.targetCurrentHp >= 0 ? d.targetCurrentHp : d.targetMaxHp - d.targetTotalDamage);
    const pct = (left / d.targetMaxHp) * 100;
    hp.hidden = false;
    $("hpFill").style.width = `${Math.min(100, pct)}%`;
    $("hpPct").textContent = `${pct.toFixed(1)}%`;
    $("hpAmt").textContent = `${num(left)} / ${num(d.targetMaxHp)}`;
    est = ttk(d.targetId, d.battleTime, left, d.targetMaxHp);
  } else {
    hp.hidden = true;
  }

  // rows: the top of the list, and always you. With more players than rows,
  // the last row is yours at your true rank, so you never fall off the meter.
  const max = Math.max(1, Math.min(12, Number(store.get("overlayRows") ?? 6)));
  const rankOf = new Map(entries.map((e, i) => [e.id, i + 1]));
  let shown = entries.slice(0, max);
  const me = entries.find((e) => e.id === d.localPlayerId);
  if (me && !shown.includes(me)) shown = [...shown.slice(0, max - 1), me];
  if (opt.pinMe() && me) shown = [me, ...shown.filter((e) => e !== me)];
  if (ascending) shown = [...shown].reverse();
  const top = entries[0]?.p.amount || 1;
  $("list").replaceChildren(...shown.map(({ id, p }) => {
    const rank = rankOf.get(id);
    const isMe = id === d.localPlayerId;
    // Bars follow the rank colours; yours is not marked by colour, your name is blue.
    const item = el("div", `item${isMe ? " user" : ""}${rank <= 8 ? ` rank${rank}` : ""}`);
    item.dataset.id = String(id);
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
    const fig = el("div", "figures", metric === "dmg" ? num(p.amount) : rate(p.dps));
    if (metric === "both") fig.append(el("small", "", num(p.amount)));
    item.append(el("span", "name", displayName(p)), ...(p.combatPower > 0 ? [cpBadge(p.combatPower)] : []), fig, el("span", "pct", `${p.damageContribution.toFixed(1)}%`));
    return item;
  }));

  lastPlayers = Object.fromEntries(entries.map((e) => [e.id, e.p]));
  lastLocalId = d.localPlayerId ?? null;
  lastRanks = rankOf;
  paintCard();

  // footer
  $("dot").className = `dot${fighting ? " fighting" : ""}`;
  $("time").textContent = mmss(entries.length ? d.battleTime : 0);
  $("status").textContent = entries.length ? (anyDirect ? "" : TEXT.partialNote) : TEXT.modes[mode][2];
  $("ttk").textContent = est && opt.showTtk() ? `${TEXT.ttk} ~${mmss(est)}` : "";
  const totalDps = entries.reduce((sum, e) => sum + e.p.dps, 0);
  $("total").textContent = opt.showTotal() && entries.length ? `${rate(totalDps)}/s · ${num(total)}` : "";
}

// ---- boss timers ----
const clockText = (ms) => {
  const s = Math.max(0, Math.floor(ms / 1000));
  const p = (n) => String(n).padStart(2, "0");
  const h = Math.floor(s / 3600);
  const m = Math.floor((s % 3600) / 60);
  return h >= 24 ? `${Math.floor(h / 24)}d ${p(h % 24)}h` : h ? `${h}:${p(m)}:${p(s % 60)}` : `${p(m)}:${p(s % 60)}`;
};
const zoneLabel = (b) => (b.group === "abyss" ? TEXT.abyss : TEXT.field);

// What the last list said about each boss, as a state at time `now`:
//   up      came up and the list says so (fresh), or came up within the last half minute
//   maybe   its timer ran out a while ago and the map has not confirmed it
//   soon    counting down to a respawn
//   none    nothing known
// Whatever happened while the app or the game was closed is shown as unknown, not guessed.
const BORN_MS = 30_000;
function bossState(b, now) {
  const age = b.seen_local_ms ? now - b.seen_local_ms : Infinity;
  if (b.spawned) {
    const since = b.spawned_at_local_ms != null ? now - b.spawned_at_local_ms : null;
    return { kind: age <= 60_000 ? "up" : "maybe", since, born: since != null && since <= BORN_MS };
  }
  if (b.respawn_local_ms == null) return { kind: "none" };
  const left = b.respawn_local_ms - now;
  if (left > 0) return { kind: "soon", left };
  return left >= -BORN_MS ? { kind: "up", since: -left, born: true } : { kind: "maybe", since: -left, born: false };
}

let bossSnap = [];
let eventSnap = [];
const eventStart = new Map(); // id -> the start that just passed, to show "started" for 30 s
let lastSyncMs = null;
let snapTick = 0;
async function refreshBosses() {
  if (snapTick++ % 4) return;
  try {
    const s = await invoke("get_snapshot");
    bossSnap = s.bosses;
    if (muted !== !!s.muted) { muted = !!s.muted; paintMute(); }
    for (const e of s.events ?? []) {
      const old = eventSnap.find((x) => x.id === e.id);
      if (old && e.next_local_ms > old.next_local_ms) eventStart.set(e.id, old.next_local_ms);
    }
    eventSnap = s.events ?? [];
    lastSyncMs = s.last_list_local_ms ?? null;
  } catch { /* keep the last */ }
}

function paintTimers() {
  const now = Date.now();
  const states = bossSnap.map((b) => ({ b, s: bossState(b, now) }));

  // Alerts: the alarm time has come (the largest minutes set for the boss, 15 by default),
  // and the boss that just spawned, for 30 s. They stay when the timers are folded away.
  const alerts = [];
  for (const { b, s } of states) {
    if (!b.alarm.enabled) continue;
    if (s.kind === "up" && s.born) alerts.push({ b, born: true, left: -1 });
    else if (s.kind === "soon") {
      const lead = Math.max(0, ...(b.alarm.minutes ?? [])) * 60_000;
      if (s.left <= lead) alerts.push({ b, born: false, left: s.left });
    }
  }
  // Scheduled events: alerts from their alarm time on, and "started" for 30 s after they begin.
  for (const e of eventSnap) {
    if (!e.alarm.enabled) continue;
    const left = e.next_local_ms - now;
    const began = eventStart.get(e.id);
    const b = { id: e.id, name: e.name, group: "event" };
    if (began != null && now - began >= 0 && now - began <= BORN_MS) alerts.push({ b, born: true, left: -1, event: true });
    else if (left <= Math.max(0, ...(e.alarm.minutes ?? [])) * 60_000) alerts.push({ b, born: false, left, event: true });
  }
  alerts.sort((x, y) => x.left - y.left);
  const alertIds = new Set(alerts.map((a) => a.b.id));

  const alertBox = $("alerts");
  alertBox.hidden = alerts.length === 0;
  if (alerts.length) {
    const rows = alerts.slice(0, 4).map((a) => {
      const r = el("div", `soon-row${a.event ? " event" : ""}${a.born ? " born" : a.left < 300_000 ? " imminent" : ""}`);
      r.append(el("span", "n", a.b.name), el("span", "z", a.event ? TEXT.event : zoneLabel(a.b)), el("span", "t", a.born ? (a.event ? TEXT.started : TEXT.born) : clockText(a.left)));
      return r;
    });
    if (alerts.length > 4) rows.push(el("div", "soon-row", `+${alerts.length - 4}`));
    alertBox.replaceChildren(...rows);
  }

  const timed = states.filter((x) => x.s.kind === "soon").sort((x, y) => x.s.left - y.s.left);
  const live = states.filter((x) => x.s.kind === "up" || (x.s.kind === "maybe" && (x.s.since == null || x.s.since < 30 * 60_000)));
  const haveAny = timed.length > 0 || live.length > 0;

  const box = $("timers");
  box.hidden = !(timersOpen && haveAny);
  const nextBox = $("nextList");
  nextBox.hidden = true;
  if (box.hidden) return;

  $("tSync").textContent = lastSyncMs ? `· ${TEXT.synced(TEXT.ago(Math.max(0, Math.round((now - lastSyncMs) / 1000))))}` : `· ${TEXT.notSynced}`;
  const next = timed[0];
  $("tNextName").textContent = next ? next.b.name : "";
  $("tNextTime").textContent = next ? clockText(next.s.left) : "";

  const liveRow = $("liveRow");
  liveRow.hidden = live.length === 0;
  if (live.length) {
    const chips = live.slice(0, 5).map(({ b, s }) => {
      const c = el("span", `chip${s.kind === "maybe" ? " maybe" : ""}`, b.name);
      c.title = s.kind === "maybe" ? TEXT.maybeTip : TEXT.upTip;
      if (s.since != null) c.append(el("small", "", TEXT.ago(Math.max(0, Math.round(s.since / 1000)))));
      return c;
    });
    if (live.length > 5) chips.push(el("span", "chip maybe", `+${live.length - 5}`));
    liveRow.replaceChildren(el("span", "cap", TEXT.live), ...chips);
  }

  // The next ones in line, besides those already shown as alerts.
  const rest = timed.filter((x) => !alertIds.has(x.b.id)).slice(0, 4);
  nextBox.hidden = rest.length === 0;
  if (rest.length) {
    nextBox.replaceChildren(...rest.map(({ b, s }) => {
      const r = el("div", "next-row");
      r.append(el("span", "n", b.name), el("span", "z", zoneLabel(b)), el("span", "t", clockText(s.left)));
      return r;
    }));
  }
}

// ---- auto height: the window is as tall as its rows, like A2Tools' ----
const autoFit = $("autoFit");
autoFit.checked = store.get("overlayAutoFit") !== "0";
autoFit.addEventListener("change", () => { store.set("overlayAutoFit", autoFit.checked ? "1" : "0"); fitHeight(); });
let fitted = 0;
function fitHeight() {
  if (!autoFit.checked) return;
  const meter = $("meter");
  let need = 2; // the panel's border
  for (const child of meter.children) {
    if (child.hidden || child.id === "resizeHandle" || child.id === "card" || getComputedStyle(child).display === "none") continue;
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
  // An open player card sits over the list; give it room.
  if (!cardEl.hidden) need = Math.max(need, cardEl.offsetHeight + 36 + 44);
  need = Math.max(70, Math.min(900, Math.ceil(need)));
  if (Math.abs(need - fitted) < 2 || Math.abs(need - window.innerHeight) < 2) { fitted = need; return; }
  fitted = need;
  const { LogicalSize } = window.__TAURI__.dpi ?? {};
  if (LogicalSize) window.__TAURI__.window.getCurrentWindow().setSize(new LogicalSize(window.innerWidth, need)).catch(() => {});
}

// ---- ping, from the game's own ping replies ----
let pingTick = 0;
async function refreshPing() {
  if (pingTick++ % 8) return;
  const el_ = $("ping");
  let ms = null;
  try { ms = await invoke("get_ping"); } catch { /* none */ }
  el_.textContent = ms == null || !opt.showPing() ? "" : `${ms} ms`;
  el_.className = `ping${ms > 150 ? " high" : ms > 80 ? " mid" : ""}`;
}

async function tick() {
  $("empty").textContent = "";
  await tickDps();
  await refreshBosses();
  paintTimers();
  // Auto-hide: nothing to show for 8 s hides the window; a fight or an alert brings it back.
  if (opt.autoHide()) {
    const busy = Object.keys(lastPlayers).length > 0 || !$("alerts").hidden;
    if (busy) { idleSince = Date.now(); setAutoHidden(false); }
    else if (Date.now() - idleSince > 8000) setAutoHidden(true);
  }
  refreshPing();
  fitHeight();
}

// A language change in the main window reloads this one too.
window.addEventListener("storage", (e) => { if (e.key === "lang") location.reload(); });

tick();
setInterval(tick, 250);

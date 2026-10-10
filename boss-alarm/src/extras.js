"use strict";
// First-run notice, Npcap guide, fight export and the log button.

const X = {
  en: {
    welcomeTitle: "Before you start",
    welcomeLead: "BellMeter is a fan-made helper. Please read this once.",
    points: [
      ["ok", "It only listens. The app reads your own game's network traffic through Npcap, and nothing else. It sends nothing to the game, and does not touch game files or memory."],
      ["ok", "Everything stays on your PC. Timers, fights and settings are stored locally. Nothing is uploaded."],
      ["warn", "It is not approved by NCSOFT. Whether tools like this are allowed under the game's rules is not known and can change. Using it could lead to warnings or account actions."],
      ["warn", "You use it at your own risk. It is provided as is, without any warranty, and its authors take no responsibility for what happens to your account."],
    ],
    consent: "I have read this and I use the app at my own risk.",
    go: "Start using the app",
    guideTitle: "Npcap is needed",
    guideLead: "BellMeter reads game traffic with Npcap, a free packet capture tool. Set it up once:",
    s1: "Open the Npcap site and download the installer.",
    s2: "Run it and tick “Install Npcap in WinPcap API-compatible Mode”, then finish the setup.",
    s3: "Come back here and press Retry.",
    openNpcap: "Open npcap.com",
    retry: "Retry",
    openLog: "Open log folder",
    logFail: "Could not open the log folder.",
    exportOk: (p) => `Saved: ${p}`,
    exportFail: "Could not save the export.",
    version: (v) => `Version ${v}`,
    checkUpdate: "Check for updates",
    upToDate: "You are up to date.",
    updateFound: (v) => `Version ${v} is available`,
    updateInstall: (v) => `Update to ${v}`,
    updating: "Downloading the update…",
    updateFail: "Update check failed.",
  },
  tr: {
    welcomeTitle: "Başlamadan önce",
    welcomeLead: "BellMeter hayran yapımı bir yardımcı araçtır. Lütfen bir kez oku.",
    points: [
      ["ok", "Sadece dinler. Uygulama, kendi oyununun ağ trafiğini Npcap ile okur, başka bir şey yapmaz. Oyuna hiçbir şey göndermez, oyun dosyalarına ve belleğine dokunmaz."],
      ["ok", "Her şey bilgisayarında kalır. Süreler, savaşlar ve ayarlar yerelde saklanır. Hiçbir şey yüklenmez."],
      ["warn", "NCSOFT tarafından onaylanmış değildir. Bu tür araçların oyun kurallarına uyup uymadığı bilinmiyor ve değişebilir. Kullanmak uyarıya veya hesap işlemine yol açabilir."],
      ["warn", "Kullanım riski sana aittir. Olduğu gibi, garantisiz sunulur; hesabına olanlar için geliştiricilerin sorumluluğu yoktur."],
    ],
    consent: "Okudum, uygulamayı kendi sorumluluğumda kullanıyorum.",
    go: "Uygulamayı kullanmaya başla",
    guideTitle: "Npcap gerekli",
    guideLead: "BellMeter oyun trafiğini Npcap ile okur, ücretsiz bir paket yakalama aracıdır. Bir kez kur:",
    s1: "Npcap sitesini aç ve kurulum dosyasını indir.",
    s2: "Çalıştır ve “Install Npcap in WinPcap API-compatible Mode” kutusunu işaretle, kurulumu bitir.",
    s3: "Buraya dön ve Yeniden dene'ye bas.",
    openNpcap: "npcap.com'u aç",
    retry: "Yeniden dene",
    openLog: "Kayıt klasörünü aç",
    logFail: "Kayıt klasörü açılamadı.",
    exportOk: (p) => `Kaydedildi: ${p}`,
    exportFail: "Dışa aktarma kaydedilemedi.",
    version: (v) => `Sürüm ${v}`,
    checkUpdate: "Güncellemeleri denetle",
    upToDate: "Uygulama güncel.",
    updateFound: (v) => `${v} sürümü hazır`,
    updateInstall: (v) => `${v} sürümüne güncelle`,
    updating: "Güncelleme indiriliyor…",
    updateFail: "Güncelleme denetlenemedi.",
  },
}[typeof lang !== "undefined" ? lang : "en"];

const checkIcon = '<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M5 12.5l4.5 4.5L19 7.5"/></svg>';
const warnIcon = '<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M12 8v5M12 16.5v.01M10.3 3.9L2.7 17.2A2 2 0 004.4 20h15.2a2 2 0 001.7-2.8L13.7 3.9a2 2 0 00-3.4 0z"/></svg>';

// ---- first-run notice: nothing is read from the network until it is accepted ----
async function showWelcome() {
  let accepted = true;
  try { accepted = await invoke("terms_accepted"); } catch { return; }
  if (accepted) return;
  const box = document.getElementById("welcome");
  document.getElementById("welcomeTitle").textContent = X.welcomeTitle;
  document.getElementById("welcomeLead").textContent = X.welcomeLead;
  document.getElementById("welcomeConsent").textContent = X.consent;
  const go = document.getElementById("welcomeGo");
  go.textContent = X.go;
  document.getElementById("welcomePoints").replaceChildren(...X.points.map(([kind, text]) => {
    const li = document.createElement("li");
    li.className = kind;
    li.innerHTML = kind === "warn" ? warnIcon : checkIcon;
    const span = document.createElement("span");
    span.textContent = text;
    li.append(span);
    return li;
  }));
  const check = document.getElementById("welcomeCheck");
  check.addEventListener("change", () => { go.disabled = !check.checked; });
  go.addEventListener("click", async () => {
    if (!check.checked) return;
    try { await invoke("accept_terms"); } catch { return; }
    box.hidden = true;
  });
  box.hidden = false;
  check.focus();
}

// ---- Npcap guide, drawn into the setup box by main.js ----
function renderNpcapGuide(setup, retry) {
  setup.classList.add("guide");
  const title = document.createElement("div");
  title.className = "guide-title";
  title.textContent = X.guideTitle;
  const lead = document.createElement("div");
  lead.className = "guide-lead";
  lead.textContent = X.guideLead;
  const steps = document.createElement("ol");
  steps.className = "steps";
  for (const t of [X.s1, X.s2, X.s3]) {
    const li = document.createElement("li");
    li.textContent = t;
    steps.append(li);
  }
  const actions = document.createElement("div");
  actions.className = "guide-actions";
  const open = document.createElement("button");
  open.className = "btn primary";
  open.textContent = X.openNpcap;
  open.onclick = () => invoke("open_url", { url: "https://npcap.com" }).catch(() => {});
  const again = document.createElement("button");
  again.className = "btn";
  again.textContent = X.retry;
  again.onclick = retry;
  actions.append(open, again);
  setup.replaceChildren(title, lead, steps, actions);
}

// ---- fight export (the fight open in History) ----
function exportStamp(rec) {
  const d = new Date(rec.startTimeMs || Date.now());
  const p = (n) => String(n).padStart(2, "0");
  const boss = String(rec.bossName || "fight").replace(/[^A-Za-z0-9]+/g, "-").replace(/^-|-$/g, "").slice(0, 30) || "fight";
  return `${boss}-${d.getFullYear()}${p(d.getMonth() + 1)}${p(d.getDate())}-${p(d.getHours())}${p(d.getMinutes())}`;
}
function csvCell(v) {
  let t = String(v ?? "");
  // A cell that starts like a formula would run when the file is opened in a spreadsheet.
  if (/^[=+\-@\t\r]/.test(t)) t = "'" + t;
  return /[",\n\r]/.test(t) ? `"${t.replace(/"/g, '""')}"` : t;
}
function fightExport(kind, record) {
  const rec = record ?? openFight;
  if (!rec) return null;
  const players = fightPlayers(rec);
  if (kind === "csv") {
    const rows = [["rank", "name", "class", "damage", "dps", "share_percent"]];
    players.forEach((p, i) => rows.push([i + 1, displayName(p), p.job, Math.round(p.dmg), p.dps.toFixed(1), ((p.dmg / (p.total || 1)) * 100).toFixed(2)]));
    return { name: `${exportStamp(rec)}.csv`, content: "﻿" + rows.map((r) => r.map(csvCell).join(",")).join("\r\n") + "\r\n" };
  }
  const out = {
    app: "BellMeter",
    boss: rec.bossName,
    startedAt: new Date(rec.startTimeMs).toISOString(),
    durationMs: rec.durationMs,
    totalDamage: rec.totalDamage,
    players: players.map((p, i) => ({ rank: i + 1, name: displayName(p), class: p.job, damage: Math.round(p.dmg), dps: Math.round(p.dps * 10) / 10, sharePercent: Math.round((p.dmg / (p.total || 1)) * 10000) / 100 })),
    skills: rec.details?.skills ?? [],
  };
  return { name: `${exportStamp(rec)}.json`, content: JSON.stringify(out, null, 2) };
}
async function doExport(kind, record) {
  const ex = fightExport(kind, record);
  if (!ex) return;
  try { toast(X.exportOk(await invoke("save_export", { name: ex.name, content: ex.content }))); } catch { toast(X.exportFail); }
}
document.getElementById("exportJson").addEventListener("click", () => doExport("json"));
document.getElementById("exportCsv").addEventListener("click", () => doExport("csv"));

// JSON / CSV buttons on each History row, so a fight can be exported without opening it.
const histObserver = new MutationObserver(() => {
  const rows = [...document.querySelectorAll("#histList .hist-row")];
  rows.forEach((row) => {
    if (row.querySelector(".hist-actions")) return;
    const box = document.createElement("div");
    box.className = "hist-actions";
    for (const kind of ["json", "csv"]) {
      const b = document.createElement("button");
      b.type = "button";
      b.className = "mini";
      b.textContent = kind.toUpperCase();
      b.addEventListener("click", async (e) => {
        e.stopPropagation();
        const idx = [...row.parentElement.children].indexOf(row);
        const f = histRows[idx];
        if (!f) return;
        try { await doExport(kind, await invoke("load_fight", { id: f.id })); } catch { toast(X.exportFail); }
      });
      b.addEventListener("keydown", (e) => e.stopPropagation());
      box.append(b);
    }
    row.append(box);
  });
  const n = document.getElementById("metaHist");
  if (n) n.textContent = rows.length ? String(rows.length) : "";
});
histObserver.observe(document.getElementById("histList"), { childList: true });
// The list the rows were drawn from, in the same order (newest first).
let histRows = [];
setInterval(async () => {
  try {
    const list = await invoke("list_fights");
    list.sort((a, b) => b.startTimeMs - a.startTimeMs);
    histRows = list;
    document.getElementById("metaHist").textContent = list.length ? String(list.length) : "";
  } catch { /* none */ }
}, 2000);

// Fight clock next to DPS in the side menu.
setInterval(async () => {
  try {
    const d = await invoke("get_dps");
    const live = Object.keys(d.map ?? {}).length > 0;
    const s = Math.max(0, Math.round((d.battleTime || 0) / 1000));
    document.getElementById("metaDps").textContent = live ? `${Math.floor(s / 60)}:${String(s % 60).padStart(2, "0")}` : "";
  } catch { /* backend not ready */ }
}, 1000);

// ---- log button and version ----
const logBtn = document.getElementById("openLog");
logBtn.textContent = X.openLog;
logBtn.addEventListener("click", () => invoke("open_log_folder").catch(() => toast(X.logFail)));
invoke("get_version").then((v) => { document.getElementById("appVersion").textContent = X.version(v); }).catch(() => {});

showTab("bosses");
showWelcome();

document.querySelectorAll("button.ext[data-url]").forEach((b) => {
  b.onclick = () => invoke("open_url", { url: b.dataset.url }).catch(() => {});
});

// ---- in-app update ----
// A found update is announced and waits for the click; nothing installs on its own.
const updBanner = document.getElementById("updateBanner");
async function checkUpdate(silent) {
  try {
    const update = await window.__TAURI__.updater.check();
    if (!update) { if (!silent) toast(X.upToDate); return; }
    toast(X.updateFound(update.version));
    updBanner.textContent = X.updateInstall(update.version);
    updBanner.hidden = false;
    updBanner.onclick = async () => {
      updBanner.disabled = true;
      toast(X.updating);
      try { await update.downloadAndInstall(); } catch { updBanner.disabled = false; toast(X.updateFail); }
    };
  } catch { if (!silent) toast(X.updateFail); }
}
const updBtn = document.getElementById("checkUpdate");
updBtn.textContent = X.checkUpdate;
updBtn.onclick = () => checkUpdate(false);
setTimeout(() => checkUpdate(true), 8000);
setInterval(() => checkUpdate(true), 6 * 3600 * 1000);

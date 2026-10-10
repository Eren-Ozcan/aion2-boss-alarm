"use strict";

// DPS timeline: damage per second over the fight, one line per player.
//
// The game does not report each hit's damage with its time, only each skill's
// total and its hit times, so a hit counts as its skill's average hit. That is
// the same approximation A2Tools DPS Meter draws its chart from.

const SVG_NS = "http://www.w3.org/2000/svg";
const W = 560, H = 200, ML = 46, MR = 10, MT = 10, MB = 24;

function svg(tag, attrs = {}) {
  const e = document.createElementNS(SVG_NS, tag);
  for (const [k, v] of Object.entries(attrs)) e.setAttribute(k, String(v));
  return e;
}

const compact = (n) => (n >= 1e6 ? `${(n / 1e6).toFixed(1)}M` : n >= 1e3 ? `${(n / 1e3).toFixed(n >= 1e4 ? 0 : 1)}k` : String(Math.round(n)));
const clock = (ms) => {
  const s = Math.max(0, Math.round(ms / 1000));
  return `${Math.floor(s / 60)}:${String(s % 60).padStart(2, "0")}`;
};

// Rolling-window DPS of one player's hits, sampled at `samples` points.
function sampleSeries(hits, t0, t1, samples, windowMs) {
  const out = [];
  let lo = 0, hi = 0, sum = 0;
  for (let i = 0; i < samples; i++) {
    const t = t0 + ((t1 - t0) * i) / (samples - 1);
    while (hi < hits.length && hits[hi].ts <= t) sum += hits[hi++].dmg;
    while (lo < hi && hits[lo].ts <= t - windowMs) sum -= hits[lo++].dmg;
    out.push(sum / (Math.min(windowMs, Math.max(t - t0, 1000)) / 1000));
  }
  return out;
}

/**
 * res: the skill rows of every player (`TargetDetailsResponse`).
 * label(actorId) -> display name. Returns an element, or null with nothing to draw.
 */
function timelineChart(res, label) {
  const byActor = new Map();
  for (const s of res?.skills ?? []) {
    const stamps = Array.isArray(s.hitTimestamps) ? s.hitTimestamps : [];
    if (!stamps.length || !(s.dmg > 0)) continue;
    const per = s.dmg / stamps.length;
    if (!byActor.has(s.actorId)) byActor.set(s.actorId, []);
    const arr = byActor.get(s.actorId);
    for (const ts of stamps) arr.push({ ts: Number(ts), dmg: per });
  }
  if (!byActor.size) return null;

  // At most eight lines (the palette has eight validated slots): the eight
  // strongest players. Each keeps one colour by id, not by rank, so a changing
  // order never repaints anyone.
  const strongest = [...byActor.entries()]
    .map(([id, hits]) => ({ id, hits: hits.sort((a, b) => a.ts - b.ts), total: hits.reduce((s, h) => s + h.dmg, 0) }))
    .sort((a, b) => b.total - a.total)
    .slice(0, 8)
    .sort((a, b) => a.id - b.id);

  let t0 = Infinity, t1 = -Infinity;
  for (const p of strongest) { t0 = Math.min(t0, p.hits[0].ts); t1 = Math.max(t1, p.hits[p.hits.length - 1].ts); }
  if (res.startTime > 0 && res.startTime <= t0) t0 = res.startTime;
  if (!(t1 > t0)) t1 = t0 + 1000;
  const windowMs = Math.max(5000, Math.min(15000, (t1 - t0) * 0.1));
  const SAMPLES = 80;
  const series = strongest.map((p, i) => ({ ...p, color: `var(--series-${i + 1})`, values: sampleSeries(p.hits, t0, t1, SAMPLES, windowMs) }));
  const yMax = Math.max(1, ...series.flatMap((s) => s.values)) * 1.08;

  const x = (i) => ML + ((W - ML - MR) * i) / (SAMPLES - 1);
  const y = (v) => MT + (H - MT - MB) * (1 - v / yMax);

  const root = document.createElement("div");
  root.className = "chart";
  const box = svg("svg", { viewBox: `0 0 ${W} ${H}`, role: "img", "aria-label": "DPS over time", class: "chart-svg" });

  for (let g = 0; g <= 4; g++) {
    const v = (yMax * g) / 4;
    box.append(svg("line", { x1: ML, x2: W - MR, y1: y(v), y2: y(v), class: "grid" }));
    const t = svg("text", { x: ML - 6, y: y(v) + 4, "text-anchor": "end", class: "axis" });
    t.textContent = compact(v);
    box.append(t);
  }
  for (let g = 0; g <= 4; g++) {
    const i = Math.round(((SAMPLES - 1) * g) / 4);
    const t = svg("text", { x: x(i), y: H - 6, "text-anchor": g === 0 ? "start" : g === 4 ? "end" : "middle", class: "axis" });
    t.textContent = clock((t1 - t0) * (i / (SAMPLES - 1)));
    box.append(t);
  }
  for (const s of series) {
    const d = s.values.map((v, i) => `${i ? "L" : "M"}${x(i).toFixed(1)} ${y(v).toFixed(1)}`).join(" ");
    box.append(svg("path", { d, class: "line", style: `stroke:${s.color}` }));
  }

  const cross = svg("line", { y1: MT, y2: H - MB, class: "cross", visibility: "hidden" });
  const dots = series.map((s) => svg("circle", { r: 4, class: "dot", style: `fill:${s.color}`, visibility: "hidden" }));
  box.append(cross, ...dots);
  const hit = svg("rect", { x: ML, y: MT, width: W - ML - MR, height: H - MT - MB, fill: "transparent" });
  box.append(hit);

  const tip = document.createElement("div");
  tip.className = "chart-tip";
  tip.hidden = true;

  const show = (clientX) => {
    const r = box.getBoundingClientRect();
    const px = ((clientX - r.left) / r.width) * W;
    const i = Math.max(0, Math.min(SAMPLES - 1, Math.round(((px - ML) / (W - ML - MR)) * (SAMPLES - 1))));
    cross.setAttribute("x1", x(i)); cross.setAttribute("x2", x(i)); cross.setAttribute("visibility", "visible");
    series.forEach((s, k) => {
      dots[k].setAttribute("cx", x(i)); dots[k].setAttribute("cy", y(s.values[i])); dots[k].setAttribute("visibility", "visible");
    });
    tip.replaceChildren();
    const head = document.createElement("div");
    head.className = "tip-time";
    head.textContent = clock((t1 - t0) * (i / (SAMPLES - 1)));
    tip.append(head);
    for (const s of [...series].sort((a, b) => b.values[i] - a.values[i])) {
      const row = document.createElement("div");
      row.className = "tip-row";
      const sw = document.createElement("span");
      sw.className = "swatch";
      sw.style.background = s.color;
      const name = document.createElement("span");
      name.textContent = label(s.id);
      const val = document.createElement("b");
      val.textContent = new Intl.NumberFormat().format(Math.round(s.values[i]));
      row.append(sw, name, val);
      tip.append(row);
    }
    tip.hidden = false;
    const left = ((x(i) / W) * r.width);
    tip.style.left = `${left > r.width / 2 ? left - tip.offsetWidth - 10 : left + 10}px`;
  };
  const hide = () => {
    tip.hidden = true;
    cross.setAttribute("visibility", "hidden");
    dots.forEach((d) => d.setAttribute("visibility", "hidden"));
  };
  hit.addEventListener("mousemove", (e) => show(e.clientX));
  hit.addEventListener("mouseleave", hide);

  const legend = document.createElement("div");
  legend.className = "chart-legend";
  for (const s of series) {
    const item = document.createElement("span");
    const sw = document.createElement("span");
    sw.className = "swatch";
    sw.style.background = s.color;
    item.append(sw, document.createTextNode(label(s.id)));
    legend.append(item);
  }

  root.append(box, tip, legend);
  return root;
}

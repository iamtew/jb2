(function () {
  // Keep in step with the seek fills in overlay/style.css.
  const SEEK = {
    "sesh-sofa": [0xe5f20d, 0xf20d0d],
    ice: [0x8ef6ff, 0xe8ffff, 0x3ad0ff],
    magenta: [0xff4dce, 0x9b5cff, 0xff4dce],
    white: [0xeeeeee],
    sunset: [0xff7a18, 0xff2d95],
    forest: [0x0b4d2a, 0x7dff6a],
    gold: [0xb8860b, 0xffe566],
    lava: [0x1a0500, 0xff3b00],
    ocean: [0x05204a, 0x1ec8c8],
    candy: [0xff2d95, 0x7dffc8],
    ultraviolet: [0xc77dff, 0x2a0a6a],
    acid: [0xb6ff00, 0xfff200],
    copper: [0x5a2a12, 0xe07a3a],
    cherry: [0x9b0018, 0xff6b8a],
    vapor: [0xff71ce, 0x01cdfe, 0xb967ff],
    graphite: [0x4a4f55, 0xc5cdd4],
    honey: [0xffe08a, 0x8a4b08],
  };
  const STYLES = { bands: 1, wave: 1, dots: 1, trail: 1, warp: 1 };
  const DIRS = { lr: 1, rl: 1, tb: 1, bt: 1 };
  const LOOK_DEFAULTS = {
    style: "bands",
    color: "",
    colorDir: "lr",
    bands: 16,
    smoothing: 0.35,
    mirror: false,
    peakFall: 0.35,
  };

  const layers = new Map();
  let enabled = false;
  let hideForStatus = true;
  let reduced = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
  let spectrum = [];
  let bass = 0;
  let punch = 0;
  let lastPunchTick = 0;
  let audioSource = null;
  let selectedId = "";
  let smoothing = LOOK_DEFAULTS.smoothing;
  let saveTimer = 0;
  let bound = false;

  function clampBands(n) {
    const v = Math.round(Number(n));
    if (!Number.isFinite(v)) return LOOK_DEFAULTS.bands;
    return Math.min(32, Math.max(1, v));
  }

  function clampUnit(n, fallback) {
    const v = Number(n);
    if (!Number.isFinite(v)) return fallback;
    return Math.min(1, Math.max(0, v));
  }

  function clampPeak(n, fallback) {
    const v = Number(n);
    if (!Number.isFinite(v)) return fallback;
    return Math.min(1, Math.max(-1, v));
  }

  function rangeNum(range) {
    return range && range.parentElement && range.parentElement.querySelector('input[type="number"]');
  }

  function fillRangeNum(range) {
    const num = rangeNum(range);
    if (num) num.value = range.value;
  }

  function pairRange(range) {
    const num = rangeNum(range);
    if (!range || !num) return;
    range.addEventListener("input", () => {
      num.value = range.value;
    });
    num.addEventListener("change", () => {
      const min = Number(range.min);
      const max = Number(range.max);
      let v = Number(num.value);
      if (!Number.isFinite(v)) v = Number(range.value);
      range.value = String(Math.min(max, Math.max(min, v)));
      num.value = range.value;
      range.dispatchEvent(new Event("input", { bubbles: true }));
      range.dispatchEvent(new Event("change", { bubbles: true }));
    });
    num.addEventListener("blur", () => {
      if (num.value === "" || !Number.isFinite(Number(num.value))) num.value = range.value;
    });
  }

  function mix(a, b, t) {
    const ch = (c, shift) => (c >> shift) & 255;
    const r = Math.round(ch(a, 16) + (ch(b, 16) - ch(a, 16)) * t);
    const g = Math.round(ch(a, 8) + (ch(b, 8) - ch(a, 8)) * t);
    const bl = Math.round(ch(a, 0) + (ch(b, 0) - ch(a, 0)) * t);
    return (r << 16) | (g << 8) | bl;
  }

  function colorAt(stops, t) {
    if (!stops || !stops.length) return 0xffffff;
    if (stops.length === 1) return stops[0];
    const x = Math.min(1, Math.max(0, t)) * (stops.length - 1);
    const i = Math.min(stops.length - 2, Math.floor(x));
    return mix(stops[i], stops[i + 1], x - i);
  }

  function lookOf(src) {
    const next = {
      style: LOOK_DEFAULTS.style,
      color: LOOK_DEFAULTS.color,
      colorDir: LOOK_DEFAULTS.colorDir,
      bands: LOOK_DEFAULTS.bands,
      mirror: LOOK_DEFAULTS.mirror,
      peakFall: LOOK_DEFAULTS.peakFall,
      hidden: true,
    };
    if (!src) return next;
    if (STYLES[src.style]) next.style = src.style;
    if (typeof src.color === "string") next.color = SEEK[src.color] ? src.color : "";
    if (typeof src.colorDir === "string") {
      next.colorDir = DIRS[src.colorDir] ? src.colorDir : LOOK_DEFAULTS.colorDir;
    }
    if (src.bands != null) next.bands = clampBands(src.bands);
    if (src.spectrumBands != null) next.bands = clampBands(src.spectrumBands);
    if (typeof src.mirror === "boolean") next.mirror = src.mirror;
    if (src.peakFall != null) next.peakFall = clampPeak(src.peakFall, LOOK_DEFAULTS.peakFall);
    next.hidden = Boolean(src.hidden);
    return next;
  }

  function stopsForFrame(look) {
    const id = look.color || document.getElementById("songInfo")?.getAttribute("data-seek") || "sesh-sofa";
    return SEEK[id] || SEEK["sesh-sofa"];
  }

  function isStage() {
    return Boolean(document.getElementById("stageCanvas"));
  }

  function setLookLock() {
    const set = document.getElementById("vizLook");
    if (!set) return;
    const layer = layers.get(selectedId);
    set.disabled = !layer || layer.look.hidden;
  }

  function setEnabled(on) {
    enabled = Boolean(on);
    if (enabled) connectAudio();
    else disconnectAudio();
    applyVisibility();
  }

  function setStatus(text) {
    const el = document.getElementById("vizStatus");
    if (el) el.textContent = text || "";
  }

  function saveViz(patch) {
    fetch("/api/config", {
      method: "PUT",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ audioVisualizer: patch }),
    }).catch(() => {});
  }

  function selectedLayer() {
    return layers.get(selectedId) || null;
  }

  function syncControls() {
    const layer = selectedLayer();
    const look = layer ? layer.look : LOOK_DEFAULTS;
    const styleSel = document.getElementById("vizStyle");
    if (styleSel) styleSel.value = look.style;
    const colorSel = document.getElementById("vizColor");
    if (colorSel) colorSel.value = look.color;
    const dirSel = document.getElementById("vizColorDir");
    if (dirSel) dirSel.value = DIRS[look.colorDir] ? look.colorDir : LOOK_DEFAULTS.colorDir;
    const bands = document.getElementById("vizBands");
    if (bands) {
      bands.value = String(look.bands);
      fillRangeNum(bands);
    }
    const smoothingEl = document.getElementById("vizSmoothing");
    if (smoothingEl) {
      smoothingEl.value = String(smoothing);
      fillRangeNum(smoothingEl);
    }
    const mirror = document.getElementById("vizMirror");
    if (mirror) mirror.checked = look.mirror;
    const peak = document.getElementById("vizPeak");
    if (peak) {
      peak.value = String(look.peakFall);
      fillRangeNum(peak);
    }
    setLookLock();
  }

  function readControls() {
    const layer = selectedLayer();
    if (!layer) return;
    const look = layer.look;
    const styleSel = document.getElementById("vizStyle");
    if (styleSel && STYLES[styleSel.value]) look.style = styleSel.value;
    const colorSel = document.getElementById("vizColor");
    if (colorSel) look.color = SEEK[colorSel.value] ? colorSel.value : "";
    const dirSel = document.getElementById("vizColorDir");
    if (dirSel) look.colorDir = DIRS[dirSel.value] ? dirSel.value : LOOK_DEFAULTS.colorDir;
    const bands = document.getElementById("vizBands");
    if (bands) look.bands = clampBands(bands.value);
    const smoothingEl = document.getElementById("vizSmoothing");
    if (smoothingEl) smoothing = clampUnit(smoothingEl.value, LOOK_DEFAULTS.smoothing);
    const mirror = document.getElementById("vizMirror");
    if (mirror) look.mirror = mirror.checked;
    const peak = document.getElementById("vizPeak");
    if (peak) look.peakFall = clampPeak(peak.value, LOOK_DEFAULTS.peakFall);
  }

  function emitLook() {
    const layer = selectedLayer();
    if (!layer || typeof window.JB2Viz.onLookChange !== "function") return;
    window.JB2Viz.onLookChange(layer.id, layer.look);
  }

  function scheduleLookSave() {
    window.clearTimeout(saveTimer);
    saveTimer = window.setTimeout(() => {
      saveTimer = 0;
      emitLook();
    }, 120);
  }

  function resampleSpectrum(src, bands) {
    const n = src.length;
    if (!n) return [];
    const count = clampBands(bands);
    const out = [];
    if (count <= n) {
      for (let i = 0; i < count; i++) {
        const a = Math.floor((i * n) / count);
        const b = Math.max(a + 1, Math.floor(((i + 1) * n) / count));
        let sum = 0;
        for (let j = a; j < b; j++) sum += Number(src[j]) || 0;
        out.push(sum / (b - a));
      }
      return out;
    }
    for (let i = 0; i < count; i++) {
      const t = count <= 1 ? 0 : i / (count - 1);
      const x = t * (n - 1);
      const j = Math.min(n - 2, Math.floor(x));
      const f = x - j;
      out.push((Number(src[j]) || 0) * (1 - f) + (Number(src[j + 1]) || 0) * f);
    }
    return out;
  }

  function stepPeaks(layer, levels, dt) {
    const peaks = layer.peaks;
    if (peaks.length !== levels.length) {
      layer.peaks = levels.slice();
      return;
    }
    const fall = layer.look.peakFall;
    if (!fall) return;
    // Positive drops toward the bar. Negative flies up, then re-arms at 1.
    const drop = Math.abs(fall) * 0.22 * dt;
    for (let i = 0; i < levels.length; i++) {
      const level = levels[i];
      const peak = peaks[i] || 0;
      if (fall > 0) {
        peaks[i] = Math.max(level, peak - drop);
        continue;
      }
      if (level >= peak) {
        peaks[i] = level;
        continue;
      }
      const next = peak + drop;
      peaks[i] = next > 1 ? level : next;
    }
  }

  function colorT(look, i, n, level) {
    const dir = DIRS[look.colorDir] ? look.colorDir : "lr";
    if (dir === "tb" || dir === "bt") {
      const t = Math.min(1, Math.max(0, Number(level) || 0));
      return dir === "bt" ? 1 - t : t;
    }
    const t = n <= 1 ? 0.5 : i / (n - 1);
    return dir === "rl" ? 1 - t : t;
  }

  function bandColor(look, stops, i, n, level) {
    return colorAt(stops, colorT(look, i, n, level));
  }

  function paintBands(gfx, look, w, h, levels, peaks, stops) {
    const n = levels.length;
    const gap = n <= 1 ? 0 : Math.max(2, Math.round(w / 400));
    const bw = Math.max(1, (w - gap * (n + 1)) / n);
    const maxH = h * 0.92 * (1 + punch * 0.18);
    const mid = h / 2;
    const showPeak = look.peakFall !== 0;
    for (let i = 0; i < n; i++) {
      const color = bandColor(look, stops, i, n, levels[i]);
      const bh = Math.max(1, levels[i] * maxH);
      const x = gap + i * (bw + gap);
      if (look.mirror) {
        gfx.rect(x, mid - bh / 2, bw, bh).fill({ color, alpha: 0.9 });
      } else {
        gfx.rect(x, h - bh, bw, bh).fill({ color, alpha: 0.9 });
      }
      if (!showPeak) continue;
      const ph = Math.max(2, peaks[i] * maxH);
      if (look.mirror) {
        gfx.rect(x, mid - ph / 2, bw, 2).fill({ color, alpha: 1 });
        gfx.rect(x, mid + ph / 2 - 2, bw, 2).fill({ color, alpha: 1 });
      } else {
        gfx.rect(x, h - ph, bw, 2).fill({ color, alpha: 1 });
      }
    }
  }

  function waveY(look, h, level, maxH, top) {
    const mid = h / 2;
    if (!look.mirror) return top ? h - level * maxH : h;
    return top ? mid - (level * maxH) / 2 : mid + (level * maxH) / 2;
  }

  function paintWave(gfx, look, w, h, levels, peaks, stops) {
    const n = levels.length;
    const maxH = h * 0.92 * (1 + punch * 0.18);
    if (n === 1) {
      const color = bandColor(look, stops, 0, 1, levels[0]);
      const top = waveY(look, h, levels[0], maxH, true);
      const bot = waveY(look, h, levels[0], maxH, false);
      gfx.rect(0, Math.min(top, bot), w, Math.max(1, Math.abs(bot - top))).fill({ color, alpha: 0.9 });
      if (look.peakFall !== 0) {
        const py = waveY(look, h, peaks[0], maxH, true);
        gfx.rect(0, py, w, 2).fill({ color, alpha: 1 });
        if (look.mirror) gfx.rect(0, waveY(look, h, peaks[0], maxH, false) - 2, w, 2).fill({ color, alpha: 1 });
      }
      return;
    }
    for (let i = 0; i < n - 1; i++) {
      const x0 = (i / (n - 1)) * w;
      const x1 = ((i + 1) / (n - 1)) * w;
      const color = bandColor(look, stops, i, n, levels[i]);
      gfx
        .poly([
          x0, waveY(look, h, levels[i], maxH, true),
          x1, waveY(look, h, levels[i + 1], maxH, true),
          x1, waveY(look, h, levels[i + 1], maxH, false),
          x0, waveY(look, h, levels[i], maxH, false),
        ])
        .fill({ color, alpha: 0.9 });
    }
    if (!look.peakFall) return;
    for (let i = 0; i < n - 1; i++) {
      const color = bandColor(look, stops, i, n, levels[i]);
      const x0 = (i / (n - 1)) * w;
      const x1 = ((i + 1) / (n - 1)) * w;
      gfx.moveTo(x0, waveY(look, h, peaks[i], maxH, true));
      gfx.lineTo(x1, waveY(look, h, peaks[i + 1], maxH, true));
      gfx.stroke({ width: 2, color, alpha: 1 });
      if (!look.mirror) continue;
      gfx.moveTo(x0, waveY(look, h, peaks[i], maxH, false));
      gfx.lineTo(x1, waveY(look, h, peaks[i + 1], maxH, false));
      gfx.stroke({ width: 2, color, alpha: 1 });
    }
  }

  function paintDots(gfx, look, w, h, levels, peaks, stops) {
    const n = levels.length;
    const slot = w / n;
    const maxR = Math.min(slot * 0.46, h * 0.42);
    const cy = look.mirror ? h * 0.33 : h * 0.5;
    const showPeak = look.peakFall !== 0;
    for (let i = 0; i < n; i++) {
      const color = bandColor(look, stops, i, n, levels[i]);
      const cx = slot * i + slot / 2;
      const r = Math.max(1.5, levels[i] * maxR);
      gfx.circle(cx, cy, r).fill({ color, alpha: 0.9 });
      if (look.mirror) gfx.circle(cx, h - cy, r).fill({ color, alpha: 0.9 });
      if (!showPeak) continue;
      const pr = Math.max(2, peaks[i] * maxR);
      gfx.circle(cx, cy, pr).stroke({ width: 1.5, color, alpha: 1 });
      if (look.mirror) gfx.circle(cx, h - cy, pr).stroke({ width: 1.5, color, alpha: 1 });
    }
  }

  function strokeSnap(gfx, look, w, h, snap, stops, alpha, maxH) {
    const n = snap.length;
    if (!n) return;
    const width = 2 + punch * 2;
    if (n === 1) {
      const color = bandColor(look, stops, 0, 1, snap[0]);
      gfx.moveTo(0, waveY(look, h, snap[0], maxH, true));
      gfx.lineTo(w, waveY(look, h, snap[0], maxH, true));
      gfx.stroke({ width, color, alpha });
      if (!look.mirror) return;
      gfx.moveTo(0, waveY(look, h, snap[0], maxH, false));
      gfx.lineTo(w, waveY(look, h, snap[0], maxH, false));
      gfx.stroke({ width, color, alpha });
      return;
    }
    for (let i = 0; i < n - 1; i++) {
      const x0 = (i / (n - 1)) * w;
      const x1 = ((i + 1) / (n - 1)) * w;
      const color = bandColor(look, stops, i, n, snap[i]);
      gfx.moveTo(x0, waveY(look, h, snap[i], maxH, true));
      gfx.lineTo(x1, waveY(look, h, snap[i + 1], maxH, true));
      gfx.stroke({ width, color, alpha });
      if (!look.mirror) continue;
      gfx.moveTo(x0, waveY(look, h, snap[i], maxH, false));
      gfx.lineTo(x1, waveY(look, h, snap[i + 1], maxH, false));
      gfx.stroke({ width, color, alpha });
    }
  }

  function paintTrail(gfx, look, layer, w, h, levels, stops) {
    const cap = !look.peakFall ? 1 : 1 + Math.round(Math.abs(look.peakFall) * 20);
    if (layer.trailHist.length && layer.trailHist[0].length !== levels.length) layer.trailHist = [];
    layer.trailHist.push(levels.slice());
    while (layer.trailHist.length > cap) layer.trailHist.shift();
    const maxH = h * 0.92 * (1 + punch * 0.18);
    for (let t = 0; t < layer.trailHist.length; t++) {
      const alpha = 0.12 + 0.88 * ((t + 1) / layer.trailHist.length);
      strokeSnap(gfx, look, w, h, layer.trailHist[t], stops, alpha, maxH);
    }
  }

  function sampleLevel(levels, t) {
    const n = levels.length;
    if (n <= 1) return Number(levels[0]) || 0;
    const x = Math.min(1, Math.max(0, t)) * (n - 1);
    const i = Math.min(n - 2, Math.floor(x));
    const f = x - i;
    return (Number(levels[i]) || 0) * (1 - f) + (Number(levels[i + 1]) || 0) * f;
  }

  function paintWarp(gfx, look, layer, w, h, levels, stops, dt) {
    const n = levels.length;
    if (!n) return;
    layer.warpZ = (layer.warpZ + (0.01 + bass * 0.05 + punch * 0.08) * dt) % 1;
    const horizon = look.mirror ? h * 0.5 : Math.min(h * 0.18, Math.max(8, h - 48));
    const vpX = w / 2;
    const floorH = h - horizon;
    if (floorH < 4) return;
    const rows = 14;
    const steps = Math.max(2, n);
    const warpZ = layer.warpZ;

    function yAt(depth, t, sign) {
      const persp = depth * depth;
      const lift = sampleLevel(levels, t) * floorH * 0.22 * persp;
      return horizon + sign * (floorH * persp - lift);
    }

    function xAt(depth, t) {
      return vpX + (t * w - vpX) * depth;
    }

    function drawFloor(sign) {
      for (let r = 0; r < rows; r++) {
        const depth = (r / rows + warpZ) % 1;
        if (depth < 0.04) continue;
        const alpha = 0.15 + 0.75 * depth;
        const width = 1 + depth * 1.5;
        for (let i = 0; i < steps - 1; i++) {
          const t0 = i / (steps - 1);
          const t1 = (i + 1) / (steps - 1);
          const color = bandColor(look, stops, Math.min(n - 1, i), n, sampleLevel(levels, t0));
          gfx.moveTo(xAt(depth, t0), yAt(depth, t0, sign));
          gfx.lineTo(xAt(depth, t1), yAt(depth, t1, sign));
          gfx.stroke({ width, color, alpha });
        }
      }
      for (let i = 0; i <= steps; i++) {
        const t = i / steps;
        const color = bandColor(look, stops, Math.min(n - 1, i), n, sampleLevel(levels, t));
        const segs = 8;
        for (let s = 0; s < segs; s++) {
          const d0 = s / segs;
          const d1 = (s + 1) / segs;
          gfx.moveTo(xAt(d0, t), yAt(d0, t, sign));
          gfx.lineTo(xAt(d1, t), yAt(d1, t, sign));
          gfx.stroke({ width: 1, color, alpha: 0.18 + 0.62 * d1 });
        }
      }
    }

    drawFloor(1);
    if (look.mirror) drawFloor(-1);
  }

  function resizeLayer(layer) {
    if (!layer.app || !layer.host) return;
    const w = layer.host.clientWidth;
    const h = layer.host.clientHeight;
    if (w < 2 || h < 2) return;
    if (layer.app.renderer.width === w && layer.app.renderer.height === h) return;
    layer.app.renderer.resize(w, h);
  }

  function decayPunch(ticker) {
    const t = ticker && ticker.lastTime;
    if (t === lastPunchTick) return;
    lastPunchTick = t;
    punch *= 0.86;
  }

  function paintLayer(layer, ticker) {
    const gfx = layer.gfx;
    const app = layer.app;
    if (!gfx || !app) return;
    const w = app.renderer.width;
    const h = app.renderer.height;
    gfx.clear();
    decayPunch(ticker);
    if (!enabled || hideForStatus || reduced || layer.look.hidden || w < 2 || h < 2) return;
    const levels = resampleSpectrum(spectrum, layer.look.bands);
    if (!levels.length) return;
    const dt = ticker && ticker.deltaTime ? ticker.deltaTime : 1;
    stepPeaks(layer, levels, dt);
    const stops = stopsForFrame(layer.look);
    const tint = Math.min(0.18, bass * 0.16);
    if (tint > 0.01) gfx.rect(0, 0, w, h).fill({ color: stops[0], alpha: tint });
    if (layer.look.style !== "trail") layer.trailHist = [];
    if (layer.look.style === "wave") paintWave(gfx, layer.look, w, h, levels, layer.peaks, stops);
    else if (layer.look.style === "dots") paintDots(gfx, layer.look, w, h, levels, layer.peaks, stops);
    else if (layer.look.style === "trail") paintTrail(gfx, layer.look, layer, w, h, levels, stops);
    else if (layer.look.style === "warp") paintWarp(gfx, layer.look, layer, w, h, levels, stops, dt);
    else paintBands(gfx, layer.look, w, h, levels, layer.peaks, stops);
  }

  function applyVisibility() {
    layers.forEach((layer) => {
      const box = layer.box;
      if (!box) return;
      if (isStage()) {
        box.hidden = false;
        return;
      }
      const hide = !enabled || hideForStatus || layer.look.hidden;
      if (box.hidden !== hide) box.hidden = hide;
      if (!hide) window.requestAnimationFrame(() => resizeLayer(layer));
    });
  }

  function destroyLayer(layer) {
    if (layer.ro) layer.ro.disconnect();
    if (layer.app) {
      try {
        layer.app.destroy(true);
      } catch (_) {
        /* already gone */
      }
    }
    layer.app = null;
    layer.gfx = null;
  }

  function mountPixi(layer) {
    if (typeof PIXI === "undefined" || !layer.host || layer.app || layer.pending) return;
    layer.pending = true;
    const app = new PIXI.Application();
    app
      .init({
        width: 32,
        height: 32,
        backgroundAlpha: 0,
        antialias: false,
        autoDensity: false,
        resolution: 1,
        preference: "webgl",
      })
      .then(function () {
        if (!layers.has(layer.id)) {
          app.destroy(true);
          return;
        }
        layer.pending = false;
        layer.app = app;
        app.canvas.setAttribute("aria-hidden", "true");
        layer.host.appendChild(app.canvas);
        layer.gfx = new PIXI.Graphics();
        app.stage.addChild(layer.gfx);
        app.ticker.add((ticker) => paintLayer(layer, ticker));
        if (typeof ResizeObserver === "function") {
          layer.ro = new ResizeObserver(() => resizeLayer(layer));
          layer.ro.observe(layer.host);
        }
        resizeLayer(layer);
      })
      .catch(function () {
        layer.pending = false;
        if (layer.host) layer.host.hidden = true;
      });
  }

  function onAudio(ev) {
    let data;
    try {
      data = JSON.parse(ev.data || "{}");
    } catch {
      return;
    }
    if (data.type === "status") {
      const message = String(data.message || "");
      const device = String(data.device || "");
      hideForStatus =
        data.state === "error" || (data.state === "silent" && /disabled/i.test(message));
      if (data.state === "capturing") setStatus(device || "capturing");
      else if (data.state === "error" || data.state === "device_changed") setStatus(message || data.state);
      else setStatus("");
      applyVisibility();
      return;
    }
    if (data.type !== "audio" || reduced) return;
    spectrum = Array.isArray(data.spectrum) ? data.spectrum : [];
    bass = Number(data.bass) || 0;
    if (data.beat) punch = 1;
    hideForStatus = false;
    applyVisibility();
  }

  function connectAudio() {
    if (audioSource) return;
    audioSource = new EventSource("/sse/audio");
    audioSource.addEventListener("audio", onAudio);
  }

  function disconnectAudio() {
    if (!audioSource) return;
    audioSource.close();
    audioSource = null;
  }

  let deviceList = null;

  function fillDevices(selected) {
    const sel = document.getElementById("vizDevice");
    if (!sel) return Promise.resolve();
    const load = deviceList || (deviceList = fetch("/api/audio/devices")
      .then((res) => res.json())
      .then((env) => (env && env.data && env.data.devices) || [])
      .catch(() => []));
    return load.then((devices) => {
      const want = selected || "default";
      sel.replaceChildren();
      const fallback = document.createElement("option");
      fallback.value = "default";
      fallback.textContent = "Default output";
      sel.appendChild(fallback);
      devices.forEach((device) => {
        if (!device || !device.name || device.name === "default") return;
        const opt = document.createElement("option");
        opt.value = device.name;
        opt.textContent = device.default ? `${device.name} (system default)` : device.name;
        sel.appendChild(opt);
      });
      if (![...sel.options].some((opt) => opt.value === want)) {
        const extra = document.createElement("option");
        extra.value = want;
        extra.textContent = want;
        sel.appendChild(extra);
      }
      sel.value = want;
    });
  }

  function followEnabled() {
    fetch("/api/config")
      .then((res) => res.json())
      .then((env) => {
        const viz = (env && env.data && env.data.audioVisualizer) || {};
        setEnabled(Boolean(viz.enabled));
        if (viz.smoothing != null) smoothing = clampUnit(viz.smoothing, LOOK_DEFAULTS.smoothing);
        syncControls();
        return fillDevices(viz.device || "default");
      })
      .catch(() => {});
  }

  function bindControls() {
    if (bound) return;
    bound = true;
    ["vizBands", "vizSmoothing", "vizPeak"].forEach((id) => pairRange(document.getElementById(id)));
    ["vizStyle", "vizColor", "vizColorDir", "vizBands", "vizMirror", "vizPeak"].forEach((id) => {
      const el = document.getElementById(id);
      if (!el) return;
      const event = el.type === "range" || el.type === "checkbox" ? "input" : "change";
      el.addEventListener(event, () => {
        readControls();
        scheduleLookSave();
      });
    });
    const smoothingEl = document.getElementById("vizSmoothing");
    if (smoothingEl) {
      smoothingEl.addEventListener("input", () => {
        smoothing = clampUnit(smoothingEl.value, LOOK_DEFAULTS.smoothing);
        window.clearTimeout(saveTimer);
        saveTimer = window.setTimeout(() => {
          saveTimer = 0;
          saveViz({ smoothing });
        }, 120);
      });
    }
    document.querySelectorAll("[data-viz-reset]").forEach((btn) => {
      btn.addEventListener("click", () => {
        const key = btn.getAttribute("data-viz-reset");
        if (key === "smoothing") {
          smoothing = LOOK_DEFAULTS.smoothing;
          syncControls();
          saveViz({ smoothing });
          return;
        }
        const layer = selectedLayer();
        if (layer && Object.prototype.hasOwnProperty.call(LOOK_DEFAULTS, key)) {
          layer.look[key] = LOOK_DEFAULTS[key];
        }
        syncControls();
        emitLook();
      });
    });
    const deviceSel = document.getElementById("vizDevice");
    if (deviceSel) {
      deviceSel.addEventListener("change", () => {
        saveViz({ device: deviceSel.value || "default" });
      });
    }
    window.addEventListener("jb2-viz", (event) => {
      const data = event.detail || {};
      if (typeof data.enabled === "boolean") setEnabled(data.enabled);
      if (data.smoothing != null) {
        smoothing = clampUnit(data.smoothing, LOOK_DEFAULTS.smoothing);
        syncControls();
      }
    });
    const events = new EventSource("/api/events");
    events.addEventListener("audioVisualizer", (event) => {
      try {
        const data = JSON.parse(event.data || "{}");
        if (typeof data.enabled === "boolean") setEnabled(data.enabled);
        if (data.smoothing != null) {
          smoothing = clampUnit(data.smoothing, LOOK_DEFAULTS.smoothing);
          syncControls();
        }
      } catch (_) {
        /* ignore */
      }
    });
    followEnabled();
  }

  function select(id) {
    selectedId = id || "";
    syncControls();
  }

  function sync(layout) {
    bindControls();
    const list = (layout && layout.vizLayers) || [];
    const keep = new Set();
    list.forEach((src) => {
      if (!src || !src.id) return;
      keep.add(src.id);
      let layer = layers.get(src.id);
      const box = document.querySelector(`[data-viz-id="${src.id}"]`);
      const host = box ? box.querySelector(".viz-layer") : null;
      if (!layer) {
        layer = {
          id: src.id,
          box,
          host,
          app: null,
          gfx: null,
          pending: false,
          ro: null,
          look: lookOf(src),
          peaks: [],
          trailHist: [],
          warpZ: 0,
        };
        layers.set(src.id, layer);
      } else {
        layer.box = box;
        layer.host = host;
        layer.look = lookOf(src);
      }
      if (host) mountPixi(layer);
    });
    Array.from(layers.keys()).forEach((id) => {
      if (keep.has(id)) return;
      destroyLayer(layers.get(id));
      layers.delete(id);
    });
    if (selectedId && !layers.has(selectedId)) selectedId = "";
    if (!selectedId && list.length) selectedId = list[0].id;
    applyVisibility();
    syncControls();
  }

  window.JB2Viz = {
    sync,
    select,
    lookDefaults: LOOK_DEFAULTS,
    onLookChange: null,
  };

  bindControls();
})();

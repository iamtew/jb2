(function () {
  const SKINS = {
    "lcd-glass": { a: 0x3ec8ff, b: 0xff4cf0 },
    "lower-third": { a: 0xee1111, b: 0xf4f4f4 },
    "amber-terminal": { a: 0xffb000, b: 0xff6a00 },
    crypt: { a: 0xc02020, b: 0xff6a20 },
    fraktur: { a: 0xe0c060, b: 0x8a6a20 },
    konstrukt: { a: 0xcc0000, b: 0xf4ead8 },
    street: { a: 0x39ff14, b: 0xff2bd6 },
    drop: { a: 0xff78b4, b: 0xffdc50 },
    capsule: { a: 0x3cb89a, b: 0xa8fff0 },
    poster: { a: 0x00c8ff, b: 0xe85d04 },
  };

  let app = null;
  let gfx = null;
  let host = null;
  let enabled = false;
  let hideForStatus = true;
  let reduced = false;
  let spectrum = [];
  let bass = 0;
  let punch = 0;
  let audioSource = null;

  function skin() {
    const theme = document.getElementById("songInfo")?.getAttribute("data-theme") || "lcd-glass";
    return SKINS[theme] || SKINS["lcd-glass"];
  }

  function applyVisibility() {
    const frame = document.getElementById("vizBox") || host;
    if (!frame) return;
    const hide = !enabled;
    if (frame.hidden === hide) return;
    frame.hidden = hide;
    if (!hide) window.requestAnimationFrame(resize);
  }

  function setEnabled(on) {
    enabled = Boolean(on);
    const box = document.getElementById("vizInput");
    if (box && box.checked !== enabled) box.checked = enabled;
    applyVisibility();
    if (enabled) connectAudio();
    else disconnectAudio();
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

  function resize() {
    if (!app || !host || host.hidden) return;
    const w = host.clientWidth;
    const h = host.clientHeight;
    if (w < 2 || h < 2) return;
    if (app.renderer.width === w && app.renderer.height === h) return;
    app.renderer.resize(w, h);
  }

  function paint() {
    if (!gfx || !app) return;
    const colors = skin();
    const w = app.renderer.width;
    const h = app.renderer.height;
    gfx.clear();
    if (!enabled || hideForStatus || reduced || w < 2 || h < 2) return;
    const n = spectrum.length;
    if (!n) return;
    const tint = Math.min(0.18, bass * 0.16);
    if (tint > 0.01) gfx.rect(0, 0, w, h).fill({ color: colors.a, alpha: tint });
    const gap = Math.max(2, Math.round(w / 400));
    const bw = (w - gap * (n + 1)) / n;
    const maxH = h * 0.92 * (1 + punch * 0.18);
    for (let i = 0; i < n; i++) {
      const bh = Math.max(1, (Number(spectrum[i]) || 0) * maxH);
      gfx.rect(gap + i * (bw + gap), h - bh, Math.max(1, bw), bh).fill({
        color: i % 2 ? colors.b : colors.a,
        alpha: 0.9,
      });
    }
    punch *= 0.86;
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
        return fillDevices(viz.device || "default");
      })
      .catch(() => {});
  }

  function init(el) {
    host = el;
    reduced = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
    const box = document.getElementById("vizInput");
    if (box) {
      box.addEventListener("change", () => {
        setEnabled(box.checked);
        saveViz({ enabled: box.checked });
      });
    }
    const deviceSel = document.getElementById("vizDevice");
    if (deviceSel) {
      deviceSel.addEventListener("change", () => {
        saveViz({ device: deviceSel.value || "default" });
      });
    }
    window.addEventListener("jb2-viz-enabled", (event) => {
      setEnabled(Boolean(event.detail));
    });
    followEnabled();
    if (typeof PIXI === "undefined") return;
    app = new PIXI.Application();
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
        app.canvas.setAttribute("aria-hidden", "true");
        host.appendChild(app.canvas);
        gfx = new PIXI.Graphics();
        app.stage.addChild(gfx);
        app.ticker.add(paint);
        if (typeof ResizeObserver === "function") new ResizeObserver(resize).observe(host);
        resize();
      })
      .catch(function () {
        if (host) host.hidden = true;
      });
  }

  const hostEl = document.querySelector(".viz-layer");
  if (hostEl) init(hostEl);
})();

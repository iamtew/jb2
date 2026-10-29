(function () {
  const SNAP_PX = 12;
  const PUT_MS = 100;
  const DEMO_DURATION = 180;
  const MIN_W = 280;
  const BASE_W = 500;
  const PREVIEW_MODES = ["cycle", "playing", "paused", "nosignal"];
  const PREVIEW_LABELS = {
    cycle: "Preview: cycle",
    playing: "Preview: playing",
    paused: "Preview: paused",
    nosignal: "Preview: no signal",
  };

  const canvas = document.getElementById("stageCanvas");
  const fit = document.getElementById("stageFit");
  const overlayBox = document.getElementById("overlayBox");
  const songInfo = document.getElementById("songInfo");
  const overlayState = document.getElementById("overlayState");
  const btn720 = document.getElementById("size720");
  const btn1080 = document.getElementById("size1080");
  const themeSelect = document.getElementById("themeSelect");
  const seekSelect = document.getElementById("seekSelect");
  const playingOpacity = document.getElementById("playingOpacity");
  const pausedOpacity = document.getElementById("pausedOpacity");
  const pulseInput = document.getElementById("pulseInput");
  const glassInput = document.getElementById("glassInput");
  const gridSelect = document.getElementById("gridSelect");
  const previewBtn = document.getElementById("previewBtn");
  const ui = {
    artist: document.getElementById("artist"),
    artistWrap: document.getElementById("artistWrap"),
    title: document.getElementById("title"),
    titleWrap: document.getElementById("titleWrap"),
    album: document.getElementById("album"),
    albumWrap: document.getElementById("albumWrap"),
    playState: document.getElementById("playState"),
    currentTime: document.getElementById("currentTime"),
    totalTime: document.getElementById("totalTime"),
    marker: document.getElementById("marker"),
    bar: document.getElementById("bar"),
    fill: document.getElementById("fill"),
  };

  const DEFAULTS = {
    theme: "lcd-glass",
    seekStyle: "sesh-sofa",
    playingOpacity: 1,
    pausedOpacity: 0.25,
    panelW: 500,
    panelH: 0,
    pulse: 3,
    glass: true,
    grid: "thirds-center",
  };
  const layout = {
    x: 0,
    y: 0,
    canvasW: 1920,
    canvasH: 1080,
    ...DEFAULTS,
  };
  let drag = null;
  let putTimer = null;
  let demoPaused = false;
  let demoPos = 0;
  let previewMode = "cycle";
  let cycleTimer = null;

  function formatTime(sec) {
    const n = Math.max(0, Math.floor(sec));
    const m = Math.floor(n / 60);
    const s = n % 60;
    return `${String(m).padStart(2, "0")}:${String(s).padStart(2, "0")}`;
  }

  function getMarqueeTrack(textEl) {
    if (!textEl) return null;
    let track = textEl.querySelector(".overlay-marquee-track");
    if (track) return track;
    track = document.createElement("span");
    track.className = "overlay-marquee-track";
    track.textContent = textEl.textContent || "";
    textEl.textContent = "";
    textEl.appendChild(track);
    return track;
  }

  function updateTextMarquee(textEl, wrapEl) {
    if (!textEl || !wrapEl) return;
    const track = getMarqueeTrack(textEl);
    if (!track) return;
    textEl.classList.remove("is-marquee");
    textEl.style.removeProperty("--marquee-distance");
    window.requestAnimationFrame(() => {
      const overflowWidth = track.scrollWidth - wrapEl.clientWidth;
      if (overflowWidth > 4) {
        textEl.style.setProperty("--marquee-distance", `${overflowWidth}px`);
        textEl.classList.add("is-marquee");
      }
    });
  }

  function updateMarker() {
    if (!ui.bar || !ui.marker || !ui.fill) return;
    const barWidth = ui.bar.clientWidth;
    const pct = demoPos / DEMO_DURATION;
    const x = Math.min(barWidth - 6, Math.max(0, barWidth * pct));
    ui.marker.style.transform = `translateX(${x}px)`;
    ui.fill.style.width = `${Math.max(0, Math.min(barWidth, barWidth * pct))}px`;
    if (ui.currentTime) ui.currentTime.textContent = formatTime(demoPos);
    if (ui.totalTime) ui.totalTime.textContent = formatTime(DEMO_DURATION - demoPos);
  }

  function syncFx() {
    const fx = window.JB2OverlayFx;
    if (!fx) return;
    const playing =
      previewMode !== "nosignal" &&
      !(previewMode === "paused" || (previewMode === "cycle" && demoPaused));
    fx.set({
      theme: layout.theme,
      pulse: layout.pulse,
      glass: layout.glass,
      playing,
    });
  }

  function applyLook() {
    songInfo.setAttribute("data-theme", layout.theme);
    songInfo.setAttribute("data-seek", layout.seekStyle);
    songInfo.classList.toggle("is-text-only", !layout.glass);
    const speed = Number(layout.pulse) || 0;
    if (speed <= 0 || demoPaused || previewMode === "nosignal") {
      songInfo.classList.remove("is-glowing");
    } else {
      const speedScale = Math.max(0.1, speed / 5);
      songInfo.style.setProperty("--glow-duration", `${(2 / speedScale).toFixed(2)}s`);
      songInfo.classList.add("is-glowing");
    }
    syncFx();
  }

  function applyPreview() {
    const noSignal = previewMode === "nosignal";
    const paused = previewMode === "paused" || (previewMode === "cycle" && demoPaused);
    if (noSignal) {
      if (overlayState) overlayState.textContent = "NO SIGNAL";
      songInfo.classList.add("is-status");
      songInfo.style.transition = "opacity 180ms ease";
      songInfo.style.opacity = String(layout.playingOpacity);
      if (ui.playState) ui.playState.textContent = "⏸";
      songInfo.classList.remove("is-glowing");
      syncFx();
      return;
    }
    if (paused) {
      if (overlayState) overlayState.textContent = "PAUSED";
      songInfo.classList.add("is-status");
      songInfo.style.transition = "opacity 600ms ease";
      songInfo.style.opacity = String(layout.pausedOpacity);
      if (ui.playState) ui.playState.textContent = "⏸";
      songInfo.classList.remove("is-glowing");
      syncFx();
      return;
    }
    songInfo.classList.remove("is-status");
    songInfo.style.transition = "opacity 180ms ease";
    songInfo.style.opacity = String(layout.playingOpacity);
    if (ui.playState) ui.playState.textContent = "⏵";
    applyLook();
  }

  function setPaused(paused) {
    demoPaused = paused;
    applyPreview();
  }

  function startCycle() {
    stopCycle();
    if (previewMode !== "cycle") return;
    cycleTimer = window.setInterval(() => {
      setPaused(!demoPaused);
    }, 8000);
  }

  function stopCycle() {
    if (cycleTimer) {
      clearInterval(cycleTimer);
      cycleTimer = null;
    }
  }

  function setPreviewMode(mode) {
    previewMode = mode;
    previewBtn.textContent = PREVIEW_LABELS[mode] || mode;
    if (mode === "playing") setPaused(false);
    else if (mode === "paused") setPaused(true);
    else if (mode === "nosignal") applyPreview();
    else applyPreview();
    startCycle();
  }

  function panelScale() {
    return (layout.panelW || BASE_W) / BASE_W;
  }

  function panelSize() {
    const w = layout.panelW || BASE_W;
    const h = Math.round((songInfo.offsetHeight || 160) * panelScale());
    return { w, h };
  }

  function gridLines(span) {
    const edges = [0, span];
    switch (layout.grid) {
      case "off":
        return edges;
      case "center":
        return edges.concat([span / 2]);
      case "thirds":
        return edges.concat([span / 3, (2 * span) / 3]);
      case "golden":
        return edges.concat([span * 0.382, span * 0.618]);
      case "safe":
        return edges.concat([span * 0.05, span * 0.1, span * 0.9, span * 0.95]);
      case "quad":
        return edges.concat([span * 0.25, span / 2, span * 0.75]);
      default:
        return edges.concat([span / 3, span / 2, (2 * span) / 3]);
    }
  }

  function snapAxis(origin, size, canvasSpan, lines) {
    const points = [origin, origin + size / 2, origin + size];
    let bestDelta = 0;
    let bestDist = SNAP_PX + 1;
    for (const p of points) {
      for (const line of lines) {
        const d = Math.abs(p - line);
        if (d < bestDist) {
          bestDist = d;
          bestDelta = line - p;
        }
      }
    }
    let next = origin;
    if (bestDist <= SNAP_PX) next += bestDelta;
    return Math.max(0, Math.min(canvasSpan - size, Math.round(next)));
  }

  function snapPanel(x, y) {
    const { w, h } = panelSize();
    return {
      x: snapAxis(x, w, layout.canvasW, gridLines(layout.canvasW)),
      y: snapAxis(y, h, layout.canvasH, gridLines(layout.canvasH)),
    };
  }

  function applyPanelBox() {
    overlayBox.style.left = `${layout.x}px`;
    overlayBox.style.top = `${layout.y}px`;
    songInfo.style.width = `${BASE_W}px`;
    songInfo.style.removeProperty("height");
    const s = panelScale();
    songInfo.style.transformOrigin = "0 0";
    songInfo.style.transform = s === 1 ? "none" : `scale(${s})`;
    overlayBox.style.width = `${layout.panelW}px`;
    overlayBox.style.height = `${Math.round((songInfo.offsetHeight || 160) * s)}px`;
  }

  function syncToolbar() {
    themeSelect.value = layout.theme;
    seekSelect.value = layout.seekStyle;
    playingOpacity.value = String(layout.playingOpacity);
    pausedOpacity.value = String(layout.pausedOpacity);
    pulseInput.value = String(layout.pulse);
    glassInput.checked = layout.glass;
    gridSelect.value = layout.grid;
    canvas.setAttribute("data-grid", layout.grid);
  }

  function fitCanvas() {
    canvas.style.width = `${layout.canvasW}px`;
    canvas.style.height = `${layout.canvasH}px`;
    const host = fit.getBoundingClientRect();
    const pad = 24;
    const scale = Math.min((host.width - pad) / layout.canvasW, (host.height - pad) / layout.canvasH);
    canvas.style.transform = `translate(-50%, -50%) scale(${Math.max(0.05, scale)})`;
    btn720.classList.toggle("is-active", layout.canvasW === 1280);
    btn1080.classList.toggle("is-active", layout.canvasW === 1920);
    btn720.setAttribute("aria-pressed", layout.canvasW === 1280 ? "true" : "false");
    btn1080.setAttribute("aria-pressed", layout.canvasW === 1920 ? "true" : "false");
  }

  function pointerToCanvas(ev) {
    const r = canvas.getBoundingClientRect();
    if (r.width <= 0 || r.height <= 0) return { x: 0, y: 0 };
    return {
      x: ((ev.clientX - r.left) / r.width) * layout.canvasW,
      y: ((ev.clientY - r.top) / r.height) * layout.canvasH,
    };
  }

  function layoutBody(save) {
    return {
      x: layout.x,
      y: layout.y,
      canvasW: layout.canvasW,
      canvasH: layout.canvasH,
      theme: layout.theme,
      seekStyle: layout.seekStyle,
      playingOpacity: layout.playingOpacity,
      pausedOpacity: layout.pausedOpacity,
      panelW: layout.panelW,
      panelH: layout.panelH,
      pulse: layout.pulse,
      glass: layout.glass,
      grid: layout.grid,
      save: Boolean(save),
    };
  }

  function putLayout(save) {
    return fetch("/api/overlay", {
      method: "PUT",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(layoutBody(save)),
    }).catch(() => {});
  }

  function scheduleLivePut() {
    if (putTimer) return;
    putTimer = window.setTimeout(() => {
      putTimer = null;
      putLayout(false);
    }, PUT_MS);
  }

  function saveNow() {
    if (putTimer) {
      clearTimeout(putTimer);
      putTimer = null;
    }
    putLayout(true);
  }

  function clampBox() {
    layout.panelH = 0;
    layout.panelW = Math.max(MIN_W, Math.min(layout.canvasW, Math.round(layout.panelW)));
    applyPanelBox();
    const { w, h } = panelSize();
    layout.x = Math.max(0, Math.min(layout.canvasW - w, Math.round(layout.x)));
    layout.y = Math.max(0, Math.min(layout.canvasH - h, Math.round(layout.y)));
  }

  function setCanvasSize(w, h) {
    if (w === layout.canvasW && h === layout.canvasH) return;
    layout.x = Math.round((layout.x * w) / layout.canvasW);
    layout.y = Math.round((layout.y * h) / layout.canvasH);
    layout.canvasW = w;
    layout.canvasH = h;
    clampBox();
    const snapped = snapPanel(layout.x, layout.y);
    layout.x = snapped.x;
    layout.y = snapped.y;
    applyPanelBox();
    fitCanvas();
    saveNow();
  }

  function handleOf(el) {
    const node = el && el.closest ? el.closest("[data-handle]") : null;
    return node ? node.getAttribute("data-handle") : "";
  }

  overlayBox.addEventListener("pointerdown", (ev) => {
    if (ev.button !== 0) return;
    ev.preventDefault();
    overlayBox.setPointerCapture(ev.pointerId);
    const p = pointerToCanvas(ev);
    const handle = handleOf(ev.target);
    const { w, h } = panelSize();
    if (handle) {
      drag = {
        kind: "resize",
        handle,
        x: layout.x,
        y: layout.y,
        w,
        h,
        px: p.x,
        py: p.y,
      };
    } else {
      drag = { kind: "move", dx: p.x - layout.x, dy: p.y - layout.y };
    }
    overlayBox.classList.add("is-dragging");
  });

  overlayBox.addEventListener("pointermove", (ev) => {
    if (!drag) return;
    const p = pointerToCanvas(ev);
    if (drag.kind === "move") {
      const snapped = snapPanel(p.x - drag.dx, p.y - drag.dy);
      if (snapped.x === layout.x && snapped.y === layout.y) return;
      layout.x = snapped.x;
      layout.y = snapped.y;
      applyPanelBox();
      scheduleLivePut();
      return;
    }
    const ratio = drag.w / Math.max(1, drag.h);
    const handle = drag.handle;
    const fromW = handle.includes("e") || handle.includes("w");
    const fromH = handle.includes("n") || handle.includes("s");
    const rawW = handle.includes("w") ? drag.x + drag.w - p.x : handle.includes("e") ? p.x - drag.x : drag.w;
    const rawH = handle.includes("n") ? drag.y + drag.h - p.y : handle.includes("s") ? p.y - drag.y : drag.h;
    let w;
    if (fromW && fromH) {
      if (Math.abs(rawW / drag.w) >= Math.abs(rawH / drag.h)) w = Math.abs(rawW);
      else w = Math.abs(rawH) * ratio;
    } else if (fromW) {
      w = Math.abs(rawW);
    } else {
      w = Math.abs(rawH) * ratio;
    }
    w = Math.max(MIN_W, Math.min(layout.canvasW, w));
    let h = w / ratio;
    if (h > layout.canvasH) {
      h = layout.canvasH;
      w = h * ratio;
    }
    w = Math.max(MIN_W, w);
    h = w / ratio;
    let x = handle.includes("w") ? drag.x + drag.w - w : drag.x;
    let y = handle.includes("n") ? drag.y + drag.h - h : drag.y;
    x = Math.max(0, Math.min(layout.canvasW - w, x));
    y = Math.max(0, Math.min(layout.canvasH - h, y));
    layout.x = Math.round(x);
    layout.y = Math.round(y);
    layout.panelW = Math.round(w);
    layout.panelH = 0;
    applyPanelBox();
    scheduleLivePut();
  });

  function endDrag() {
    if (!drag) return;
    drag = null;
    overlayBox.classList.remove("is-dragging");
    saveNow();
    window.requestAnimationFrame(() => {
      updateTextMarquee(ui.title, ui.titleWrap);
      updateTextMarquee(ui.artist, ui.artistWrap);
      updateTextMarquee(ui.album, ui.albumWrap);
      updateMarker();
    });
  }

  overlayBox.addEventListener("pointerup", endDrag);
  overlayBox.addEventListener("pointercancel", endDrag);

  btn720.addEventListener("click", () => setCanvasSize(1280, 720));
  btn1080.addEventListener("click", () => setCanvasSize(1920, 1080));
  themeSelect.addEventListener("change", () => {
    layout.theme = themeSelect.value;
    applyLook();
    applyPreview();
    applyPanelBox();
    saveNow();
  });
  seekSelect.addEventListener("change", () => {
    layout.seekStyle = seekSelect.value;
    applyLook();
    saveNow();
  });
  playingOpacity.addEventListener("input", () => {
    layout.playingOpacity = Number(playingOpacity.value);
    applyPreview();
    scheduleLivePut();
  });
  playingOpacity.addEventListener("change", saveNow);
  pausedOpacity.addEventListener("input", () => {
    layout.pausedOpacity = Number(pausedOpacity.value);
    applyPreview();
    scheduleLivePut();
  });
  pausedOpacity.addEventListener("change", saveNow);
  pulseInput.addEventListener("input", () => {
    layout.pulse = Number(pulseInput.value);
    applyLook();
    applyPreview();
    scheduleLivePut();
  });
  pulseInput.addEventListener("change", saveNow);
  glassInput.addEventListener("change", () => {
    layout.glass = glassInput.checked;
    applyLook();
    saveNow();
  });
  gridSelect.addEventListener("change", () => {
    layout.grid = gridSelect.value;
    canvas.setAttribute("data-grid", layout.grid);
    saveNow();
  });
  previewBtn.addEventListener("click", () => {
    const i = PREVIEW_MODES.indexOf(previewMode);
    setPreviewMode(PREVIEW_MODES[(i + 1) % PREVIEW_MODES.length]);
  });
  document.querySelectorAll("[data-reset]").forEach((btn) => {
    btn.addEventListener("click", () => {
      const key = btn.getAttribute("data-reset");
      if (key === "size") {
        layout.panelW = DEFAULTS.panelW;
        layout.panelH = DEFAULTS.panelH;
        clampBox();
        applyPanelBox();
      } else if (Object.prototype.hasOwnProperty.call(DEFAULTS, key)) {
        layout[key] = DEFAULTS[key];
        syncToolbar();
        applyLook();
        applyPreview();
      }
      saveNow();
      window.requestAnimationFrame(() => {
        updateTextMarquee(ui.title, ui.titleWrap);
        updateTextMarquee(ui.artist, ui.artistWrap);
        updateTextMarquee(ui.album, ui.albumWrap);
        updateMarker();
      });
    });
  });
  window.addEventListener("resize", fitCanvas);

  function readLayout(data) {
    if (!data) return;
    layout.x = Number(data.x) || 0;
    layout.y = Number(data.y) || 0;
    layout.canvasW = Number(data.canvasW) || 1920;
    layout.canvasH = Number(data.canvasH) || 1080;
    if (data.theme) layout.theme = data.theme;
    if (data.seekStyle) layout.seekStyle = data.seekStyle;
    if (data.playingOpacity != null) layout.playingOpacity = Number(data.playingOpacity);
    if (data.pausedOpacity != null) layout.pausedOpacity = Number(data.pausedOpacity);
    if (data.panelW) layout.panelW = Number(data.panelW);
    if (data.panelH != null) layout.panelH = Number(data.panelH) || 0;
    if (data.pulse != null) layout.pulse = Number(data.pulse);
    if (typeof data.glass === "boolean") layout.glass = data.glass;
    if (data.grid) layout.grid = data.grid;
  }

  if (window.JB2OverlayFx) {
    window.JB2OverlayFx.init(overlayBox);
  }

  applyLook();
  setPreviewMode("cycle");
  setPaused(false);

  window.requestAnimationFrame(() => {
    updateTextMarquee(ui.title, ui.titleWrap);
    updateTextMarquee(ui.artist, ui.artistWrap);
    updateTextMarquee(ui.album, ui.albumWrap);
    updateMarker();
  });

  window.setInterval(() => {
    if (previewMode === "paused" || previewMode === "nosignal") return;
    if (previewMode === "cycle" && demoPaused) return;
    demoPos += 1;
    if (demoPos > DEMO_DURATION) demoPos = 0;
    updateMarker();
  }, 1000);

  fetch("/api/overlay")
    .then((res) => res.json())
    .then((env) => {
      if (!env || env.exitCode !== 0 || !env.data) return;
      readLayout(env.data);
    })
    .catch(() => {})
    .finally(() => {
      syncToolbar();
      applyLook();
      applyPanelBox();
      applyPreview();
      fitCanvas();
    });
})();

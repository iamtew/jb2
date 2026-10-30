(function () {
  const SNAP_PX = 12;
  const PUT_MS = 100;
  const MIN_W = 280;
  const MIN_VIZ = 80;
  const BASE_W = 500;

  const canvas = document.getElementById("stageCanvas");
  const fit = document.getElementById("stageFit");
  const overlayBox = document.getElementById("overlayBox");
  const vizBox = document.getElementById("vizBox");
  const layerList = document.getElementById("layerList");
  const songInfo = document.getElementById("songInfo");
  const btn720 = document.getElementById("size720");
  const btn1080 = document.getElementById("size1080");
  const themeSelect = document.getElementById("themeSelect");
  const seekSelect = document.getElementById("seekSelect");
  const playingOpacity = document.getElementById("playingOpacity");
  const pausedOpacity = document.getElementById("pausedOpacity");
  const pulseInput = document.getElementById("pulseInput");
  const glassInput = document.getElementById("glassInput");
  const gridChecks = document.querySelectorAll('input[name="grid"]');
  const gridSvg = document.getElementById("stageGridLines");
  const copyOverlayUrl = document.getElementById("copyOverlayUrl");

  const DEFAULTS = {
    theme: "lcd-glass",
    seekStyle: "sesh-sofa",
    playingOpacity: 1,
    pausedOpacity: 0.25,
    panelW: 500,
    panelH: 0,
    pulse: 3,
    glass: true,
    grid: "center,thirds",
    vizX: 0,
    vizY: 800,
    vizW: 1920,
    vizH: 280,
  };
  const layoutLayers = ["overlay", "viz"];
  const GRID_IDS = ["center", "golden", "quad", "safe", "thirds"];
  const GRID_CYAN = "rgba(0, 255, 255, 0.45)";
  const GRID_ORANGE = "rgba(255, 64, 0, 0.55)";
  const layout = {
    x: 0,
    y: 0,
    canvasW: 1920,
    canvasH: 1080,
    layers: layoutLayers.slice(),
    ...DEFAULTS,
  };
  let drag = null;
  let putTimer = null;
  let canvasScale = 1;

  function refreshNow() {
    if (window.JB2NowPlaying) window.JB2NowPlaying.refresh();
  }

  function applyLook() {
    songInfo.setAttribute("data-theme", layout.theme);
    songInfo.setAttribute("data-seek", layout.seekStyle);
    songInfo.classList.toggle("is-text-only", !layout.glass);
    refreshNow();
  }

  function panelScale() {
    return (layout.panelW || BASE_W) / BASE_W;
  }

  function panelSize() {
    const w = layout.panelW || BASE_W;
    const h = Math.round((songInfo.offsetHeight || 160) * panelScale());
    return { w, h };
  }

  function parseGrid(value) {
    const raw = String(value || "").trim();
    if (!raw || raw === "off") return [];
    const text = raw === "thirds-center" ? "center,thirds" : raw;
    const seen = new Set();
    const ids = [];
    text.split(",").forEach((part) => {
      const id = part.trim();
      if (GRID_IDS.includes(id) && !seen.has(id)) {
        seen.add(id);
        ids.push(id);
      }
    });
    ids.sort();
    return ids;
  }

  function formatGrid(ids) {
    const unique = parseGrid(ids.join(","));
    return unique.length ? unique.join(",") : "off";
  }

  function gridFractions(id) {
    switch (id) {
      case "center":
        return [0.5];
      case "thirds":
        return [1 / 3, 2 / 3];
      case "golden":
        return [0.382, 0.618];
      case "quad":
        return [0.25, 0.5, 0.75];
      case "safe":
        return [0.05, 0.1, 0.9, 0.95];
      default:
        return [];
    }
  }

  function gridLines(span) {
    const ids = parseGrid(layout.grid);
    const set = new Set([0, span]);
    ids.forEach((id) => {
      gridFractions(id).forEach((f) => set.add(span * f));
    });
    return Array.from(set);
  }

  function paintGrid() {
    const w = layout.canvasW;
    const h = layout.canvasH;
    const stroke = Math.max(2, Math.round(1 / canvasScale));
    const ids = parseGrid(layout.grid);
    gridSvg.setAttribute("viewBox", `0 0 ${w} ${h}`);
    const marks = [];
    function addCross(fracs, color) {
      fracs.forEach((f) => {
        const x = w * f;
        const y = h * f;
        marks.push(
          `<line x1="${x}" y1="0" x2="${x}" y2="${h}" stroke="${color}" stroke-width="${stroke}"/>`,
          `<line x1="0" y1="${y}" x2="${w}" y2="${y}" stroke="${color}" stroke-width="${stroke}"/>`
        );
      });
    }
    ids.forEach((id) => {
      if (id === "safe") {
        marks.push(
          `<rect x="${w * 0.05}" y="${h * 0.05}" width="${w * 0.9}" height="${h * 0.9}" fill="none" stroke="${GRID_ORANGE}" stroke-width="${stroke}"/>`,
          `<rect x="${w * 0.1}" y="${h * 0.1}" width="${w * 0.8}" height="${h * 0.8}" fill="none" stroke="${GRID_CYAN}" stroke-width="${stroke}"/>`
        );
        return;
      }
      const fracs = gridFractions(id);
      if (id === "center" || id === "quad") {
        addCross(fracs.filter((f) => f === 0.5), GRID_ORANGE);
        addCross(fracs.filter((f) => f !== 0.5), GRID_CYAN);
      } else {
        addCross(fracs, GRID_CYAN);
      }
    });
    gridSvg.innerHTML = marks.join("");
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
    applyStack();
  }

  function applyStack() {
    const front = layout.layers[0] === "viz" ? "viz" : "overlay";
    overlayBox.style.zIndex = front === "overlay" ? "3" : "2";
    if (vizBox) vizBox.style.zIndex = front === "viz" ? "3" : "2";
  }

  function applyVizBox() {
    if (!vizBox) return;
    vizBox.style.left = `${layout.vizX}px`;
    vizBox.style.top = `${layout.vizY}px`;
    vizBox.style.width = `${layout.vizW}px`;
    vizBox.style.height = `${layout.vizH}px`;
    applyStack();
  }

  function clampViz() {
    layout.vizW = Math.max(MIN_VIZ, Math.min(layout.canvasW, Math.round(layout.vizW)));
    layout.vizH = Math.max(MIN_VIZ, Math.min(layout.canvasH, Math.round(layout.vizH)));
    layout.vizX = Math.max(0, Math.min(layout.canvasW - layout.vizW, Math.round(layout.vizX)));
    layout.vizY = Math.max(0, Math.min(layout.canvasH - layout.vizH, Math.round(layout.vizY)));
  }

  function snapViz(x, y) {
    return {
      x: snapAxis(x, layout.vizW, layout.canvasW, gridLines(layout.canvasW)),
      y: snapAxis(y, layout.vizH, layout.canvasH, gridLines(layout.canvasH)),
    };
  }

  function parseLayers(value) {
    const ids = [];
    String(value || "")
      .split(",")
      .forEach((part) => {
        const id = part.trim();
        if ((id === "overlay" || id === "viz") && !ids.includes(id)) ids.push(id);
      });
    ["overlay", "viz"].forEach((id) => {
      if (!ids.includes(id)) ids.push(id);
    });
    return ids;
  }

  function paintLayers() {
    if (!layerList) return;
    layerList.replaceChildren();
    layout.layers.forEach((id) => {
      const li = document.createElement("li");
      li.draggable = true;
      li.dataset.layer = id;
      li.textContent = id === "viz" ? "Visualizer" : "Overlay";
      layerList.appendChild(li);
    });
  }

  function syncToolbar() {
    themeSelect.value = layout.theme;
    seekSelect.value = layout.seekStyle;
    playingOpacity.value = String(layout.playingOpacity);
    pausedOpacity.value = String(layout.pausedOpacity);
    pulseInput.value = String(layout.pulse);
    glassInput.checked = layout.glass;
    const selected = new Set(parseGrid(layout.grid));
    gridChecks.forEach((el) => {
      el.checked = selected.has(el.value);
    });
    paintGrid();
    paintLayers();
  }

  function fitCanvas() {
    canvas.style.width = `${layout.canvasW}px`;
    canvas.style.height = `${layout.canvasH}px`;
    const host = fit.getBoundingClientRect();
    const pad = 24;
    const scale = Math.min((host.width - pad) / layout.canvasW, (host.height - pad) / layout.canvasH);
    canvasScale = Math.max(0.05, scale);
    canvas.style.transform = `translate(-50%, -50%) scale(${canvasScale})`;
    paintGrid();
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
      vizX: layout.vizX,
      vizY: layout.vizY,
      vizW: layout.vizW,
      vizH: layout.vizH,
      layers: layout.layers.join(","),
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
    layout.vizX = Math.round((layout.vizX * w) / layout.canvasW);
    layout.vizY = Math.round((layout.vizY * h) / layout.canvasH);
    layout.vizW = Math.round((layout.vizW * w) / layout.canvasW);
    layout.vizH = Math.round((layout.vizH * h) / layout.canvasH);
    layout.canvasW = w;
    layout.canvasH = h;
    clampBox();
    clampViz();
    const snapped = snapPanel(layout.x, layout.y);
    layout.x = snapped.x;
    layout.y = snapped.y;
    applyPanelBox();
    applyVizBox();
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
    if (vizBox) vizBox.classList.remove("is-dragging");
    saveNow();
    refreshNow();
  }

  overlayBox.addEventListener("pointerup", endDrag);
  overlayBox.addEventListener("pointercancel", endDrag);

  if (vizBox) {
    vizBox.addEventListener("pointerdown", (ev) => {
      if (ev.button !== 0) return;
      ev.preventDefault();
      vizBox.setPointerCapture(ev.pointerId);
      const p = pointerToCanvas(ev);
      const handle = handleOf(ev.target);
      if (handle) {
        drag = {
          kind: "viz-resize",
          handle,
          x: layout.vizX,
          y: layout.vizY,
          w: layout.vizW,
          h: layout.vizH,
          px: p.x,
          py: p.y,
        };
      } else {
        drag = { kind: "viz-move", dx: p.x - layout.vizX, dy: p.y - layout.vizY };
      }
      vizBox.classList.add("is-dragging");
    });
    vizBox.addEventListener("pointermove", (ev) => {
      if (!drag || (drag.kind !== "viz-move" && drag.kind !== "viz-resize")) return;
      const p = pointerToCanvas(ev);
      if (drag.kind === "viz-move") {
        const snapped = snapViz(p.x - drag.dx, p.y - drag.dy);
        if (snapped.x === layout.vizX && snapped.y === layout.vizY) return;
        layout.vizX = snapped.x;
        layout.vizY = snapped.y;
        applyVizBox();
        scheduleLivePut();
        return;
      }
      const handle = drag.handle;
      let w = drag.w;
      let h = drag.h;
      let x = drag.x;
      let y = drag.y;
      const dx = p.x - drag.px;
      const dy = p.y - drag.py;
      if (handle.includes("e")) w = drag.w + dx;
      if (handle.includes("w")) w = drag.w - dx;
      if (handle.includes("s")) h = drag.h + dy;
      if (handle.includes("n")) h = drag.h - dy;
      w = Math.max(MIN_VIZ, Math.min(layout.canvasW, w));
      h = Math.max(MIN_VIZ, Math.min(layout.canvasH, h));
      if (handle.includes("w")) x = drag.x + drag.w - w;
      if (handle.includes("n")) y = drag.y + drag.h - h;
      x = Math.max(0, Math.min(layout.canvasW - w, x));
      y = Math.max(0, Math.min(layout.canvasH - h, y));
      layout.vizX = Math.round(x);
      layout.vizY = Math.round(y);
      layout.vizW = Math.round(w);
      layout.vizH = Math.round(h);
      applyVizBox();
      scheduleLivePut();
    });
    vizBox.addEventListener("pointerup", endDrag);
    vizBox.addEventListener("pointercancel", endDrag);
  }

  if (layerList) {
    layerList.addEventListener("dragstart", (ev) => {
      const li = ev.target.closest("li");
      if (!li) return;
      ev.dataTransfer.setData("text/plain", li.dataset.layer || "");
      ev.dataTransfer.effectAllowed = "move";
    });
    layerList.addEventListener("dragover", (ev) => {
      const li = ev.target.closest("li");
      if (!li) return;
      ev.preventDefault();
      layerList.querySelectorAll("li").forEach((el) => el.classList.toggle("is-over", el === li));
    });
    layerList.addEventListener("dragleave", (ev) => {
      const li = ev.target.closest("li");
      if (li) li.classList.remove("is-over");
    });
    layerList.addEventListener("drop", (ev) => {
      ev.preventDefault();
      const li = ev.target.closest("li");
      const from = ev.dataTransfer.getData("text/plain");
      const to = li && li.dataset.layer;
      layerList.querySelectorAll("li").forEach((el) => el.classList.remove("is-over"));
      if (!from || !to || from === to) return;
      const order = layout.layers.filter((id) => id !== from);
      order.splice(Math.max(0, order.indexOf(to)), 0, from);
      layout.layers = parseLayers(order.join(","));
      paintLayers();
      applyStack();
      saveNow();
    });
  }

  document.querySelectorAll("[data-stage-tab]").forEach((btn) => {
    btn.addEventListener("click", () => {
      const id = btn.getAttribute("data-stage-tab");
      document.querySelectorAll("[data-stage-tab]").forEach((el) => {
        const on = el === btn;
        el.classList.toggle("is-active", on);
        el.setAttribute("aria-selected", on ? "true" : "false");
      });
      document.querySelectorAll("[data-stage-panel]").forEach((panel) => {
        panel.hidden = panel.getAttribute("data-stage-panel") !== id;
      });
    });
  });

  btn720.addEventListener("click", () => setCanvasSize(1280, 720));
  btn1080.addEventListener("click", () => setCanvasSize(1920, 1080));
  themeSelect.addEventListener("change", () => {
    layout.theme = themeSelect.value;
    applyLook();
    saveNow();
  });
  seekSelect.addEventListener("change", () => {
    layout.seekStyle = seekSelect.value;
    applyLook();
    saveNow();
  });
  playingOpacity.addEventListener("input", () => {
    layout.playingOpacity = Number(playingOpacity.value);
    refreshNow();
    scheduleLivePut();
  });
  playingOpacity.addEventListener("change", saveNow);
  pausedOpacity.addEventListener("input", () => {
    layout.pausedOpacity = Number(pausedOpacity.value);
    refreshNow();
    scheduleLivePut();
  });
  pausedOpacity.addEventListener("change", saveNow);
  pulseInput.addEventListener("input", () => {
    layout.pulse = Number(pulseInput.value);
    applyLook();
    scheduleLivePut();
  });
  pulseInput.addEventListener("change", saveNow);
  glassInput.addEventListener("change", () => {
    layout.glass = glassInput.checked;
    applyLook();
    saveNow();
  });
  gridChecks.forEach((el) => {
    el.addEventListener("change", () => {
      const ids = [];
      gridChecks.forEach((box) => {
        if (box.checked) ids.push(box.value);
      });
      layout.grid = formatGrid(ids);
      paintGrid();
      saveNow();
    });
  });
  // execCommand in the click itself. Off-screen nodes often return false;
  // writeText after await loses the user gesture.
  function copyNow(text) {
    const el = document.createElement("textarea");
    el.value = text;
    el.setAttribute("readonly", "");
    el.style.cssText = "position:fixed;top:0;left:0;width:1px;height:1px;opacity:0";
    document.body.appendChild(el);
    el.focus();
    el.select();
    el.setSelectionRange(0, text.length);
    let ok = false;
    try {
      ok = document.execCommand("copy");
    } catch (_) {
      /* fall through */
    }
    el.remove();
    return ok;
  }
  copyOverlayUrl.addEventListener("click", () => {
    const text = `${window.location.origin}/overlay/`;
    const icon = copyOverlayUrl.querySelector("use");
    function flash(ok) {
      if (icon) icon.setAttribute("href", ok ? "#check" : "#copy");
      copyOverlayUrl.setAttribute("aria-label", ok ? "Copied" : "Copy failed");
      window.setTimeout(() => {
        if (icon) icon.setAttribute("href", "#copy");
        copyOverlayUrl.setAttribute("aria-label", "Copy Overlay URL");
      }, 1200);
    }
    if (copyNow(text)) {
      flash(true);
      return;
    }
    window.focus();
    if (navigator.clipboard?.writeText) {
      navigator.clipboard.writeText(text).then(
        () => flash(true),
        () => flash(false)
      );
      return;
    }
    flash(false);
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
      }
      saveNow();
      refreshNow();
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
    if (data.grid) layout.grid = formatGrid(parseGrid(data.grid));
    if (data.vizX != null) layout.vizX = Number(data.vizX) || 0;
    if (data.vizY != null) layout.vizY = Number(data.vizY) || 0;
    if (data.vizW != null) layout.vizW = Number(data.vizW) || MIN_VIZ;
    if (data.vizH != null) layout.vizH = Number(data.vizH) || MIN_VIZ;
    if (data.layers) layout.layers = parseLayers(data.layers);
  }

  if (window.JB2OverlayFx) {
    window.JB2OverlayFx.init(overlayBox);
  }

  applyLook();
  applyVizBox();
  paintLayers();

  if (window.JB2NowPlaying) {
    window.JB2NowPlaying.start({
      look: layout,
      onChange: function () {
        applyPanelBox();
      },
    });
  }

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
      applyVizBox();
      refreshNow();
      fitCanvas();
    });
})();

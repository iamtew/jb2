(function () {
  const SNAP_PX = 12;
  const PUT_MS = 100;
  const MIN_W = 280;
  const MIN_VIZ = 80;
  const BASE_W = 500;
  const VIZ_MAX = 8;

  const canvas = document.getElementById("stageCanvas");
  const fit = document.getElementById("stageFit");
  const overlayBox = document.getElementById("overlayBox");
  const vizStack = document.getElementById("vizStack");
  const layerList = document.getElementById("layerList");
  const songInfo = document.getElementById("songInfo");
  const btn720 = document.getElementById("size720");
  const btn1080 = document.getElementById("size1080");
  const themeSelect = document.getElementById("themeSelect");
  const seekSelect = document.getElementById("seekSelect");
  const seekDirSelect = document.getElementById("seekDirSelect");
  const playingOpacity = document.getElementById("playingOpacity");
  const pausedOpacity = document.getElementById("pausedOpacity");
  const pulseInput = document.getElementById("pulseInput");
  const glassInput = document.getElementById("glassInput");
  const gridChecks = document.querySelectorAll('input[name="grid"]');
  const snapInput = document.getElementById("snapInput");
  const gridSvg = document.getElementById("stageGridLines");
  const copyOverlayUrl = document.getElementById("copyOverlayUrl");
  const addViz = document.getElementById("addViz");
  const overlayLook = document.getElementById("overlayLook");

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

  const DEFAULTS = {
    theme: "lcd-glass",
    seekStyle: "sesh-sofa",
    seekDir: "lr",
    playingOpacity: 1,
    pausedOpacity: 0.25,
    panelW: 500,
    panelH: 0,
    pulse: 3,
    glass: true,
    grid: "center,thirds",
    snap: true,
  };
  const GRID_IDS = ["center", "golden", "quad", "safe", "thirds"];
  const GRID_CYAN = "rgba(0, 255, 255, 0.45)";
  const GRID_ORANGE = "rgba(255, 64, 0, 0.55)";

  function defaultVizLayer(id, src) {
    const base = src || {};
    return {
      id,
      x: Number.isFinite(Number(base.x)) ? Number(base.x) : 0,
      y: Number.isFinite(Number(base.y)) ? Number(base.y) : 800,
      w: Number.isFinite(Number(base.w)) ? Number(base.w) : 1920,
      h: Number.isFinite(Number(base.h)) ? Number(base.h) : 280,
      hidden: base.hidden !== false,
      style: base.style || "bands",
      color: typeof base.color === "string" ? base.color : "",
      colorDir: base.colorDir || "lr",
      bands: Number.isFinite(Number(base.bands)) ? Number(base.bands) : 16,
      mirror: Boolean(base.mirror),
      peakFall: Number.isFinite(Number(base.peakFall)) ? Number(base.peakFall) : 0.35,
    };
  }

  const layout = {
    x: 0,
    y: 0,
    canvasW: 1920,
    canvasH: 1080,
    layers: ["overlay", "viz"],
    overlayHidden: false,
    vizLayers: [defaultVizLayer("viz")],
    ...DEFAULTS,
  };
  let drag = null;
  let putTimer = null;
  let canvasScale = 1;
  let editTarget = "overlay";
  let vizSeq = 1;

  function refreshNow() {
    if (window.JB2NowPlaying) window.JB2NowPlaying.refresh();
  }

  function seekDirOf(value) {
    return value === "rl" || value === "tb" || value === "bt" ? value : "lr";
  }

  function isVizId(id) {
    return id === "viz" || (typeof id === "string" && id.startsWith("viz-"));
  }

  function vizLayer(id) {
    return layout.vizLayers.find((layer) => layer.id === id) || null;
  }

  function applyLook() {
    songInfo.setAttribute("data-theme", layout.theme);
    songInfo.setAttribute("data-seek", layout.seekStyle);
    songInfo.setAttribute("data-seek-dir", seekDirOf(layout.seekDir));
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

  function siblingLines(span, origin, size) {
    return [origin, origin + size / 2, origin + size].filter((n) => n >= 0 && n <= span);
  }

  function snapBox(x, y, w, h, others) {
    const clampX = Math.max(0, Math.min(layout.canvasW - w, Math.round(x)));
    const clampY = Math.max(0, Math.min(layout.canvasH - h, Math.round(y)));
    if (!layout.snap) return { x: clampX, y: clampY };
    let xLines = gridLines(layout.canvasW);
    let yLines = gridLines(layout.canvasH);
    (others || []).forEach((box) => {
      xLines = xLines.concat(siblingLines(layout.canvasW, box.x, box.w));
      yLines = yLines.concat(siblingLines(layout.canvasH, box.y, box.h));
    });
    return {
      x: snapAxis(x, w, layout.canvasW, xLines),
      y: snapAxis(y, h, layout.canvasH, yLines),
    };
  }

  function overlayRect() {
    const { w, h } = panelSize();
    return { x: layout.x, y: layout.y, w, h };
  }

  function snapTargets(skipId) {
    const boxes = [overlayRect()];
    layout.vizLayers.forEach((layer) => {
      if (layer.id === skipId) return;
      boxes.push({ x: layer.x, y: layer.y, w: layer.w, h: layer.h });
    });
    return boxes;
  }

  function snapPanel(x, y) {
    const { w, h } = panelSize();
    return snapBox(x, y, w, h, snapTargets());
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
    overlayBox.classList.toggle("is-ghost", layout.overlayHidden);
    applyStack();
  }

  function applyStack() {
    const depth = layout.layers.length;
    layout.layers.forEach((id, i) => {
      const z = String(depth - i + 2);
      if (id === "overlay") {
        overlayBox.style.zIndex = z;
        return;
      }
      const box = vizStack && vizStack.querySelector(`[data-viz-id="${id}"]`);
      if (box) box.style.zIndex = z;
    });
  }

  function vizBoxHtml() {
    const box = document.createElement("div");
    box.className = "viz-box";
    box.innerHTML =
      '<div class="viz-layer" aria-hidden="true"></div>' +
      '<div class="overlay-handles" aria-hidden="true">' +
      '<span class="overlay-handle" data-handle="n"></span>' +
      '<span class="overlay-handle" data-handle="s"></span>' +
      '<span class="overlay-handle" data-handle="e"></span>' +
      '<span class="overlay-handle" data-handle="w"></span>' +
      '<span class="overlay-handle" data-handle="ne"></span>' +
      '<span class="overlay-handle" data-handle="nw"></span>' +
      '<span class="overlay-handle" data-handle="se"></span>' +
      '<span class="overlay-handle" data-handle="sw"></span>' +
      "</div>";
    return box;
  }

  function applyVizBoxes() {
    if (!vizStack) return;
    const keep = new Set(layout.vizLayers.map((layer) => layer.id));
    layout.vizLayers.forEach((layer) => {
      let box = vizStack.querySelector(`[data-viz-id="${layer.id}"]`);
      if (!box) {
        box = vizBoxHtml();
        box.dataset.vizId = layer.id;
        vizStack.appendChild(box);
      }
      box.style.left = `${layer.x}px`;
      box.style.top = `${layer.y}px`;
      box.style.width = `${layer.w}px`;
      box.style.height = `${layer.h}px`;
      box.classList.toggle("is-ghost", layer.hidden);
      box.classList.toggle("is-editing", editTarget === layer.id);
    });
    vizStack.querySelectorAll("[data-viz-id]").forEach((box) => {
      if (!keep.has(box.dataset.vizId)) box.remove();
    });
    applyStack();
    if (window.JB2Viz) {
      window.JB2Viz.sync({ vizLayers: layout.vizLayers });
      window.JB2Viz.select(isVizId(editTarget) ? editTarget : "");
    }
  }

  function clampVizLayer(layer) {
    layer.w = Math.max(MIN_VIZ, Math.min(layout.canvasW, Math.round(layer.w)));
    layer.h = Math.max(MIN_VIZ, Math.min(layout.canvasH, Math.round(layer.h)));
    layer.x = Math.max(0, Math.min(layout.canvasW - layer.w, Math.round(layer.x)));
    layer.y = Math.max(0, Math.min(layout.canvasH - layer.h, Math.round(layer.y)));
  }

  function clampViz() {
    layout.vizLayers.forEach(clampVizLayer);
  }

  function lockInspectors() {
    if (overlayLook) overlayLook.disabled = layout.overlayHidden;
    const vizLook = document.getElementById("vizLook");
    const layer = vizLayer(editTarget);
    if (vizLook) vizLook.disabled = !layer || layer.hidden;
  }

  function applyEditTarget() {
    overlayBox.classList.toggle("is-editing", editTarget === "overlay");
    if (vizStack) {
      vizStack.querySelectorAll("[data-viz-id]").forEach((box) => {
        box.classList.toggle("is-editing", box.dataset.vizId === editTarget);
      });
    }
    lockInspectors();
  }

  function setEditTarget(id) {
    editTarget = id === "overlay" || isVizId(id) ? id : "overlay";
    document.querySelectorAll("[data-stage-panel]").forEach((panel) => {
      const kind = panel.getAttribute("data-stage-panel");
      panel.hidden = kind === "viz" ? !isVizId(editTarget) : editTarget !== "overlay";
    });
    paintLayers();
    applyEditTarget();
    if (window.JB2Viz) window.JB2Viz.select(isVizId(editTarget) ? editTarget : "");
  }

  function parseLayers(value, vizIds) {
    const allowed = new Set(["overlay"].concat(vizIds));
    const ids = [];
    String(value || "")
      .split(",")
      .forEach((part) => {
        const id = part.trim();
        if (allowed.has(id) && !ids.includes(id)) ids.push(id);
      });
    if (!ids.includes("overlay")) ids.push("overlay");
    vizIds.forEach((id) => {
      if (!ids.includes(id)) ids.push(id);
    });
    return ids;
  }

  function layerName(id) {
    if (id === "overlay") return "Overlay";
    const i = layout.vizLayers.findIndex((layer) => layer.id === id);
    return layout.vizLayers.length === 1 ? "Visualizer" : `Visualizer ${i + 1}`;
  }

  function paintLayers() {
    if (!layerList) return;
    layerList.replaceChildren();
    const vizCount = layout.vizLayers.length;
    layout.layers.forEach((id) => {
      const li = document.createElement("li");
      li.dataset.layer = id;
      li.classList.toggle("is-active", id === editTarget);
      li.classList.toggle("is-hidden", id === "overlay" ? layout.overlayHidden : Boolean(vizLayer(id)?.hidden));
      li.classList.toggle("is-viz", isVizId(id));
      const grip = document.createElement("span");
      grip.className = "stage-layer-grip";
      grip.draggable = true;
      grip.title = "Drag to reorder";
      grip.innerHTML = '<svg class="icon" aria-hidden="true"><use href="#grip-vertical"></use></svg>';
      const name = document.createElement("span");
      name.className = "stage-layer-name";
      name.textContent = layerName(id);
      const eye = document.createElement("button");
      eye.type = "button";
      eye.className = "stage-icon-btn";
      const hidden = id === "overlay" ? layout.overlayHidden : Boolean(vizLayer(id)?.hidden);
      eye.dataset.eye = id;
      eye.setAttribute("aria-label", hidden ? "Show layer" : "Hide layer");
      eye.innerHTML = `<svg class="icon" aria-hidden="true"><use href="${hidden ? "#eye-off" : "#eye"}"></use></svg>`;
      li.append(grip, name, eye);
      if (isVizId(id)) {
        const trash = document.createElement("button");
        trash.type = "button";
        trash.className = "stage-icon-btn";
        trash.dataset.trash = id;
        trash.disabled = vizCount <= 1;
        trash.setAttribute("aria-label", "Delete visualizer");
        trash.innerHTML = '<svg class="icon" aria-hidden="true"><use href="#trash"></use></svg>';
        li.appendChild(trash);
      }
      layerList.appendChild(li);
    });
    if (addViz) addViz.disabled = vizCount >= VIZ_MAX;
  }

  function syncToolbar() {
    themeSelect.value = layout.theme;
    seekSelect.value = layout.seekStyle;
    if (seekDirSelect) seekDirSelect.value = seekDirOf(layout.seekDir);
    playingOpacity.value = String(layout.playingOpacity);
    fillRangeNum(playingOpacity);
    pausedOpacity.value = String(layout.pausedOpacity);
    fillRangeNum(pausedOpacity);
    pulseInput.value = String(layout.pulse);
    fillRangeNum(pulseInput);
    glassInput.checked = layout.glass;
    const selected = new Set(parseGrid(layout.grid));
    gridChecks.forEach((el) => {
      el.checked = selected.has(el.value);
    });
    if (snapInput) snapInput.checked = layout.snap !== false;
    paintGrid();
    paintLayers();
    applyEditTarget();
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
      seekDir: seekDirOf(layout.seekDir),
      playingOpacity: layout.playingOpacity,
      pausedOpacity: layout.pausedOpacity,
      panelW: layout.panelW,
      panelH: layout.panelH,
      pulse: layout.pulse,
      glass: layout.glass,
      grid: layout.grid,
      snap: layout.snap !== false,
      overlayHidden: layout.overlayHidden,
      layers: layout.layers.join(","),
      vizLayers: layout.vizLayers,
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
    const sx = w / layout.canvasW;
    const sy = h / layout.canvasH;
    layout.x = Math.round(layout.x * sx);
    layout.y = Math.round(layout.y * sy);
    layout.vizLayers.forEach((layer) => {
      layer.x = Math.round(layer.x * sx);
      layer.y = Math.round(layer.y * sy);
      layer.w = Math.round(layer.w * sx);
      layer.h = Math.round(layer.h * sy);
    });
    layout.canvasW = w;
    layout.canvasH = h;
    clampBox();
    clampViz();
    const snapped = snapPanel(layout.x, layout.y);
    layout.x = snapped.x;
    layout.y = snapped.y;
    layout.vizLayers.forEach((layer) => {
      const next = snapBox(layer.x, layer.y, layer.w, layer.h, snapTargets(layer.id));
      layer.x = next.x;
      layer.y = next.y;
    });
    applyPanelBox();
    applyVizBoxes();
    fitCanvas();
    saveNow();
  }

  function handleOf(el) {
    const node = el && el.closest ? el.closest("[data-handle]") : null;
    return node ? node.getAttribute("data-handle") : "";
  }

  overlayBox.addEventListener("pointerdown", (ev) => {
    if (editTarget !== "overlay") return;
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
    if (drag.kind !== "resize") return;
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
    const snapped = snapBox(x, y, w, h, snapTargets());
    x = snapped.x;
    y = snapped.y;
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
    if (vizStack) vizStack.querySelectorAll(".viz-box").forEach((box) => box.classList.remove("is-dragging"));
    saveNow();
    refreshNow();
  }

  overlayBox.addEventListener("pointerup", endDrag);
  overlayBox.addEventListener("pointercancel", endDrag);

  if (vizStack) {
    vizStack.addEventListener("pointerdown", (ev) => {
      const box = ev.target.closest("[data-viz-id]");
      if (!box) return;
      const id = box.dataset.vizId;
      if (editTarget !== id) setEditTarget(id);
      if (ev.button !== 0) return;
      const layer = vizLayer(id);
      if (!layer) return;
      ev.preventDefault();
      box.setPointerCapture(ev.pointerId);
      const p = pointerToCanvas(ev);
      const handle = handleOf(ev.target);
      if (handle) {
        drag = {
          kind: "viz-resize",
          id,
          handle,
          x: layer.x,
          y: layer.y,
          w: layer.w,
          h: layer.h,
          px: p.x,
          py: p.y,
        };
      } else {
        drag = { kind: "viz-move", id, dx: p.x - layer.x, dy: p.y - layer.y };
      }
      box.classList.add("is-dragging");
    });
    vizStack.addEventListener("pointermove", (ev) => {
      if (!drag || (drag.kind !== "viz-move" && drag.kind !== "viz-resize")) return;
      const layer = vizLayer(drag.id);
      if (!layer) return;
      const p = pointerToCanvas(ev);
      if (drag.kind === "viz-move") {
        const snapped = snapBox(p.x - drag.dx, p.y - drag.dy, layer.w, layer.h, snapTargets(layer.id));
        if (snapped.x === layer.x && snapped.y === layer.y) return;
        layer.x = snapped.x;
        layer.y = snapped.y;
        applyVizBoxes();
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
      const snapped = snapBox(x, y, w, h, snapTargets(layer.id));
      layer.x = Math.round(snapped.x);
      layer.y = Math.round(snapped.y);
      layer.w = Math.round(w);
      layer.h = Math.round(h);
      applyVizBoxes();
      scheduleLivePut();
    });
    vizStack.addEventListener("pointerup", endDrag);
    vizStack.addEventListener("pointercancel", endDrag);
  }

  function nextVizId() {
    vizSeq += 1;
    let id = `viz-${vizSeq}`;
    while (vizLayer(id)) {
      vizSeq += 1;
      id = `viz-${vizSeq}`;
    }
    return id;
  }

  function addVizLayer() {
    if (layout.vizLayers.length >= VIZ_MAX) return;
    const src = vizLayer(editTarget) || layout.vizLayers[0];
    const layer = defaultVizLayer(nextVizId(), src);
    layer.hidden = false;
    const ox = layer.x + 24;
    const oy = layer.y + 24;
    layer.x = ox + layer.w <= layout.canvasW ? ox : Math.max(0, layer.x - 24);
    layer.y = oy + layer.h <= layout.canvasH ? oy : Math.max(0, layer.y - 24);
    clampVizLayer(layer);
    layout.vizLayers.push(layer);
    layout.layers = [layer.id].concat(layout.layers.filter((id) => id !== layer.id));
    setEditTarget(layer.id);
    applyVizBoxes();
    saveNow();
  }

  function removeVizLayer(id) {
    if (layout.vizLayers.length <= 1) return;
    const idx = layout.vizLayers.findIndex((layer) => layer.id === id);
    if (idx < 0) return;
    layout.vizLayers.splice(idx, 1);
    layout.layers = layout.layers.filter((item) => item !== id);
    const next = layout.vizLayers[Math.min(idx, layout.vizLayers.length - 1)];
    setEditTarget(next ? next.id : "overlay");
    applyVizBoxes();
    saveNow();
  }

  function toggleHidden(id) {
    if (id === "overlay") layout.overlayHidden = !layout.overlayHidden;
    else {
      const layer = vizLayer(id);
      if (!layer) return;
      layer.hidden = !layer.hidden;
    }
    paintLayers();
    applyPanelBox();
    applyVizBoxes();
    lockInspectors();
    saveNow();
  }

  if (addViz) addViz.addEventListener("click", addVizLayer);

  if (layerList) {
    layerList.addEventListener("dragstart", (ev) => {
      const grip = ev.target.closest(".stage-layer-grip");
      const li = ev.target.closest("li");
      if (!grip || !li) {
        ev.preventDefault();
        return;
      }
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
    layerList.addEventListener("click", (ev) => {
      const eye = ev.target.closest("[data-eye]");
      if (eye) {
        ev.stopPropagation();
        toggleHidden(eye.dataset.eye);
        return;
      }
      const trash = ev.target.closest("[data-trash]");
      if (trash) {
        ev.stopPropagation();
        if (!trash.disabled) removeVizLayer(trash.dataset.trash);
        return;
      }
      const li = ev.target.closest("li");
      if (!li || !li.dataset.layer) return;
      setEditTarget(li.dataset.layer);
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
      layout.layers = parseLayers(order.join(","), layout.vizLayers.map((layer) => layer.id));
      paintLayers();
      applyStack();
      saveNow();
    });
  }

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
  if (seekDirSelect) {
    seekDirSelect.addEventListener("change", () => {
      layout.seekDir = seekDirOf(seekDirSelect.value);
      applyLook();
      saveNow();
    });
  }
  pairRange(playingOpacity);
  pairRange(pausedOpacity);
  pairRange(pulseInput);
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
  if (snapInput) {
    snapInput.addEventListener("change", () => {
      layout.snap = snapInput.checked;
      saveNow();
    });
  }
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
      copyOverlayUrl.setAttribute("aria-label", ok ? "Copied" : "Copy Overlay URL");
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

  if (window.JB2Viz) {
    window.JB2Viz.onLookChange = function (id, look) {
      const layer = vizLayer(id);
      if (!layer) return;
      layer.style = look.style;
      layer.color = look.color;
      layer.colorDir = look.colorDir;
      layer.bands = look.bands;
      layer.mirror = look.mirror;
      layer.peakFall = look.peakFall;
      saveNow();
    };
  }

  function readVizLayers(data) {
    const raw = Array.isArray(data.vizLayers) ? data.vizLayers : [];
    if (raw.length) {
      layout.vizLayers = raw.map((layer, i) => defaultVizLayer(layer.id || `viz-${i + 1}`, layer));
    } else {
      layout.vizLayers = [
        defaultVizLayer("viz", {
          x: data.vizX,
          y: data.vizY,
          w: data.vizW,
          h: data.vizH,
        }),
      ];
    }
    layout.vizLayers.forEach((layer) => {
      const n = Number(String(layer.id).replace(/^viz-?/, ""));
      if (Number.isFinite(n) && n >= vizSeq) vizSeq = n;
    });
    layout.layers = parseLayers(data.layers, layout.vizLayers.map((layer) => layer.id));
  }

  function readLayout(data) {
    if (!data) return;
    layout.x = Number(data.x) || 0;
    layout.y = Number(data.y) || 0;
    layout.canvasW = Number(data.canvasW) || 1920;
    layout.canvasH = Number(data.canvasH) || 1080;
    if (data.theme) layout.theme = data.theme;
    if (data.seekStyle) layout.seekStyle = data.seekStyle;
    if (data.seekDir) layout.seekDir = seekDirOf(data.seekDir);
    if (data.playingOpacity != null) layout.playingOpacity = Number(data.playingOpacity);
    if (data.pausedOpacity != null) layout.pausedOpacity = Number(data.pausedOpacity);
    if (data.panelW) layout.panelW = Number(data.panelW);
    if (data.panelH != null) layout.panelH = Number(data.panelH) || 0;
    if (data.pulse != null) layout.pulse = Number(data.pulse);
    if (typeof data.glass === "boolean") layout.glass = data.glass;
    if (data.grid) layout.grid = formatGrid(parseGrid(data.grid));
    if (typeof data.snap === "boolean") layout.snap = data.snap;
    if (typeof data.overlayHidden === "boolean") layout.overlayHidden = data.overlayHidden;
    readVizLayers(data);
  }

  if (window.JB2OverlayFx) {
    window.JB2OverlayFx.init(overlayBox);
  }

  applyLook();
  applyVizBoxes();
  paintLayers();
  applyEditTarget();

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
      applyVizBoxes();
      refreshNow();
      fitCanvas();
    });
})();

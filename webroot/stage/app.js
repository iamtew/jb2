(function () {
  const SNAP_PX = 12;
  const PUT_MS = 100;
  const DEMO_DURATION = 180;

  const canvas = document.getElementById("stageCanvas");
  const fit = document.getElementById("stageFit");
  const songInfo = document.getElementById("songInfo");
  const btn720 = document.getElementById("size720");
  const btn1080 = document.getElementById("size1080");
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

  const layout = { x: 0, y: 0, canvasW: 1920, canvasH: 1080 };
  let drag = null;
  let putTimer = null;
  let demoPaused = false;
  let demoPos = 0;

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

  function setPaused(paused) {
    demoPaused = paused;
    if (ui.playState) ui.playState.textContent = paused ? "⏸" : "⏵";
    songInfo.style.transition = "opacity 600ms ease";
    songInfo.style.opacity = paused ? "0.25" : "1";
    songInfo.classList.toggle("is-glowing", !paused);
  }

  function panelSize() {
    return { w: songInfo.offsetWidth || 500, h: songInfo.offsetHeight || 160 };
  }

  function snapAxis(origin, size, canvas, lines) {
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
    return Math.max(0, Math.min(canvas - size, Math.round(next)));
  }

  function snapPanel(x, y) {
    const { w, h } = panelSize();
    const xs = [0, layout.canvasW / 3, layout.canvasW / 2, (2 * layout.canvasW) / 3, layout.canvasW];
    const ys = [0, layout.canvasH / 3, layout.canvasH / 2, (2 * layout.canvasH) / 3, layout.canvasH];
    return { x: snapAxis(x, w, layout.canvasW, xs), y: snapAxis(y, h, layout.canvasH, ys) };
  }

  function applyPanelPos() {
    songInfo.style.left = `${layout.x}px`;
    songInfo.style.top = `${layout.y}px`;
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

  function putLayout(save) {
    const body = {
      x: layout.x,
      y: layout.y,
      canvasW: layout.canvasW,
      canvasH: layout.canvasH,
      save: Boolean(save),
    };
    return fetch("/api/overlay", {
      method: "PUT",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(body),
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

  function setCanvasSize(w, h) {
    if (w === layout.canvasW && h === layout.canvasH) return;
    const { w: pw, h: ph } = panelSize();
    layout.x = Math.round((layout.x * w) / layout.canvasW);
    layout.y = Math.round((layout.y * h) / layout.canvasH);
    layout.canvasW = w;
    layout.canvasH = h;
    const snapped = snapPanel(layout.x, layout.y);
    layout.x = Math.max(0, Math.min(w - pw, snapped.x));
    layout.y = Math.max(0, Math.min(h - ph, snapped.y));
    applyPanelPos();
    fitCanvas();
    saveNow();
  }

  songInfo.addEventListener("pointerdown", (ev) => {
    if (ev.button !== 0) return;
    ev.preventDefault();
    songInfo.setPointerCapture(ev.pointerId);
    const p = pointerToCanvas(ev);
    drag = { dx: p.x - layout.x, dy: p.y - layout.y };
    songInfo.classList.add("is-dragging");
  });

  songInfo.addEventListener("pointermove", (ev) => {
    if (!drag) return;
    const p = pointerToCanvas(ev);
    const snapped = snapPanel(p.x - drag.dx, p.y - drag.dy);
    if (snapped.x === layout.x && snapped.y === layout.y) return;
    layout.x = snapped.x;
    layout.y = snapped.y;
    applyPanelPos();
    scheduleLivePut();
  });

  function endDrag() {
    if (!drag) return;
    drag = null;
    songInfo.classList.remove("is-dragging");
    saveNow();
  }

  songInfo.addEventListener("pointerup", endDrag);
  songInfo.addEventListener("pointercancel", endDrag);

  btn720.addEventListener("click", () => setCanvasSize(1280, 720));
  btn1080.addEventListener("click", () => setCanvasSize(1920, 1080));
  window.addEventListener("resize", fitCanvas);

  songInfo.style.setProperty("--glow-duration", "2s");
  songInfo.classList.add("is-glowing");
  setPaused(false);

  window.requestAnimationFrame(() => {
    updateTextMarquee(ui.title, ui.titleWrap);
    updateTextMarquee(ui.artist, ui.artistWrap);
    updateTextMarquee(ui.album, ui.albumWrap);
    updateMarker();
  });

  window.setInterval(() => {
    if (demoPaused) return;
    demoPos += 1;
    if (demoPos > DEMO_DURATION) demoPos = 0;
    updateMarker();
  }, 1000);

  window.setInterval(() => {
    setPaused(!demoPaused);
  }, 8000);

  fetch("/api/overlay")
    .then((res) => res.json())
    .then((env) => {
      if (!env || env.exitCode !== 0 || !env.data) return;
      layout.x = Number(env.data.x) || 0;
      layout.y = Number(env.data.y) || 0;
      layout.canvasW = Number(env.data.canvasW) || 1920;
      layout.canvasH = Number(env.data.canvasH) || 1080;
    })
    .catch(() => {})
    .finally(() => {
      applyPanelPos();
      fitCanvas();
    });
})();

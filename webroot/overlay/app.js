(function () {
  const output = document.getElementById("output");
  const songInfo = document.getElementById("songInfo");

  const params = new URLSearchParams(window.location.search);
  const devMode = params.get("dev") === "true";
  const pulseParam = params.get("pulse");
  const pulseOverride =
    pulseParam === null ? null : Math.max(0, Math.min(10, Number(pulseParam) || 0));
  const bgParam = params.get("bg");
  const glassOverride = bgParam === "0" || bgParam === "false" ? false : null;

  const look = {
    playingOpacity: 1,
    pausedOpacity: 0.25,
    pulse: pulseOverride === null ? 3 : pulseOverride,
    glass: glassOverride === null ? true : glassOverride,
    theme: "lcd-glass",
  };

  if (devMode) {
    document.documentElement.classList.add("dev");
    document.body.classList.add("dev");
    if (output) output.style.display = "block";
  } else if (output) {
    output.style.display = "none";
  }

  function log(obj) {
    if (!devMode || !output) return;
    output.textContent = JSON.stringify(obj, null, 2);
  }

  function applyChrome() {
    if (!songInfo) return;
    songInfo.classList.toggle("is-text-only", look.glass === false);
    songInfo.style.backdropFilter = "none";
    songInfo.style.webkitBackdropFilter = "none";
  }

  const BASE_W = 500;

  function applyLayout(layout) {
    if (!songInfo || !layout) return;
    const x = Number(layout.x);
    const y = Number(layout.y);
    const panelW = Number(layout.panelW);
    songInfo.style.left = `${Number.isFinite(x) ? x : 0}px`;
    songInfo.style.top = `${Number.isFinite(y) ? y : 0}px`;
    songInfo.style.width = `${BASE_W}px`;
    songInfo.style.removeProperty("height");
    const w = Number.isFinite(panelW) && panelW > 0 ? panelW : BASE_W;
    const s = w / BASE_W;
    songInfo.style.transformOrigin = "0 0";
    songInfo.style.transform = s === 1 ? "none" : `scale(${s})`;
    applyVizFrame(layout);
    if (songInfo) songInfo.hidden = Boolean(layout.overlayHidden);
    songInfo.setAttribute("data-theme", layout.theme || "lcd-glass");
    songInfo.setAttribute("data-seek", layout.seekStyle || "sesh-sofa");
    const seekDir = layout.seekDir;
    songInfo.setAttribute(
      "data-seek-dir",
      seekDir === "rl" || seekDir === "tb" || seekDir === "bt" ? seekDir : "lr"
    );
    const playOp = Number(layout.playingOpacity);
    const pauseOp = Number(layout.pausedOpacity);
    look.playingOpacity = Number.isFinite(playOp) ? playOp : 1;
    look.pausedOpacity = Number.isFinite(pauseOp) ? pauseOp : 0.25;
    if (pulseOverride === null) {
      const pulse = Number(layout.pulse);
      look.pulse = Number.isFinite(pulse) ? pulse : 3;
    }
    if (glassOverride === null) look.glass = layout.glass !== false;
    look.theme = layout.theme || "lcd-glass";
    applyChrome();
    if (window.JB2NowPlaying) window.JB2NowPlaying.refresh();
  }

  function applyVizFrame(layout) {
    const stack = document.getElementById("vizStack");
    if (!stack || !layout) return;
    const vizLayers = Array.isArray(layout.vizLayers) ? layout.vizLayers : [];
    const byId = {};
    vizLayers.forEach((layer) => {
      if (layer && layer.id) byId[layer.id] = layer;
    });
    const order = String(layout.layers || "")
      .split(",")
      .map((id) => id.trim())
      .filter(Boolean);
    if (!order.includes("overlay")) order.push("overlay");
    const depth = order.length;
    order.forEach((id, i) => {
      const z = String(depth - i + 1);
      if (id === "overlay") {
        if (songInfo) songInfo.style.zIndex = z;
        return;
      }
      const layer = byId[id];
      if (!layer) return;
      let box = stack.querySelector(`[data-viz-id="${id}"]`);
      if (!box) {
        box = document.createElement("div");
        box.className = "viz-box";
        box.dataset.vizId = id;
        const host = document.createElement("div");
        host.className = "viz-layer";
        host.setAttribute("aria-hidden", "true");
        box.appendChild(host);
        stack.appendChild(box);
      }
      const x = Number(layer.x);
      const y = Number(layer.y);
      const w = Number(layer.w);
      const h = Number(layer.h);
      box.style.left = `${Number.isFinite(x) ? x : 0}px`;
      box.style.top = `${Number.isFinite(y) ? y : 0}px`;
      box.style.width = `${Number.isFinite(w) && w > 0 ? w : 1920}px`;
      box.style.height = `${Number.isFinite(h) && h > 0 ? h : 280}px`;
      box.style.zIndex = z;
    });
    stack.querySelectorAll("[data-viz-id]").forEach((box) => {
      if (!byId[box.dataset.vizId]) box.remove();
    });
    if (window.JB2Viz) window.JB2Viz.sync(layout);
  }

  function followOverlayLayout() {
    fetch("/api/overlay")
      .then((res) => res.json())
      .then((env) => {
        if (env && env.exitCode === 0) applyLayout(env.data);
      })
      .catch(() => {});
    const events = new EventSource("/api/events");
    events.addEventListener("overlay", (event) => {
      try {
        applyLayout(JSON.parse(event.data || "{}"));
      } catch (err) {
        log({ error: "overlay layout parse", details: String(err) });
      }
    });
    events.addEventListener("audioVisualizer", (event) => {
      try {
        const data = JSON.parse(event.data || "{}");
        window.dispatchEvent(new CustomEvent("jb2-viz", { detail: data }));
      } catch (err) {
        log({ error: "audio visualizer parse", details: String(err) });
      }
    });
  }

  applyChrome();

  if (window.JB2OverlayFx) {
    window.JB2OverlayFx.init(document.querySelector(".overlay-stage"));
  }

  if (window.JB2NowPlaying) {
    window.JB2NowPlaying.start({ look: look, onMessage: log });
  }
  followOverlayLayout();
})();

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
    const vizBox = document.getElementById("vizBox");
    if (!vizBox || !layout) return;
    const order = String(layout.layers || "overlay,viz").split(",");
    const vizFront = order[0] === "viz";
    const x = Number(layout.vizX);
    const y = Number(layout.vizY);
    const w = Number(layout.vizW);
    const h = Number(layout.vizH);
    vizBox.style.left = `${Number.isFinite(x) ? x : 0}px`;
    vizBox.style.top = `${Number.isFinite(y) ? y : 0}px`;
    vizBox.style.width = `${Number.isFinite(w) && w > 0 ? w : 1920}px`;
    vizBox.style.height = `${Number.isFinite(h) && h > 0 ? h : 280}px`;
    vizBox.style.zIndex = vizFront ? "3" : "1";
    if (songInfo) songInfo.style.zIndex = vizFront ? "1" : "3";
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

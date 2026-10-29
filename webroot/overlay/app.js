(function () {
  const output = document.getElementById("output");

  const ui = {
    songInfo: document.getElementById("songInfo"),
    overlayState: document.getElementById("overlayState"),
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

  const state = {
    songDuration: 0,
    position: 0,
    pauseFadeTimer: null,
    progressStallTimer: null,
    isPaused: true,
    hasSignal: false,
    fadeGeneration: 0,
    playingOpacity: 1,
    pausedOpacity: 0.25,
    pulse: 3,
    glass: true,
  };

  const timings = {
    fadeOutDelay: 180,
    fadeOutDuration: 600,
    fadeInDuration: 180,
  };
  // YTMD position ticks are often ~1s; keep headroom so we don't false-pause between ticks.
  const PROGRESS_STALL_MS = 1800;

  const params = new URLSearchParams(window.location.search);
  const devMode = params.get("dev") === "true";
  const protocol = window.location.protocol === "https:" ? "wss" : "ws";
  const host = params.get("host") || window.location.hostname || "localhost";
  const port = params.get("port") || "26538";
  const pulseParam = params.get("pulse");
  const pulseOverride =
    pulseParam === null ? null : Math.max(0, Math.min(10, Number(pulseParam) || 0));
  const bgParam = params.get("bg");
  const glassOverride = bgParam === "0" || bgParam === "false" ? false : null;
  const WS_URL = `${protocol}://${host}:${port}/api/v1/ws`;

  if (devMode) {
    document.documentElement.classList.add("dev");
    document.body.classList.add("dev");
    if (output) output.style.display = "block";
  } else if (output) {
    output.style.display = "none";
  }

  function pulseSpeed() {
    return pulseOverride === null ? state.pulse : pulseOverride;
  }

  function useGlass() {
    return glassOverride === null ? state.glass : glassOverride;
  }

  function applyChrome() {
    if (!ui.songInfo) return;
    ui.songInfo.classList.toggle("is-text-only", !useGlass());
    ui.songInfo.style.backdropFilter = "none";
    ui.songInfo.style.webkitBackdropFilter = "none";
  }

  function syncFx() {
    const fx = window.JB2OverlayFx;
    if (!fx || !ui.songInfo) return;
    fx.set({
      theme: ui.songInfo.getAttribute("data-theme") || "lcd-glass",
      pulse: pulseSpeed(),
      glass: useGlass(),
      playing: state.hasSignal && !state.isPaused,
    });
  }

  function setStatus(kind) {
    if (!ui.songInfo) return;
    if (!kind) {
      ui.songInfo.classList.remove("is-status");
      return;
    }
    if (ui.overlayState) {
      ui.overlayState.innerHTML =
        kind === "nosignal"
          ? "NO SIGNAL"
          : '<span class="overlay-state-lead">Jukeboks Paused</span><span class="overlay-state-hint">Add song with !sr &lt;song / URL&gt;</span>';
    }
    ui.songInfo.classList.add("is-status");
  }

  function applyVisual(opts) {
    const duration = opts && Number.isFinite(opts.duration) ? opts.duration : 0;
    if (!state.hasSignal) {
      setStatus("nosignal");
      if (ui.playState) ui.playState.textContent = "⏸";
      setSongInfoOpacity(state.playingOpacity, duration);
      stopGlowPulse();
      syncFx();
      return;
    }
    if (state.isPaused) {
      setStatus("paused");
      if (ui.playState) ui.playState.textContent = "⏸";
      setSongInfoOpacity(state.pausedOpacity, duration);
      stopGlowPulse();
      syncFx();
      return;
    }
    setStatus(null);
    if (ui.playState) ui.playState.textContent = "⏵";
    setSongInfoOpacity(state.playingOpacity, duration);
    startGlowPulse();
    syncFx();
  }

  const BASE_W = 500;

  function applyLayout(layout) {
    if (!ui.songInfo || !layout) return;
    const x = Number(layout.x);
    const y = Number(layout.y);
    const panelW = Number(layout.panelW);
    ui.songInfo.style.left = `${Number.isFinite(x) ? x : 0}px`;
    ui.songInfo.style.top = `${Number.isFinite(y) ? y : 0}px`;
    ui.songInfo.style.width = `${BASE_W}px`;
    ui.songInfo.style.removeProperty("height");
    const w = Number.isFinite(panelW) && panelW > 0 ? panelW : BASE_W;
    const s = w / BASE_W;
    ui.songInfo.style.transformOrigin = "0 0";
    ui.songInfo.style.transform = s === 1 ? "none" : `scale(${s})`;
    ui.songInfo.setAttribute("data-theme", layout.theme || "lcd-glass");
    ui.songInfo.setAttribute("data-seek", layout.seekStyle || "sesh-sofa");
    const playOp = Number(layout.playingOpacity);
    const pauseOp = Number(layout.pausedOpacity);
    state.playingOpacity = Number.isFinite(playOp) ? playOp : 1;
    state.pausedOpacity = Number.isFinite(pauseOp) ? pauseOp : 0.25;
    const pulse = Number(layout.pulse);
    state.pulse = Number.isFinite(pulse) ? pulse : 3;
    state.glass = layout.glass !== false;
    applyChrome();
    applyVisual({ duration: 0 });
    syncFx();
    window.requestAnimationFrame(() => {
      updateTextMarquee(ui.title, ui.titleWrap);
      updateTextMarquee(ui.artist, ui.artistWrap);
      updateTextMarquee(ui.album, ui.albumWrap);
      updateMarker();
    });
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
  }

  function setSongInfoOpacity(opacity, duration) {
    if (!ui.songInfo) return;
    ui.songInfo.style.transition = `opacity ${duration}ms ease`;
    ui.songInfo.style.opacity = String(opacity);
  }

  function resetPauseFade() {
    if (state.pauseFadeTimer) {
      clearTimeout(state.pauseFadeTimer);
      state.pauseFadeTimer = null;
    }
  }

  function startGlowPulse() {
    if (!ui.songInfo) return;
    const speed = pulseSpeed();
    if (speed <= 0) {
      ui.songInfo.classList.remove("is-glowing");
      return;
    }
    const speedScale = Math.max(0.1, speed / 5);
    ui.songInfo.style.setProperty("--glow-duration", `${(2 / speedScale).toFixed(2)}s`);
    ui.songInfo.classList.add("is-glowing");
  }

  function stopGlowPulse() {
    ui.songInfo?.classList.remove("is-glowing");
  }

  // Paused → fade + request cover. Playing → track. No WS → NO SIGNAL.
  function handlePlaybackState(isPaused) {
    const next = Boolean(isPaused);
    if (state.hasSignal && state.isPaused === next) {
      if (ui.playState) ui.playState.textContent = next ? "⏸" : "⏵";
      return;
    }
    state.isPaused = next;
    resetPauseFade();
    state.fadeGeneration += 1;
    const generation = state.fadeGeneration;

    if (!state.hasSignal) {
      applyVisual({ duration: 0 });
      return;
    }

    if (next) {
      state.pauseFadeTimer = window.setTimeout(() => {
        if (generation !== state.fadeGeneration) return;
        applyVisual({ duration: timings.fadeOutDuration });
      }, timings.fadeOutDelay);
      return;
    }

    applyVisual({ duration: timings.fadeInDuration });
  }

  function setHasSignal(ok) {
    state.hasSignal = Boolean(ok);
    resetPauseFade();
    applyVisual({ duration: ok ? timings.fadeInDuration : 0 });
  }

  // YTMD sends POSITION_CHANGED while playing and stops when paused — that is the truth signal.
  function notePlaybackProgress() {
    window.clearTimeout(state.progressStallTimer);
    state.progressStallTimer = window.setTimeout(() => {
      if (state.isPaused) return;
      handlePlaybackState(true);
    }, PROGRESS_STALL_MS);
  }

  function formatTime(sec) {
    const m = Math.floor(sec / 60);
    const s = sec % 60;
    return `${String(m).padStart(2, "0")}:${String(s).padStart(2, "0")}`;
  }

  function updateMarker() {
    if (state.songDuration <= 0 || !ui.bar || !ui.marker || !ui.fill) return;
    const barWidth = ui.bar.clientWidth;
    const pct = state.position / state.songDuration;
    const x = Math.min(barWidth - 6, Math.max(0, barWidth * pct));
    const fillWidth = Math.max(0, Math.min(barWidth, barWidth * pct));
    ui.marker.style.transform = `translateX(${x}px)`;
    ui.fill.style.width = `${fillWidth}px`;
  }

  function updateRemainingTime() {
    if (!ui.totalTime) return;
    ui.totalTime.textContent = formatTime(Math.max(0, state.songDuration - state.position));
  }

  function updateOutput(data) {
    if (!devMode || !output) return;
    output.textContent = JSON.stringify(data, null, 2);
  }

  function log(obj) {
    if (!devMode || !output) return;
    output.textContent = JSON.stringify(obj, null, 2);
  }

  function getMarqueeTrack(textEl) {
    if (!textEl) return null;
    let track = textEl.querySelector(".overlay-marquee-track");
    if (!track) {
      track = document.createElement("span");
      track.className = "overlay-marquee-track";
      track.textContent = textEl.textContent || "";
      textEl.textContent = "";
      textEl.appendChild(track);
    }
    return track;
  }

  function setMarqueeText(textEl, value) {
    const track = getMarqueeTrack(textEl);
    if (!track) return false;
    const nextValue = String(value || "");
    if (track.textContent === nextValue) return false;
    track.textContent = nextValue;
    return true;
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

  function setOverlayField(textEl, wrapEl, value) {
    if (setMarqueeText(textEl, value)) {
      updateTextMarquee(textEl, wrapEl);
    }
  }

  let ws;

  function connect() {
    ws = new WebSocket(WS_URL);

    ws.onopen = () => {
      log({ status: "connected", url: WS_URL });
      setHasSignal(true);
      handlePlaybackState(true);
    };

    ws.onmessage = (msg) => {
      try {
        const data = JSON.parse(msg.data);

        if (data.type === "PLAYER_INFO" || data.type === "VIDEO_CHANGED") {
          if (data.song) {
            setOverlayField(ui.artist, ui.artistWrap, data.song.artist || "");
            setOverlayField(ui.title, ui.titleWrap, data.song.title || "");
            setOverlayField(ui.album, ui.albumWrap, data.song.album || "");

            if (data.song.songDuration) {
              state.songDuration = data.song.songDuration;
            }

            if (typeof data.position === "number") {
              state.position = data.position;
            } else if (typeof data.song.elapsedSeconds === "number") {
              state.position = data.song.elapsedSeconds;
            }
            ui.currentTime.textContent = formatTime(state.position);
            updateMarker();
            updateRemainingTime();
            // Do not take play/pause from PLAYER_INFO — song.isPaused / isPlaying are stale.
            // POSITION_CHANGED stall + PLAYER_STATE_CHANGED are the truth.
          }
          updateOutput(data);
        }

        if (data.type === "POSITION_CHANGED") {
          const next = Math.max(0, Number(data.position) || 0);
          const advanced = next > state.position + 0.05;
          state.position = next;
          ui.currentTime.textContent = formatTime(state.position);
          updateRemainingTime();
          updateMarker();
          if (advanced) {
            handlePlaybackState(false);
            notePlaybackProgress();
          }
        }

        if (data.type === "PLAYER_STATE_CHANGED") {
          const isPaused = typeof data.isPlaying === "boolean" ? !data.isPlaying : Boolean(data.isPaused);
          handlePlaybackState(isPaused);
          if (isPaused) {
            window.clearTimeout(state.progressStallTimer);
          } else {
            notePlaybackProgress();
          }
          if (data.position != null) {
            state.position = Math.max(0, Number(data.position) || 0);
            ui.currentTime.textContent = formatTime(state.position);
            updateRemainingTime();
            updateMarker();
          }
        }
      } catch (err) {
        log({ error: "Failed to parse message", raw: msg.data });
      }
    };

    ws.onclose = () => {
      log({ status: "disconnected" });
      setHasSignal(false);
      setTimeout(connect, 2000);
    };

    ws.onerror = (err) => {
      log({ error: "WebSocket error", details: err });
    };
  }

  window.requestAnimationFrame(() => {
    updateTextMarquee(ui.title, ui.titleWrap);
    updateTextMarquee(ui.artist, ui.artistWrap);
    updateTextMarquee(ui.album, ui.albumWrap);
  });

  window.addEventListener("resize", () => {
    updateTextMarquee(ui.title, ui.titleWrap);
    updateTextMarquee(ui.artist, ui.artistWrap);
    updateTextMarquee(ui.album, ui.albumWrap);
    updateMarker();
  });

  applyChrome();
  applyVisual({ duration: 0 });

  if (window.JB2OverlayFx) {
    window.JB2OverlayFx.init(document.querySelector(".overlay-stage"));
  }

  connect();
  followOverlayLayout();
})();

// Live now-playing for /overlay/ and /stage/. Same DOM ids, same pear-desktop rules.
(function () {
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
  };

  const timings = {
    fadeOutDuration: 600,
    fadeInDuration: 180,
  };
  // Hold last track through YTMD's between-song pause blip; retune if the desk gap feels wrong.
  const PAUSE_COPY_GRACE_MS = 2000;
  // YTMD position ticks are often ~1s; keep headroom so we don't false-pause between ticks.
  const PROGRESS_STALL_MS = 1800;

  const params = new URLSearchParams(window.location.search);
  const protocol = window.location.protocol === "https:" ? "wss" : "ws";
  const host = params.get("host") || window.location.hostname || "localhost";
  const port = params.get("port") || "26538";

  let look = {
    playingOpacity: 1,
    pausedOpacity: 0.25,
    pulse: 3,
    glass: true,
    theme: "lcd-glass",
  };
  let onChange = function () {};
  let onMessage = function () {};
  let started = false;
  let ws;

  function ytmdWsUrl(token) {
    const url = `${protocol}://${host}:${port}/api/v1/ws`;
    if (!token) return url;
    return `${url}?token=${encodeURIComponent(token)}`;
  }

  function emit(obj) {
    onMessage(obj);
  }

  function pulseSpeed() {
    const speed = Number(look.pulse);
    return Number.isFinite(speed) ? speed : 3;
  }

  function useGlass() {
    return look.glass !== false;
  }

  function themeName() {
    return look.theme || (ui.songInfo && ui.songInfo.getAttribute("data-theme")) || "lcd-glass";
  }

  function syncFx() {
    const fx = window.JB2OverlayFx;
    if (!fx || !ui.songInfo) return;
    fx.set({
      theme: themeName(),
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

  function setSongInfoOpacity(opacity, duration) {
    if (!ui.songInfo) return;
    ui.songInfo.style.transition = `opacity ${duration}ms ease`;
    ui.songInfo.style.opacity = String(opacity);
  }

  function opacityOf(key, fallback) {
    const n = Number(look[key]);
    return Number.isFinite(n) ? n : fallback;
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

  function applyVisual(opts) {
    const duration = opts && Number.isFinite(opts.duration) ? opts.duration : 0;
    if (!state.hasSignal) {
      setStatus("nosignal");
      if (ui.playState) ui.playState.textContent = "⏸";
      setSongInfoOpacity(opacityOf("playingOpacity", 1), duration);
      stopGlowPulse();
    } else if (state.isPaused) {
      setStatus("paused");
      if (ui.playState) ui.playState.textContent = "⏸";
      setSongInfoOpacity(opacityOf("pausedOpacity", 0.25), duration);
      stopGlowPulse();
    } else {
      setStatus(null);
      if (ui.playState) ui.playState.textContent = "⏵";
      setSongInfoOpacity(opacityOf("playingOpacity", 1), duration);
      startGlowPulse();
    }
    syncFx();
    onChange();
  }

  function resetPauseFade() {
    if (state.pauseFadeTimer) {
      clearTimeout(state.pauseFadeTimer);
      state.pauseFadeTimer = null;
    }
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
      }, PAUSE_COPY_GRACE_MS);
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

  function repaintText() {
    updateTextMarquee(ui.title, ui.titleWrap);
    updateTextMarquee(ui.artist, ui.artistWrap);
    updateTextMarquee(ui.album, ui.albumWrap);
    updateMarker();
  }

  function refresh() {
    applyVisual({ duration: 0 });
    window.requestAnimationFrame(repaintText);
  }

  function connect() {
    fetch("/api/ytmd")
      .then((res) => res.json())
      .then((env) => {
        const token = env && env.exitCode === 0 ? env.data && env.data.accessToken : "";
        if (!token) {
          setHasSignal(false);
          setTimeout(connect, 2000);
          return;
        }
        openSocket(ytmdWsUrl(token));
      })
      .catch((err) => {
        emit({ error: "ytmd token", details: String(err) });
        setHasSignal(false);
        setTimeout(connect, 2000);
      });
  }

  function openSocket(url) {
    ws = new WebSocket(url);

    ws.onopen = () => {
      emit({ status: "connected", url });
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
          emit(data);
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
        emit({ error: "Failed to parse message", raw: msg.data });
      }
    };

    ws.onclose = () => {
      emit({ status: "disconnected" });
      setHasSignal(false);
      setTimeout(connect, 2000);
    };

    ws.onerror = (err) => {
      emit({ error: "WebSocket error", details: err });
    };
  }

  function start(opts) {
    opts = opts || {};
    if (opts.look) look = opts.look;
    if (typeof opts.onChange === "function") onChange = opts.onChange;
    if (typeof opts.onMessage === "function") onMessage = opts.onMessage;
    if (started) {
      refresh();
      return;
    }
    started = true;
    window.addEventListener("resize", repaintText);
    refresh();
    connect();
  }

  window.JB2NowPlaying = { start: start, refresh: refresh };
})();

(function () {
  const BLEED = 48;
  const BASE_W = 500;
  const COUNT = 28;

  const SKINS = {
    "lcd-glass": { mode: "prism", a: 0x3ec8ff, b: 0xff4cf0 },
    "smoke-glass": { mode: "prism", a: 0xc8d4e0, b: 0x8ec8ff },
    "rose-glass": { mode: "prism", a: 0xff6ab8, b: 0xffd070 },
    "jade-glass": { mode: "prism", a: 0x5cffc0, b: 0xb8ff4c },
    "violet-glass": { mode: "prism", a: 0xb48cff, b: 0x4ce8ff },
    "ember-glass": { mode: "prism", a: 0xffb040, b: 0xff4c28 },
    "lower-third": { mode: "edge", a: 0xee1111, b: 0xf4f4f4 },
    "amber-terminal": { mode: "crt", a: 0xffb000, b: 0xff6a00 },
    crypt: { mode: "ember", a: 0xc02020, b: 0xff6a20 },
    fraktur: { mode: "gold", a: 0xe0c060, b: 0x8a6a20 },
    konstrukt: { mode: "blocks", a: 0xcc0000, b: 0xf4ead8 },
    street: { mode: "rain", a: 0x39ff14, b: 0xff2bd6 },
    drop: { mode: "bubbles", a: 0xff78b4, b: 0xffdc50 },
    capsule: { mode: "bokeh", a: 0x3cb89a, b: 0xa8fff0 },
    poster: { mode: "burst", a: 0x00c8ff, b: 0xe85d04 },
  };

  let app = null;
  let panel = null;
  let gfx = null;
  let motes = [];
  let orbTex = null;
  let sprites = [];
  let ready = false;
  let reduced = false;
  let clock = 0;
  let lastTheme = "";
  const view = { w: BASE_W, h: 160, s: 1 };
  const state = {
    theme: "lcd-glass",
    pulse: 3,
    playing: false,
    glass: true,
  };

  function skinOf() {
    return SKINS[state.theme] || SKINS["lcd-glass"];
  }

  function intensity() {
    if (!state.glass) return 0;
    const pulse = Math.max(0, Number(state.pulse) || 0);
    if (pulse <= 0) return 0.22;
    const live = state.playing ? 1 : 0.28;
    return Math.min(1, (pulse / 5) * 0.55 + 0.2) * live;
  }

  function wantTicker() {
    return ready && state.glass && state.playing && (Number(state.pulse) || 0) > 0 && !reduced;
  }

  function makeOrbTexture() {
    const canvas = document.createElement("canvas");
    canvas.width = 64;
    canvas.height = 64;
    const g = canvas.getContext("2d");
    const glow = g.createRadialGradient(32, 32, 0, 32, 32, 32);
    glow.addColorStop(0, "rgba(255,255,255,0.9)");
    glow.addColorStop(0.35, "rgba(255,255,255,0.35)");
    glow.addColorStop(1, "rgba(255,255,255,0)");
    g.fillStyle = glow;
    g.fillRect(0, 0, 64, 64);
    const texture = PIXI.Texture.from(canvas);
    if (texture.source) texture.source.scaleMode = "linear";
    return texture;
  }

  function resetMotes() {
    const mode = skinOf().mode;
    const w = view.w + BLEED * 2;
    const h = view.h + BLEED * 2;
    for (let i = 0; i < COUNT; i++) {
      const p = motes[i] || (motes[i] = {});
      p.x = Math.random() * w;
      p.y = Math.random() * h;
      p.r = 2 + Math.random() * 5;
      p.phase = Math.random() * Math.PI * 2;
      if (mode === "edge") {
        p.x = BLEED + 4 + Math.random() * 10;
        p.y = BLEED + Math.random() * view.h;
        p.vy = 12 + Math.random() * 28;
        p.vx = (Math.random() - 0.5) * 8;
      } else if (mode === "rain") {
        p.x = Math.random() * w;
        p.y = Math.random() * h;
        p.vy = 80 + Math.random() * 140;
        p.vx = -18 - Math.random() * 22;
        p.r = 1 + Math.random() * 2;
      } else if (mode === "ember") {
        p.vy = -18 - Math.random() * 36;
        p.vx = (Math.random() - 0.5) * 16;
      } else if (mode === "bubbles") {
        p.vy = -16 - Math.random() * 28;
        p.vx = (Math.random() - 0.5) * 12;
        p.r = 4 + Math.random() * 10;
      } else if (mode === "crt") {
        p.vx = 40 + Math.random() * 80;
        p.vy = 0;
        p.r = 1;
      } else {
        p.vx = (Math.random() - 0.5) * 24;
        p.vy = (Math.random() - 0.5) * 24;
      }
    }
  }

  function seedSprites() {
    if (!orbTex || !app) return;
    while (sprites.length < 12) {
      const spr = new PIXI.Sprite(orbTex);
      spr.anchor.set(0.5);
      spr.blendMode = "add";
      app.stage.addChild(spr);
      sprites.push(spr);
    }
  }

  function layoutCanvas() {
    if (!ready || !panel || !app) return;
    const w = panel.offsetWidth || BASE_W;
    const h = panel.offsetHeight || 160;
    const transform = panel.style.transform || "none";
    const scaleMatch = transform.match(/scale\(([\d.]+)\)/);
    const s = scaleMatch ? Number(scaleMatch[1]) : 1;
    view.w = w;
    view.h = h;
    view.s = s;
    const fw = w + BLEED * 2;
    const fh = h + BLEED * 2;
    if (Math.abs(app.screen.width - fw) > 0.5 || Math.abs(app.screen.height - fh) > 0.5) {
      app.renderer.resize(fw, fh);
      resetMotes();
    }
    app.canvas.style.left = panel.style.left || "0px";
    app.canvas.style.top = panel.style.top || "0px";
    app.canvas.style.transformOrigin = "0 0";
    app.canvas.style.transform = `scale(${s}) translate(${-BLEED}px, ${-BLEED}px)`;
    app.canvas.style.opacity = panel.style.opacity || "1";
    app.canvas.style.display = state.glass ? "block" : "none";
  }

  function drawPrism(g, t, amp, a, b) {
    const w = view.w + BLEED * 2;
    const h = view.h + BLEED * 2;
    const y = ((t * 70) % (h + 40)) - 20;
    g.rect(0, y, w, 3).fill({ color: a, alpha: 0.18 * amp });
    g.rect(0, h - y, w, 2).fill({ color: b, alpha: 0.12 * amp });
    g.roundRect(BLEED - 6, BLEED - 6, view.w + 12, view.h + 12, 20).stroke({
      width: 2,
      color: a,
      alpha: 0.22 * amp,
    });
  }

  function drawEdge(g, amp, a) {
    g.rect(BLEED - 2, BLEED, 8, view.h).fill({ color: a, alpha: 0.35 * amp });
    g.rect(BLEED - 8, BLEED, 4, view.h).fill({ color: a, alpha: 0.12 * amp });
  }

  function drawCrt(g, t, amp, a) {
    const w = view.w + BLEED * 2;
    const h = view.h + BLEED * 2;
    const y = ((t * 90) % (h + 60)) - 30;
    g.rect(BLEED, y, view.w, 10).fill({ color: a, alpha: 0.08 * amp });
    g.rect(BLEED - 2, BLEED - 2, view.w + 4, view.h + 4).stroke({
      width: 2,
      color: a,
      alpha: 0.28 * amp,
    });
    for (let i = 0; i < 18; i++) {
      g.rect(BLEED + Math.random() * view.w, BLEED + Math.random() * view.h, 2, 1).fill({
        color: a,
        alpha: 0.15 * amp,
      });
    }
  }

  function drawBlocks(g, t, amp, a, b) {
    const step = Math.floor(t * (2 + (Number(state.pulse) || 0) * 0.4)) % 2;
    const flash = step ? amp : amp * 0.35;
    g.rect(BLEED - 14, BLEED, 10, view.h).fill({ color: a, alpha: 0.85 * flash });
    g.rect(BLEED + view.w - 18, BLEED + 8, 22, 22).fill({ color: b, alpha: 0.4 * amp });
    g.rect(BLEED + view.w - 40, BLEED + view.h - 28, 36, 12).fill({ color: a, alpha: 0.5 * flash });
  }

  function drawBurst(g, t, amp, a, b) {
    const cx = BLEED + view.w - 18;
    const cy = BLEED + 16;
    for (let i = 0; i < 8; i++) {
      const ang = t * 0.6 + i * (Math.PI / 4);
      const r = 10 + (i % 2) * 8;
      g.circle(cx + Math.cos(ang) * r, cy + Math.sin(ang) * r, 2).fill({
        color: i % 2 ? a : b,
        alpha: 0.55 * amp,
      });
    }
    const step = 14;
    for (let y = BLEED + 8; y < BLEED + view.h; y += step) {
      for (let x = BLEED + 8; x < BLEED + view.w; x += step) {
        const n = 0.5 + 0.5 * Math.sin(x * 0.04 + y * 0.03 + t);
        if (n < 0.62) continue;
        g.circle(x, y, n * 2.2).fill({ color: a, alpha: 0.12 * amp * n });
      }
    }
  }

  function drawGold(g, t, amp, a) {
    const flick = 0.55 + 0.45 * Math.abs(Math.sin(t * 6));
    g.roundRect(BLEED - 4, BLEED - 4, view.w + 8, view.h + 8, 4).stroke({
      width: 2,
      color: a,
      alpha: 0.25 * amp * flick,
    });
  }

  function paint(dt) {
    if (!ready || !gfx) return;
    clock += dt;
    const amp = intensity();
    const skin = skinOf();
    const w = view.w + BLEED * 2;
    const h = view.h + BLEED * 2;
    gfx.clear();
    if (!state.glass || amp <= 0) {
      sprites.forEach((spr) => {
        spr.visible = false;
      });
      return;
    }

    if (skin.mode === "prism") drawPrism(gfx, clock, amp, skin.a, skin.b);
    else if (skin.mode === "edge") drawEdge(gfx, amp, skin.a);
    else if (skin.mode === "crt") drawCrt(gfx, clock, amp, skin.a);
    else if (skin.mode === "blocks") drawBlocks(gfx, clock, amp, skin.a, skin.b);
    else if (skin.mode === "burst") drawBurst(gfx, clock, amp, skin.a, skin.b);
    else if (skin.mode === "gold") drawGold(gfx, clock, amp, skin.a);

    for (let i = 0; i < motes.length; i++) {
      const p = motes[i];
      p.x += p.vx * dt;
      p.y += p.vy * dt;
      if (skin.mode === "prism" || skin.mode === "gold" || skin.mode === "bokeh") {
        p.x += Math.sin(clock + p.phase) * 8 * dt;
        p.y += Math.cos(clock * 0.7 + p.phase) * 8 * dt;
      }
      if (p.x < -10) p.x = w + 8;
      if (p.x > w + 10) p.x = -8;
      if (p.y < -10) p.y = h + 8;
      if (p.y > h + 10) p.y = -8;
      const color = i % 2 ? skin.b : skin.a;
      if (skin.mode === "bubbles") {
        gfx.circle(p.x, p.y, p.r).stroke({ width: 1.5, color, alpha: 0.45 * amp });
      } else if (skin.mode === "rain") {
        gfx.rect(p.x, p.y, 1.5, 10 + (i % 5)).fill({ color, alpha: 0.55 * amp });
      } else if (skin.mode === "ember") {
        gfx.circle(p.x, p.y, p.r * 0.6).fill({ color, alpha: 0.7 * amp });
      } else if (skin.mode === "edge") {
        gfx.rect(p.x, p.y, 3, 8).fill({ color: skin.a, alpha: 0.5 * amp });
      } else if (skin.mode === "blocks") {
        gfx.rect(p.x, p.y, 6, 6).fill({ color, alpha: 0.35 * amp });
      } else if (skin.mode !== "bokeh") {
        gfx.circle(p.x, p.y, p.r * 0.45).fill({ color, alpha: 0.4 * amp });
      }
    }

    sprites.forEach((spr, i) => {
      const soft = skin.mode === "bokeh" || skin.mode === "prism";
      spr.visible = soft;
      if (!spr.visible) return;
      const p = motes[i];
      if (!p) {
        spr.visible = false;
        return;
      }
      spr.x = p.x;
      spr.y = p.y;
      spr.tint = i % 2 ? skin.b : skin.a;
      spr.alpha = amp * (skin.mode === "bokeh" ? 0.45 : 0.28);
      spr.scale.set((skin.mode === "bokeh" ? 1.6 : 0.9) + Math.sin(clock + p.phase) * 0.2);
    });
  }

  function syncTicker() {
    if (!app) return;
    if (wantTicker()) app.ticker.start();
    else {
      app.ticker.stop();
      paint(0);
    }
  }

  function applySet() {
    if (!ready) return;
    layoutCanvas();
    if (lastTheme !== state.theme) {
      lastTheme = state.theme;
      resetMotes();
    }
    seedSprites();
    syncTicker();
    paint(0);
  }

  function init(hostEl) {
    if (app || typeof PIXI === "undefined" || !hostEl) return;
    panel = hostEl.querySelector("#songInfo") || document.getElementById("songInfo");
    if (!panel) return;
    reduced = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
    app = new PIXI.Application();
    app
      .init({
        width: BASE_W + BLEED * 2,
        height: 200,
        backgroundAlpha: 0,
        antialias: false,
        autoDensity: true,
        resolution: Math.min(window.devicePixelRatio || 1, 2),
        preference: "webgl",
      })
      .then(function () {
        app.canvas.id = "overlayFx";
        app.canvas.setAttribute("aria-hidden", "true");
        hostEl.insertBefore(app.canvas, panel);
        gfx = new PIXI.Graphics();
        app.stage.addChild(gfx);
        orbTex = makeOrbTexture();
        ready = true;
        applySet();
        app.ticker.add(function (ticker) {
          const dt = Math.min(ticker.deltaMS / 1000, 0.05);
          layoutCanvas();
          paint(dt);
        });
        syncTicker();
        if (typeof ResizeObserver === "function") {
          new ResizeObserver(function () {
            layoutCanvas();
            if (!wantTicker()) paint(0);
          }).observe(panel);
        }
      })
      .catch(function () {
        if (app && app.canvas && app.canvas.parentNode) app.canvas.parentNode.removeChild(app.canvas);
        app = null;
        ready = false;
      });
  }

  window.JB2OverlayFx = {
    init: init,
    set: function (opts) {
      if (!opts) return;
      if (opts.theme) state.theme = opts.theme;
      if (opts.pulse != null) state.pulse = opts.pulse;
      if (opts.playing != null) state.playing = Boolean(opts.playing);
      if (opts.glass != null) state.glass = Boolean(opts.glass);
      applySet();
    },
  };
})();

(function () {
  var stage = document.getElementById("stage");
  var fallback = document.getElementById("logoFallback");
  var reduced = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
  var CYAN = 0x3ec8ff;
  var ORANGE = 0xff7a1a;
  var playBtn = document.getElementById("deckPlay");
  var playIcon = document.getElementById("deckPlayIcon");
  var trackSel = document.getElementById("deckTrack");
  var audio = document.getElementById("deckAudio");
  var vol = document.getElementById("deckVol");
  var volWrap = document.getElementById("deckVolWrap");
  var volBtn = document.getElementById("deckVolBtn");
  // playing gates the visualizer. bins stay allocated after the first click.
  var sound = { playing: false, bins: null, analyser: null, kick: 0, bass: 0, bassAvg: 0.2, file: "" };
  var audioCtx = null;
  var playGen = 0;
  var logoFade = 1;
  var fadeLogo = null;

  function heroFade() {
    var h = window.innerHeight || 1;
    var f = 1 - Math.min(1, Math.max(0, window.scrollY / h));
    if (reduced) f = window.scrollY >= h * 0.85 ? 0 : 1;
    return f;
  }

  function syncLogoFade() {
    logoFade = heroFade();
    if (fadeLogo) fadeLogo();
    if (!fallback || fallback.classList.contains("is-hidden")) return;
    fallback.style.opacity = String(logoFade);
    fallback.style.visibility = logoFade < 0.02 ? "hidden" : "";
  }

  window.addEventListener("scroll", syncLogoFade, { passive: true });
  syncLogoFade();

  function setPlaying(on) {
    sound.playing = on;
    if (!playBtn || !playIcon) return;
    playBtn.setAttribute("aria-label", on ? "Pause" : "Play");
    playIcon.setAttribute("href", on ? "#player-pause" : "#player-play");
  }

  function arm() {
    if (audioCtx || !audio) return;
    var AC = window.AudioContext || window.webkitAudioContext;
    if (!AC) return;
    audioCtx = new AC();
    var source = audioCtx.createMediaElementSource(audio);
    var analyser = audioCtx.createAnalyser();
    analyser.fftSize = 256;
    analyser.smoothingTimeConstant = 0.4;
    source.connect(analyser);
    analyser.connect(audioCtx.destination);
    sound.analyser = analyser;
    sound.bins = new Uint8Array(analyser.frequencyBinCount);
  }

  function stop() {
    playGen += 1;
    if (audio) audio.pause();
    setPlaying(false);
    sound.kick = 0;
    sound.bass = 0;
  }

  function playSelected() {
    var gen;
    if (!audio || !trackSel) return;
    if (sound.file !== trackSel.value) {
      audio.src = trackSel.value;
      sound.file = trackSel.value;
      sound.bassAvg = 0.2;
    }
    arm();
    if (audioCtx) audioCtx.resume();
    gen = ++playGen;
    // play() stays in the click turn. A resumed context promise can miss the gesture.
    audio
      .play()
      .then(function () {
        if (gen !== playGen) return;
        setPlaying(true);
      })
      .catch(function () {
        if (gen !== playGen) return;
        setPlaying(false);
      });
  }

  if (audio && vol) audio.volume = Number(vol.value);
  if (vol && audio) {
    vol.addEventListener("input", function () {
      audio.volume = Number(vol.value);
    });
  }
  if (volWrap && volBtn) {
    volBtn.addEventListener("click", function (e) {
      e.stopPropagation();
      var open = !volWrap.classList.contains("is-open");
      volWrap.classList.toggle("is-open", open);
      volBtn.setAttribute("aria-expanded", open ? "true" : "false");
    });
    document.addEventListener("click", function (e) {
      if (volWrap.contains(e.target)) return;
      volWrap.classList.remove("is-open");
      volBtn.setAttribute("aria-expanded", "false");
    });
  }

  if (trackSel) {
    var opts = Array.prototype.slice.call(trackSel.options);
    var i;
    var j;
    var tmp;
    for (i = opts.length - 1; i > 0; i--) {
      j = Math.floor(Math.random() * (i + 1));
      tmp = opts[i];
      opts[i] = opts[j];
      opts[j] = tmp;
    }
    for (i = 0; i < opts.length; i++) trackSel.appendChild(opts[i]);
    trackSel.selectedIndex = 0;
  }

  if (playBtn && audio && trackSel) {
    playBtn.addEventListener("click", function () {
      if (sound.playing) {
        stop();
        return;
      }
      playSelected();
    });
    trackSel.addEventListener("change", function () {
      var was = sound.playing;
      stop();
      sound.file = "";
      if (was) playSelected();
    });
    audio.addEventListener("ended", function () {
      if (!trackSel.options.length) {
        stop();
        return;
      }
      trackSel.selectedIndex = (trackSel.selectedIndex + 1) % trackSel.options.length;
      sound.file = "";
      playSelected();
    });
  }

  function sample() {
    if (!sound.playing || !sound.analyser) return;
    sound.analyser.getByteFrequencyData(sound.bins);
    var bass = 0;
    var n = Math.min(4, sound.bins.length);
    var b;
    for (b = 0; b < n; b++) bass += sound.bins[b];
    bass /= n * 255;
    sound.bass = bass;
    if (bass > sound.bassAvg * 1.35 && bass > 0.28) sound.kick = 1;
    else sound.kick *= 0.85;
    sound.bassAvg = sound.bassAvg * 0.9 + bass * 0.1;
  }

  // One slice per bar, 1..~11kHz. Log mapping stacked the left on one bin
  // and parked the last three on empty air.
  function bandLevel(i, count) {
    var bins = sound.bins;
    var n = bins.length;
    var maxBin = Math.max(count + 1, Math.floor(n * 0.55));
    var a = 1 + Math.floor((maxBin - 1) * i / count);
    var c = 1 + Math.floor((maxBin - 1) * (i + 1) / count);
    var k;
    var peak = 0;
    if (c <= a) c = a + 1;
    if (c > n) c = n;
    for (k = a; k < c; k++) {
      if (bins[k] > peak) peak = bins[k];
    }
    return peak / 255;
  }

  if (!stage || typeof PIXI === "undefined") return;

  function canvasTexture(draw) {
    var canvas = document.createElement("canvas");
    draw(canvas);
    var texture = PIXI.Texture.from(canvas);
    if (texture.source) texture.source.scaleMode = "nearest";
    return texture;
  }

  var vignetteTex = canvasTexture(function (canvas) {
    canvas.width = 256;
    canvas.height = 256;
    var g = canvas.getContext("2d");
    var glow = g.createRadialGradient(128, 100, 10, 128, 118, 150);
    glow.addColorStop(0, "rgba(62,200,255,0.35)");
    glow.addColorStop(0.35, "rgba(255,122,26,0.08)");
    glow.addColorStop(0.7, "rgba(7,8,11,0)");
    glow.addColorStop(1, "rgba(7,8,11,0.2)");
    g.fillStyle = glow;
    g.fillRect(0, 0, 256, 256);
  });

  var scanTex = canvasTexture(function (canvas) {
    canvas.width = 2;
    canvas.height = 4;
    var g = canvas.getContext("2d");
    g.fillStyle = "rgba(0,0,0,0.55)";
    g.fillRect(0, 3, 2, 1);
  });

  var app = new PIXI.Application();

  app
    .init({
      background: "#07080b",
      resizeTo: window,
      antialias: false,
      autoDensity: true,
      resolution: Math.min(window.devicePixelRatio || 1, 2),
    })
    .then(function () {
      app.canvas.setAttribute("aria-hidden", "true");
      stage.appendChild(app.canvas);
      return PIXI.Assets.load("img/jukeboks.png");
    })
    .then(function (texture) {
      if (texture.source) texture.source.scaleMode = "nearest";

      var sky = new PIXI.Graphics();
      var grid = new PIXI.Graphics();
      var glow = new PIXI.Graphics();
      var bars = new PIXI.Graphics();
      var sparks = new PIXI.Graphics();
      var sweep = new PIXI.Graphics();
      var vignette = new PIXI.Sprite(vignetteTex);
      var scan = new PIXI.TilingSprite({ texture: scanTex, width: 10, height: 10 });
      scan.alpha = 0.35;

      function ghost(tint, alpha) {
        var sprite = new PIXI.Sprite(texture);
        sprite.anchor.set(0.5);
        sprite.tint = tint;
        sprite.alpha = alpha;
        sprite.blendMode = "add";
        sprite.roundPixels = true;
        return sprite;
      }

      var ghostCyan = ghost(CYAN, 0.55);
      var ghostOrange = ghost(ORANGE, 0.4);
      var logo = new PIXI.Sprite(texture);
      logo.anchor.set(0.5);
      logo.roundPixels = true;
      fadeLogo = function () {
        logo.alpha = logoFade;
        ghostCyan.alpha = 0.55 * logoFade;
        ghostOrange.alpha = 0.4 * logoFade;
        glow.alpha = logoFade;
      };

      app.stage.addChild(sky);
      app.stage.addChild(grid);
      app.stage.addChild(vignette);
      app.stage.addChild(glow);
      app.stage.addChild(ghostCyan);
      app.stage.addChild(ghostOrange);
      app.stage.addChild(logo);
      app.stage.addChild(bars);
      app.stage.addChild(sparks);
      app.stage.addChild(sweep);
      app.stage.addChild(scan);

      var notes = [];
      var shown = [];
      var avg = [];
      var stars = [];
      for (var n = 0; n < 42; n++) {
        notes.push({ x: 0, y: 0, v: 40, life: 0, color: CYAN });
      }
      for (var st = 0; st < 50; st++) {
        stars.push({
          x: Math.random(),
          y: Math.random() * 0.55,
          size: st % 7 === 0 ? 3 : 2,
          phase: Math.random() * Math.PI * 2,
          color: st % 5 === 0 ? ORANGE : CYAN,
        });
      }
      var city = [];
      var cx = 0;
      while (cx < 1) {
        var bw = 0.01 + Math.random() * 0.028;
        var mid = cx > 0.38 && cx < 0.62;
        city.push({
          x: cx,
          w: bw,
          h: mid ? 0.018 + Math.random() * 0.03 : 0.05 + Math.random() * 0.14,
          lit: !mid && Math.random() > 0.4,
        });
        cx += bw + 0.003;
      }
      var noteCursor = 0;
      var layout = { scale: 1, x: 0, y: 0 };

      function place() {
        var w = app.screen.width;
        var h = app.screen.height;
        layout.scale = Math.min(w * 0.52, h * 0.4, 440) / texture.width;
        layout.x = w / 2;
        layout.y = h * 0.3;
        vignette.width = w;
        vignette.height = h;
        scan.width = w;
        scan.height = h;
      }

      function stamp(sprite, scale, x, y) {
        sprite.scale.set(scale);
        sprite.x = x;
        sprite.y = y;
      }

      function spawn(x, y, color) {
        var note = notes[noteCursor];
        noteCursor = (noteCursor + 1) % notes.length;
        note.x = x;
        note.y = y;
        note.v = 70 + Math.random() * 140;
        note.life = 1;
        note.color = color;
      }

      function draw(time, dt) {
        var w = app.screen.width;
        var h = app.screen.height;
        var live = sound.playing && sound.bins;
        // Idle kick is ~120 bpm. A playing track uses the bass onset instead.
        var kick = live
          ? Math.min(1, Math.max(0, sound.bass - sound.bassAvg) * 5 + sound.kick)
          : Math.pow(Math.max(0, Math.sin(time * Math.PI * 2)), 8);
        var scale = layout.scale * (1 + kick * 0.09);
        var y = layout.y + Math.sin(time * 1.7) * 12;
        var split = 3 + kick * 10 + Math.sin(time * 9) * 2;

        stamp(logo, scale, layout.x, y);
        stamp(ghostCyan, scale, layout.x - split, y);
        stamp(ghostOrange, scale, layout.x + split, y + 1);
        logo.alpha = logoFade;
        ghostCyan.alpha = 0.55 * logoFade;
        ghostOrange.alpha = 0.4 * logoFade;
        glow.alpha = logoFade;

        var logoW = texture.width * scale;
        var logoH = texture.height * scale;
        glow.clear();
        glow.circle(layout.x, y, logoW * (0.55 + kick * 0.08)).fill({ color: CYAN, alpha: 0.1 });
        glow.circle(layout.x, y, logoW * 0.85).fill({ color: ORANGE, alpha: 0.05 + kick * 0.06 });
        glow
          .circle(layout.x, y, logoW * 0.62)
          .stroke({ width: 2, color: ORANGE, alpha: 0.35 + kick * 0.5 });
        glow
          .circle(layout.x, y, logoW * (0.74 + Math.sin(time * 2.2) * 0.03))
          .stroke({ width: 1, color: CYAN, alpha: 0.45 });
        for (var o = 0; o < 12; o++) {
          var ang = time * 1.5 + o * ((Math.PI * 2) / 12);
          var rad = logoW * 0.7;
          var px = layout.x + Math.cos(ang) * rad - 2;
          var py = y + Math.sin(ang) * rad * 0.42 - 2;
          glow.rect(px, py, o % 2 ? 6 : 3, o % 2 ? 6 : 3).fill({
            color: o % 2 ? ORANGE : CYAN,
            alpha: 0.9,
          });
        }

        var horizon = y + logoH * 0.55;
        sky.clear();
        for (var band = 0; band < 8; band++) {
          var t0 = band / 8;
          sky.rect(0, t0 * horizon, w, horizon / 8 + 1).fill({
            color: CYAN,
            alpha: 0.055 * (1 - t0),
          });
        }
        sky.rect(0, horizon - 2, w, 3).fill({ color: ORANGE, alpha: 0.12 });
        for (var si = 0; si < stars.length; si++) {
          var star = stars[si];
          var tw = reduced
            ? 0.4
            : 0.22 + 0.32 * (0.5 + 0.5 * Math.sin(time * 2.4 + star.phase)) + kick * 0.18;
          sky.rect(star.x * w, star.y * h, star.size, star.size).fill({
            color: star.color,
            alpha: tw,
          });
        }
        for (var ci = 0; ci < city.length; ci++) {
          var block = city[ci];
          var bx = block.x * w;
          var bwpx = Math.max(4, block.w * w);
          var bh = block.h * h;
          sky.rect(bx, horizon - bh, bwpx, bh).fill({ color: 0x05070c, alpha: 1 });
          if (block.lit) {
            sky.rect(bx + 2, horizon - bh + 5, 2, 2).fill({
              color: ci % 3 === 0 ? ORANGE : CYAN,
              alpha: 0.4 + kick * 0.45,
            });
          }
        }
        grid.clear();
        var scroll = reduced ? 0.4 : (time * 0.55) % 1;
        for (var row = 0; row < 16; row++) {
          var z = (row + scroll) / 16;
          var gy = horizon + z * z * (h * 0.72 - horizon);
          if (gy > h) continue;
          grid.moveTo(0, gy).lineTo(w, gy).stroke({
            width: 1,
            color: row % 4 === 0 ? ORANGE : CYAN,
            alpha: 0.05 + z * 0.28,
          });
        }
        for (var col = -8; col <= 8; col++) {
          grid.moveTo(w / 2, horizon).lineTo(w / 2 + col * w * 0.14, h).stroke({
            width: 1,
            color: CYAN,
            alpha: 0.16,
          });
        }

        var count = Math.max(20, Math.min(56, Math.floor(w / 16)));
        var gap = 4;
        var barW = Math.max(4, (w * 0.86) / count - gap);
        var total = count * barW + (count - 1) * gap;
        var x0 = (w - total) / 2;
        var base = Math.min(horizon + 8, h * 0.62);
        bars.clear();
        for (var i = 0; i < count; i++) {
          var raw = live
            ? bandLevel(i, count)
            : Math.abs(Math.sin(time * 7.5 + i * 0.48) * Math.sin(time * 1.8 + i * 0.27));
          if (avg[i] == null) avg[i] = raw;
          avg[i] += (raw - avg[i]) * 0.06;
          // Left keeps the bass level. Right keeps the twitch. Kick lifts the low end.
          var t = count > 1 ? i / (count - 1) : 0;
          var hit = Math.max(0, raw - avg[i]);
          var wave = live
            ? Math.max(0, Math.min(1, raw * (0.85 - 0.5 * t) + hit * (4 + 8 * t) + (1 - t) * (1 - t) * sound.kick * 0.4))
            : raw;
          var prev = shown[i] || 0;
          shown[i] = prev + (wave - prev) * (wave > prev ? 0.7 : 0.2);
          wave = shown[i];
          var bh = 2 + wave * h * 0.13 + kick * (i % 3 === 0 ? 10 : 3);
          var color = i % 4 === 0 ? ORANGE : CYAN;
          var x = x0 + i * (barW + gap);
          bars.rect(x, base - bh, barW, bh).fill({ color: color, alpha: 0.95 });
          bars.rect(x, base + 4, barW, bh * 0.45).fill({ color: color, alpha: 0.18 });
          if (!reduced && wave > 0.82 && Math.random() < 0.35) spawn(x, base - bh, color);
        }

        sparks.clear();
        for (var s = 0; s < notes.length; s++) {
          var note = notes[s];
          if (note.life <= 0) continue;
          note.y -= note.v * dt;
          note.life -= dt * 0.7;
          if (note.life <= 0) continue;
          sparks.rect(note.x, note.y, 4, 4).fill({ color: note.color, alpha: note.life });
        }

        sweep.clear();
        if (reduced) return;
        var sy = ((time * 90) % (h + 80)) - 40;
        sweep.rect(0, sy, w, 3).fill({ color: CYAN, alpha: 0.22 });
        sweep.rect(0, h - sy, w, 2).fill({ color: ORANGE, alpha: 0.12 });
        sweep.rect(0, 0, w, h).fill({ color: kick > 0.6 ? ORANGE : CYAN, alpha: kick * 0.07 });
        scan.tilePosition.y = time * 40;
      }

      place();
      draw(0.4, 0);
      if (fallback) fallback.classList.add("is-hidden");
      app.renderer.on("resize", function () {
        place();
        if (reduced) draw(0.4, 0);
      });

      if (reduced) return;

      var time = 0.4;
      app.ticker.add(function (ticker) {
        var dt = Math.min(ticker.deltaMS / 1000, 0.05);
        time += dt;
        sample();
        draw(time, dt);
      });
    })
    .catch(function () {
      if (app.canvas && app.canvas.parentNode) app.canvas.parentNode.removeChild(app.canvas);
    });
})();

(function () {
  var stage = document.getElementById("stage");
  var fallback = document.getElementById("logoFallback");
  var reduced = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
  var CYAN = 0x3ec8ff;
  var ORANGE = 0xff7a1a;

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
      for (var n = 0; n < 42; n++) {
        notes.push({ x: 0, y: 0, v: 40, life: 0, color: CYAN });
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
        // Kick at ~120 bpm. High power keeps it a hit, not a swell.
        var kick = Math.pow(Math.max(0, Math.sin(time * Math.PI * 2)), 8);
        var scale = layout.scale * (1 + kick * 0.09);
        var y = layout.y + Math.sin(time * 1.7) * 12;
        var split = 3 + kick * 10 + Math.sin(time * 9) * 2;

        stamp(logo, scale, layout.x, y);
        stamp(ghostCyan, scale, layout.x - split, y);
        stamp(ghostOrange, scale, layout.x + split, y + 1);

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
          var wave = Math.abs(Math.sin(time * 7.5 + i * 0.48) * Math.sin(time * 1.8 + i * 0.27));
          var bh = 10 + wave * h * 0.2 + kick * (i % 3 === 0 ? 36 : 12);
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
        draw(time, dt);
      });
    })
    .catch(function () {
      if (app.canvas && app.canvas.parentNode) app.canvas.parentNode.removeChild(app.canvas);
    });
})();

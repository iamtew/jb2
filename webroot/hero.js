(function () {
  var stage = document.getElementById("stage");
  var logoEl = document.getElementById("logo");
  var heroEl = document.querySelector(".hero");
  var cardsEl = document.querySelector(".cards");
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
    var glow = g.createRadialGradient(90, 70, 8, 110, 90, 160);
    glow.addColorStop(0, "rgba(62,200,255,0.28)");
    glow.addColorStop(0.4, "rgba(255,122,26,0.07)");
    glow.addColorStop(0.75, "rgba(7,8,11,0)");
    glow.addColorStop(1, "rgba(7,8,11,0.35)");
    g.fillStyle = glow;
    g.fillRect(0, 0, 256, 256);
  });

  var scanTex = canvasTexture(function (canvas) {
    canvas.width = 2;
    canvas.height = 4;
    var g = canvas.getContext("2d");
    g.fillStyle = "rgba(0,0,0,0.5)";
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
      return PIXI.Assets.load("/img/jukeboks.png");
    })
    .then(function (texture) {
      if (texture.source) texture.source.scaleMode = "nearest";

      var sky = new PIXI.Graphics();
      var glow = new PIXI.Graphics();
      var ribbon = new PIXI.Graphics();
      var motesGfx = new PIXI.Graphics();
      var sweep = new PIXI.Graphics();
      var vignette = new PIXI.Sprite(vignetteTex);
      var scan = new PIXI.TilingSprite({ texture: scanTex, width: 10, height: 10 });
      scan.alpha = 0.32;

      function ghost(tint, alpha) {
        var sprite = new PIXI.Sprite(texture);
        sprite.anchor.set(0.5);
        sprite.tint = tint;
        sprite.alpha = alpha;
        sprite.blendMode = "add";
        sprite.roundPixels = true;
        return sprite;
      }

      var ghostCyan = ghost(CYAN, 0.5);
      var ghostOrange = ghost(ORANGE, 0.35);
      var logo = new PIXI.Sprite(texture);
      logo.anchor.set(0.5);
      logo.roundPixels = true;

      app.stage.addChild(vignette);
      app.stage.addChild(sky);
      app.stage.addChild(ribbon);
      app.stage.addChild(glow);
      app.stage.addChild(ghostCyan);
      app.stage.addChild(ghostOrange);
      app.stage.addChild(logo);
      app.stage.addChild(motesGfx);
      app.stage.addChild(sweep);
      app.stage.addChild(scan);

      var stars = [];
      var s;
      for (s = 0; s < 90; s++) {
        stars.push({
          x: Math.random(),
          y: Math.random(),
          size: s % 8 === 0 ? 4 : s % 3 === 0 ? 3 : 2,
          phase: Math.random() * Math.PI * 2,
          color: s % 4 === 0 ? ORANGE : CYAN,
        });
      }

      var motes = [];
      var m;
      for (m = 0; m < 28; m++) {
        motes.push({
          a: Math.random() * Math.PI * 2,
          r: 0.4 + Math.random() * 0.7,
          spin: 0.15 + Math.random() * 0.35,
          size: m % 3 === 0 ? 4 : 2,
          color: m % 2 ? ORANGE : CYAN,
        });
      }

      var layout = { x: 0, y: 0, scale: 1, ribbonY: 0 };

      function place() {
        var w = app.screen.width;
        var h = app.screen.height;
        vignette.width = w;
        vignette.height = h;
        scan.width = w;
        scan.height = h;
        if (!logoEl) return;
        var box = logoEl.getBoundingClientRect();
        layout.x = box.left + box.width / 2;
        layout.y = box.top + box.height / 2;
        layout.scale = box.width / texture.width;
        var floor = h * 0.84;
        if (cardsEl) {
          var cards = cardsEl.getBoundingClientRect();
          floor = Math.min(h - 28, Math.max(cards.bottom + 22, h * 0.78));
        } else if (heroEl) {
          floor = heroEl.getBoundingClientRect().bottom + 28;
        }
        layout.ribbonY = floor;
      }

      function stamp(sprite, scale, x, y) {
        sprite.scale.set(scale);
        sprite.x = x;
        sprite.y = y;
      }

      function draw(time) {
        place();
        var w = app.screen.width;
        var h = app.screen.height;
        var kick = reduced ? 0.15 : Math.pow(Math.max(0, Math.sin(time * Math.PI * 1.6)), 10);
        var scale = layout.scale * (1 + kick * 0.06);
        var y = layout.y + (reduced ? 0 : Math.sin(time * 1.4) * 5);
        var split = 2 + kick * 6 + Math.sin(time * 7) * 1.4;

        stamp(logo, scale, layout.x, y);
        stamp(ghostCyan, scale, layout.x - split, y);
        stamp(ghostOrange, scale, layout.x + split, y + 1);

        var logoW = texture.width * scale;
        glow.clear();
        glow.circle(layout.x, y, logoW * (0.58 + kick * 0.06)).fill({ color: CYAN, alpha: 0.09 });
        glow.circle(layout.x, y, logoW * 0.9).fill({ color: ORANGE, alpha: 0.04 + kick * 0.05 });
        glow.circle(layout.x, y, logoW * 0.66).stroke({
          width: 2,
          color: ORANGE,
          alpha: 0.32 + kick * 0.4,
        });
        glow.circle(layout.x, y, logoW * (0.8 + Math.sin(time * 1.8) * 0.03)).stroke({
          width: 1,
          color: CYAN,
          alpha: 0.4,
        });
        var o;
        for (o = 0; o < 10; o++) {
          var ang = time * 1.1 + o * ((Math.PI * 2) / 10);
          var rad = logoW * 0.74;
          glow.rect(
            layout.x + Math.cos(ang) * rad - 2,
            y + Math.sin(ang) * rad * 0.55 - 2,
            o % 2 ? 5 : 3,
            o % 2 ? 5 : 3
          ).fill({
            color: o % 2 ? ORANGE : CYAN,
            alpha: 0.85,
          });
        }

        sky.clear();
        for (s = 0; s < stars.length; s++) {
          var star = stars[s];
          var tw = reduced
            ? 0.4
            : 0.28 + 0.4 * (0.5 + 0.5 * Math.sin(time * 2.1 + star.phase)) + kick * 0.18;
          sky.rect(star.x * w, star.y * h, star.size, star.size).fill({
            color: star.color,
            alpha: tw,
          });
        }

        motesGfx.clear();
        for (m = 0; m < motes.length; m++) {
          var mote = motes[m];
          var mr = logoW * (0.9 + mote.r);
          var ma = mote.a + time * mote.spin;
          motesGfx.rect(
            layout.x + Math.cos(ma) * mr - mote.size / 2,
            y + Math.sin(ma) * mr * 0.48 - mote.size / 2,
            mote.size,
            mote.size
          ).fill({ color: mote.color, alpha: 0.35 + kick * 0.4 });
        }

        var count = Math.max(18, Math.min(48, Math.floor(w / 22)));
        var gap = 3;
        var barW = Math.max(3, (w * 0.72) / count - gap);
        var total = count * barW + (count - 1) * gap;
        var x0 = (w - total) / 2;
        var maxH = Math.max(12, h * 0.055);
        ribbon.clear();
        var i;
        for (i = 0; i < count; i++) {
          var wave = Math.abs(Math.sin(time * 2.4 + i * 0.38) * Math.sin(time * 0.9 + i * 0.17));
          var bh = 2 + wave * maxH + kick * (i % 4 === 0 ? 6 : 2);
          var x = x0 + i * (barW + gap);
          var color = i % 5 === 0 ? ORANGE : CYAN;
          ribbon.rect(x, layout.ribbonY - bh, barW, bh).fill({ color: color, alpha: 0.7 });
          ribbon.rect(x, layout.ribbonY + 2, barW, bh * 0.28).fill({ color: color, alpha: 0.16 });
        }

        sweep.clear();
        if (!reduced) {
          var sy = ((time * 70) % (h + 60)) - 30;
          sweep.rect(0, sy, w, 2).fill({ color: CYAN, alpha: 0.16 });
          sweep.rect(0, h - sy * 0.7, w, 2).fill({ color: ORANGE, alpha: 0.08 });
          sweep.rect(0, 0, w, h).fill({ color: kick > 0.5 ? ORANGE : CYAN, alpha: kick * 0.05 });
          scan.tilePosition.y = time * 28;
        }
      }

      place();
      draw(0.4);
      if (logoEl) logoEl.classList.add("is-hidden");
      app.renderer.on("resize", function () {
        place();
        if (reduced) draw(0.4);
      });

      if (reduced) return;

      var time = 0.4;
      app.ticker.add(function (ticker) {
        var dt = Math.min(ticker.deltaMS / 1000, 0.05);
        time += dt;
        draw(time);
      });
    })
    .catch(function () {
      if (app.canvas && app.canvas.parentNode) app.canvas.parentNode.removeChild(app.canvas);
    });
})();

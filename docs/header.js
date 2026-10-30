(function () {
  var stage = document.getElementById("headerStage");
  var reduced = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
  var CYAN = 0x3ec8ff;
  var ORANGE = 0xff7a1a;

  if (!stage || typeof PIXI === "undefined") return;

  function canvasTexture(draw, scaleMode) {
    var canvas = document.createElement("canvas");
    draw(canvas);
    var texture = PIXI.Texture.from(canvas);
    if (texture.source) texture.source.scaleMode = scaleMode || "nearest";
    return texture;
  }

  var scanTex = canvasTexture(function (canvas) {
    canvas.width = 2;
    canvas.height = 4;
    var g = canvas.getContext("2d");
    g.fillStyle = "rgba(0,0,0,0.5)";
    g.fillRect(0, 3, 2, 1);
  });

  var latticeTex = canvasTexture(function (canvas) {
    canvas.width = 20;
    canvas.height = 20;
    var g = canvas.getContext("2d");
    g.strokeStyle = "rgba(62,200,255,0.55)";
    g.lineWidth = 1;
    g.beginPath();
    g.moveTo(0, 0.5);
    g.lineTo(20, 0.5);
    g.moveTo(0.5, 0);
    g.lineTo(0.5, 20);
    g.stroke();
    g.strokeStyle = "rgba(255,122,26,0.28)";
    g.beginPath();
    g.moveTo(0, 10.5);
    g.lineTo(20, 10.5);
    g.stroke();
  });

  var app = new PIXI.Application();

  app
    .init({
      background: "#07080b",
      resizeTo: stage,
      antialias: false,
      autoDensity: true,
      resolution: Math.min(window.devicePixelRatio || 1, 2),
    })
    .then(function () {
      app.canvas.setAttribute("aria-hidden", "true");
      stage.appendChild(app.canvas);

      var sky = new PIXI.Graphics();
      var ribbon = new PIXI.Graphics();
      var sweep = new PIXI.Graphics();
      var lattice = new PIXI.TilingSprite({ texture: latticeTex, width: 10, height: 10 });
      lattice.alpha = 0.28;
      var scan = new PIXI.TilingSprite({ texture: scanTex, width: 10, height: 10 });
      scan.alpha = 0.3;

      app.stage.addChild(sky);
      app.stage.addChild(lattice);
      app.stage.addChild(ribbon);
      app.stage.addChild(sweep);
      app.stage.addChild(scan);

      var stars = [];
      var s;
      for (s = 0; s < 40; s++) {
        stars.push({
          x: Math.random(),
          y: Math.random(),
          size: s % 5 === 0 ? 2 : 1,
          phase: Math.random() * Math.PI * 2,
          color: s % 4 === 0 ? ORANGE : CYAN,
        });
      }

      function draw(time) {
        var w = app.screen.width;
        var h = app.screen.height;
        if (w < 2 || h < 2) return;
        lattice.width = w;
        lattice.height = h;
        scan.width = w;
        scan.height = h;

        var kick = reduced ? 0.12 : Math.pow(Math.max(0, Math.sin(time * Math.PI * 1.5)), 10);

        sky.clear();
        for (s = 0; s < stars.length; s++) {
          var star = stars[s];
          var tw = reduced
            ? 0.32
            : 0.22 + 0.4 * (0.5 + 0.5 * Math.sin(time * 2.3 + star.phase)) + kick * 0.2;
          sky.rect(star.x * w, star.y * h, star.size, star.size).fill({
            color: star.color,
            alpha: tw,
          });
        }

        var count = Math.max(36, Math.min(96, Math.floor(w / 12)));
        var gap = 2;
        var barW = Math.max(3, w / count - gap);
        var maxH = Math.max(10, h - 6);
        ribbon.clear();
        var i;
        for (i = 0; i < count; i++) {
          var wave = Math.abs(Math.sin(time * 2.1 + i * 0.22) * Math.sin(time * 0.85 + i * 0.11));
          var bh = 4 + wave * maxH * 0.82 + kick * (i % 5 === 0 ? 8 : 2);
          if (bh > maxH) bh = maxH;
          var x = i * (barW + gap);
          var color = i % 6 === 0 ? ORANGE : CYAN;
          ribbon.rect(x, h - bh, barW, bh).fill({ color: color, alpha: 0.42 + wave * 0.28 });
        }
        ribbon.rect(0, h - 2, w, 2).fill({ color: ORANGE, alpha: 0.35 + kick * 0.3 });
        ribbon.rect(0, 0, w, 1).fill({ color: CYAN, alpha: 0.22 });

        sweep.clear();
        if (!reduced) {
          var sx = ((time * 70) % (w + 50)) - 25;
          sweep.rect(sx, 0, 3, h).fill({ color: CYAN, alpha: 0.18 });
          sweep.rect(w - sx * 0.55, 0, 2, h).fill({ color: ORANGE, alpha: 0.12 });
          sweep.rect(0, 0, w, h).fill({ color: kick > 0.5 ? ORANGE : CYAN, alpha: kick * 0.06 });
          lattice.tilePosition.x = -time * 22;
          lattice.tilePosition.y = time * 8;
          scan.tilePosition.y = time * 24;
        }
      }

      draw(0.4);
      app.renderer.on("resize", function () {
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

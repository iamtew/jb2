(function () {
  var pages = [
    { id: "overview", title: "Overview", icon: "home" },
    { id: "running", title: "Running", icon: "player-play" },
    { id: "admin", title: "Admin", icon: "settings" },
    { id: "overlay", title: "Overlay", icon: "device-tv" },
    { id: "twitch", title: "Twitch", icon: "brand-twitch" },
    { id: "requests", title: "Requests", icon: "music" },
    { id: "seed", title: "Seed", icon: "plant" },
    { id: "http", title: "HTTP", icon: "api" },
    { id: "credits", title: "Credits", icon: "certificate" },
  ];

  var nav = document.getElementById("nav");
  var article = document.getElementById("doc");

  pages.forEach(function (page) {
    var link = document.createElement("a");
    link.href = "#" + page.id;
    var icon = document.createElementNS("http://www.w3.org/2000/svg", "svg");
    icon.setAttribute("class", "icon");
    icon.setAttribute("aria-hidden", "true");
    var use = document.createElementNS("http://www.w3.org/2000/svg", "use");
    use.setAttribute("href", "icons.svg#" + page.icon);
    icon.appendChild(use);
    var label = document.createElement("span");
    label.textContent = page.title;
    link.appendChild(icon);
    link.appendChild(label);
    link.dataset.id = page.id;
    nav.appendChild(link);
  });
  if (window.jb2IconsReady) window.jb2IconsReady();

  function known(id) {
    return pages.some(function (page) { return page.id === id; });
  }

  function currentId() {
    var id = (location.hash || "#overview").replace(/^#/, "");
    return known(id) ? id : "overview";
  }

  // Markdown links stay as sibling .md files so GitHub renders them.
  // On this page they become hash routes.
  function rewrite(root) {
    root.querySelectorAll("a[href]").forEach(function (link) {
      var href = link.getAttribute("href") || "";
      var match = href.match(/^(?:\.\/)?([a-z0-9-]+)\.md(#[a-z0-9-]*)?$/);
      if (match && known(match[1])) link.setAttribute("href", "#" + match[1]);
    });
  }

  function show() {
    var id = currentId();
    if ((location.hash || "#overview").replace(/^#/, "") !== id) {
      history.replaceState(null, "", "#" + id);
    }
    nav.querySelectorAll("a").forEach(function (link) {
      var on = link.dataset.id === id;
      if (on) link.setAttribute("aria-current", "page");
      else link.removeAttribute("aria-current");
    });
    var page = pages.filter(function (item) { return item.id === id; })[0];
    document.title = page.title + " — JB2";
    article.innerHTML = "<p class=\"muted\">Loading…</p>";
    fetch("md/" + id + ".md")
      .then(function (res) {
        if (!res.ok) throw new Error(String(res.status));
        return res.text();
      })
      .then(function (md) {
        article.innerHTML = marked.parse(md);
        rewrite(article);
      })
      .catch(function () {
        article.innerHTML = "<p>Could not load this page.</p>";
      });
  }

  window.addEventListener("hashchange", show);
  show();
})();

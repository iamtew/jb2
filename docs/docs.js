(function () {
  var pages = [
    { id: "quick-start", title: "Quick Start", icon: "download" },
    { id: "overview", title: "Overview", icon: "home", group: "Start" },
    { id: "running", title: "Running", icon: "player-play", group: "Start" },
    { id: "admin", title: "Control Panel", icon: "settings", group: "Surfaces" },
    { id: "overlay", title: "Overlay", icon: "device-tv", group: "Surfaces" },
    { id: "stage", title: "Overlay Stage", icon: "artboard", group: "Surfaces" },
    { id: "typography", title: "Typography", icon: "typography", group: "Surfaces" },
    { id: "twitch", title: "Twitch", icon: "brand-twitch", group: "Requests" },
    { id: "requests", title: "Requests", icon: "music", group: "Requests" },
    { id: "seed", title: "Seed", icon: "plant", group: "Requests" },
    { id: "http", title: "HTTP", icon: "api", group: "Build" },
    { id: "credits", title: "Credits", icon: "certificate", group: "About" },
  ];

  var nav = document.getElementById("nav");
  var article = document.getElementById("doc");
  var lastGroup = "";

  pages.forEach(function (page) {
    if (page.group && page.group !== lastGroup) {
      lastGroup = page.group;
      var heading = document.createElement("p");
      heading.className = "nav-group";
      heading.textContent = page.group;
      nav.appendChild(heading);
    }
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

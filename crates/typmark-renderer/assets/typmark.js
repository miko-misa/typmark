(function () {
  "use strict";

  function applyBoxAttributes() {
    var boxes = document.querySelectorAll(".TypMark-box");
    boxes.forEach(function (box) {
      var bg = box.getAttribute("data-bg");
      var titleBg = box.getAttribute("data-title-bg");
      var border = box.getAttribute("data-border-color");
      if (bg) {
        box.style.backgroundColor = bg;
      }
      if (titleBg) {
        var title = box.querySelector(".TypMark-box-title");
        if (title) {
          title.style.backgroundColor = titleBg;
        }
      }
      if (border) {
        box.style.borderColor = border;
      }
    });
  }

  function wireLineAnchors() {
    var lines = document.querySelectorAll(
      "figure.TypMark-codeblock .line[data-line]",
    );
    lines.forEach(function (line) {
      line.addEventListener("click", function () {
        var id = line.getAttribute("id");
        if (id) {
          history.replaceState(null, "", "#" + id);
        }
      });
    });
  }

  function setupRefScroll() {
    var refs = document.querySelectorAll("a.TypMark-ref");
    refs.forEach(function (link) {
      link.addEventListener("click", function () {
        var hash = link.getAttribute("href");
        if (!hash || hash.charAt(0) !== "#") {
          return;
        }
        var target = document.getElementById(hash.slice(1));
        if (!target) {
          return;
        }
        target.classList.add("TypMark-ref-target");
        setTimeout(function () {
          target.classList.remove("TypMark-ref-target");
        }, 1200);
      });
    });
  }

  function init() {
    applyBoxAttributes();
    wireLineAnchors();
    setupRefScroll();
  }

  if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", init);
  } else {
    init();
  }
})();

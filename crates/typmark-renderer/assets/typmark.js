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

  function setupMathScrollShadows() {
    var blocks = document.querySelectorAll(".TypMark-math-block");
    if (!blocks.length) {
      return;
    }

    function ensureScrollTarget(block) {
      for (var i = 0; i < block.children.length; i++) {
        var child = block.children[i];
        if (child.classList.contains("TypMark-math-block-scroll")) {
          return child;
        }
      }

      var wrapper = document.createElement("div");
      wrapper.className = "TypMark-math-block-scroll";
      while (block.firstChild) {
        wrapper.appendChild(block.firstChild);
      }
      block.appendChild(wrapper);
      block.classList.add("TypMark-math-block--scroll");
      return wrapper;
    }

    function updateBlock(block, scrollTarget) {
      var maxScroll = Math.max(
        0,
        scrollTarget.scrollWidth - scrollTarget.clientWidth,
      );
      if (maxScroll <= 0.5) {
        block.classList.remove("TypMark-scroll-left");
        block.classList.remove("TypMark-scroll-right");
        return;
      }

      var edgeThreshold = 0.5;
      block.classList.toggle(
        "TypMark-scroll-left",
        scrollTarget.scrollLeft > edgeThreshold,
      );
      block.classList.toggle(
        "TypMark-scroll-right",
        scrollTarget.scrollLeft < maxScroll - edgeThreshold,
      );
    }

    function watchBlock(block) {
      var scrollTarget = ensureScrollTarget(block);
      var rafId = 0;
      var onScroll = function () {
        if (rafId) return;
        rafId = window.requestAnimationFrame(function () {
          rafId = 0;
          updateBlock(block, scrollTarget);
        });
      };

      scrollTarget.addEventListener("scroll", onScroll, { passive: true });
      updateBlock(block, scrollTarget);
      setTimeout(function () {
        updateBlock(block, scrollTarget);
      }, 200);

      if (typeof ResizeObserver !== "undefined") {
        var resizeObserver = new ResizeObserver(function () {
          updateBlock(block, scrollTarget);
        });
        resizeObserver.observe(block);
        resizeObserver.observe(scrollTarget);
      }
    }

    blocks.forEach(watchBlock);

    window.addEventListener("resize", function () {
      blocks.forEach(function (block) {
        var scrollTarget = ensureScrollTarget(block);
        updateBlock(block, scrollTarget);
      });
    });
  }

  function init() {
    applyBoxAttributes();
    wireLineAnchors();
    setupRefScroll();
    setupMathScrollShadows();
  }

  if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", init);
  } else {
    init();
  }
})();

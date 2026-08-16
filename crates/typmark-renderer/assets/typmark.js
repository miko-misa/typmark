(() => {
  function applyBoxAttributes() {
    var boxes = document.querySelectorAll(".TypMark-box");
    boxes.forEach((box) => {
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
    lines.forEach((line) => {
      line.addEventListener("click", () => {
        var id = line.getAttribute("id");
        if (id) {
          history.replaceState(null, "", "#" + id);
        }
      });
    });
  }

  function setupRefScroll() {
    var refs = document.querySelectorAll("a.TypMark-ref");
    refs.forEach((link) => {
      link.addEventListener("click", () => {
        var hash = link.getAttribute("href");
        if (!hash || hash.charAt(0) !== "#") {
          return;
        }
        var target = document.getElementById(hash.slice(1));
        if (!target) {
          return;
        }
        target.classList.add("TypMark-ref-target");
        setTimeout(() => {
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
      var onScroll = () => {
        if (rafId) return;
        rafId = window.requestAnimationFrame(() => {
          rafId = 0;
          updateBlock(block, scrollTarget);
        });
      };

      scrollTarget.addEventListener("scroll", onScroll, { passive: true });
      updateBlock(block, scrollTarget);
      setTimeout(() => {
        updateBlock(block, scrollTarget);
      }, 200);

      if (typeof ResizeObserver !== "undefined") {
        var resizeObserver = new ResizeObserver(() => {
          updateBlock(block, scrollTarget);
        });
        resizeObserver.observe(block);
        resizeObserver.observe(scrollTarget);
      }
    }

    blocks.forEach(watchBlock);

    window.addEventListener("resize", () => {
      blocks.forEach((block) => {
        var scrollTarget = ensureScrollTarget(block);
        updateBlock(block, scrollTarget);
      });
    });
  }

  var SVG_PAINT_PROPERTIES = ["fill", "stroke", "stop-color"];
  var svgThemeObservers = [];
  var ACHROMATIC_EPSILON = 0.000004;
  // ponytail: heuristic ceiling calibrated for diagram neutrals; make this
  // configurable only if real documents show one threshold is insufficient.
  var NEUTRAL_CHROMA_CEILING = 0.08;

  function parseHexColor(value) {
    var normalized = value.trim().toLowerCase();
    if (normalized === "black") normalized = "#000000";
    if (normalized === "white") normalized = "#ffffff";
    if (normalized === "transparent") normalized = "#00000000";
    if (normalized.charAt(0) !== "#") return null;

    var hex = normalized.slice(1);
    if (hex.length === 3 || hex.length === 4) {
      hex = hex
        .split("")
        .map((digit) => digit + digit)
        .join("");
    }
    if (hex.length !== 6 && hex.length !== 8) return null;
    if (!/^[0-9a-f]+$/.test(hex)) return null;

    return {
      r: parseInt(hex.slice(0, 2), 16) / 255,
      g: parseInt(hex.slice(2, 4), 16) / 255,
      b: parseInt(hex.slice(4, 6), 16) / 255,
      a: hex.length === 8 ? parseInt(hex.slice(6, 8), 16) / 255 : 1,
    };
  }

  var browserColorCache = Object.create(null);

  function normalizeBrowserColor(value) {
    var direct = parseHexColor(value);
    if (direct) return value;
    if (typeof document === "undefined") return null;
    if (
      typeof CSS !== "undefined" &&
      typeof CSS.supports === "function" &&
      !CSS.supports("color", value)
    ) {
      return null;
    }
    if (Object.hasOwn(browserColorCache, value)) {
      return browserColorCache[value] || null;
    }

    var canvas = document.createElement("canvas");
    canvas.width = 1;
    canvas.height = 1;
    var context = canvas.getContext("2d", { willReadFrequently: true });
    if (!context) return null;
    context.clearRect(0, 0, 1, 1);
    context.fillStyle = value;
    context.fillRect(0, 0, 1, 1);
    var pixel = context.getImageData(0, 0, 1, 1).data;
    var normalized =
      "#" +
      colorByte(pixel[0] / 255) +
      colorByte(pixel[1] / 255) +
      colorByte(pixel[2] / 255);
    if (pixel[3] < 255) normalized += colorByte(pixel[3] / 255);
    browserColorCache[value] = normalized;
    return normalized;
  }

  function srgbToLinear(value) {
    return value <= 0.04045 ? value / 12.92 : ((value + 0.055) / 1.055) ** 2.4;
  }

  function linearToSrgb(value) {
    return value <= 0.0031308
      ? 12.92 * value
      : 1.055 * value ** (1 / 2.4) - 0.055;
  }

  function rgbToOklab(color) {
    var r = srgbToLinear(color.r);
    var g = srgbToLinear(color.g);
    var b = srgbToLinear(color.b);
    var l = Math.cbrt(0.4122214708 * r + 0.5363325363 * g + 0.0514459929 * b);
    var m = Math.cbrt(0.2119034982 * r + 0.6806995451 * g + 0.1073969566 * b);
    var s = Math.cbrt(0.0883024619 * r + 0.2817188376 * g + 0.6299787005 * b);

    return {
      l: 0.2104542553 * l + 0.793617785 * m - 0.0040720468 * s,
      a: 1.9779984951 * l - 2.428592205 * m + 0.4505937099 * s,
      b: 0.0259040371 * l + 0.7827717662 * m - 0.808675766 * s,
    };
  }

  function oklabToLinearRgb(color) {
    var l = color.l + 0.3963377774 * color.a + 0.2158037573 * color.b;
    var m = color.l - 0.1055613458 * color.a - 0.0638541728 * color.b;
    var s = color.l - 0.0894841775 * color.a - 1.291485548 * color.b;
    l = l * l * l;
    m = m * m * m;
    s = s * s * s;

    return {
      r: 4.0767416621 * l - 3.3077115913 * m + 0.2309699292 * s,
      g: -1.2684380046 * l + 2.6097574011 * m - 0.3413193965 * s,
      b: -0.0041960863 * l - 0.7034186147 * m + 1.707614701 * s,
    };
  }

  function isInSrgbGamut(color) {
    var epsilon = 0.000001;
    return (
      color.r >= -epsilon &&
      color.r <= 1 + epsilon &&
      color.g >= -epsilon &&
      color.g <= 1 + epsilon &&
      color.b >= -epsilon &&
      color.b <= 1 + epsilon
    );
  }

  function clamp(value, min, max) {
    return Math.min(max, Math.max(min, value));
  }

  function gamutMapOklab(color) {
    var direct = oklabToLinearRgb(color);
    if (isInSrgbGamut(direct)) return direct;

    var low = 0;
    var high = 1;
    var best = oklabToLinearRgb({ l: color.l, a: 0, b: 0 });
    for (var i = 0; i < 16; i++) {
      var scale = (low + high) / 2;
      var candidate = oklabToLinearRgb({
        l: color.l,
        a: color.a * scale,
        b: color.b * scale,
      });
      if (isInSrgbGamut(candidate)) {
        low = scale;
        best = candidate;
      } else {
        high = scale;
      }
    }
    return best;
  }

  function colorByte(value) {
    return Math.round(clamp(value, 0, 1) * 255)
      .toString(16)
      .padStart(2, "0");
  }

  function serializeColor(linear, alpha) {
    var value =
      "#" +
      colorByte(linearToSrgb(linear.r)) +
      colorByte(linearToSrgb(linear.g)) +
      colorByte(linearToSrgb(linear.b));
    if (alpha < 0.999999) value += colorByte(alpha);
    return value;
  }

  function smoothstep(edge0, edge1, value) {
    var position = clamp((value - edge0) / (edge1 - edge0), 0, 1);
    return position * position * (3 - 2 * position);
  }

  function mixOklab(from, to, amount) {
    return {
      l: from.l + (to.l - from.l) * amount,
      a: from.a + (to.a - from.a) * amount,
      b: from.b + (to.b - from.b) * amount,
    };
  }

  function makeSvgTheme(foreground, background) {
    var fg = parseHexColor(foreground);
    var bg = parseHexColor(background);
    if (!fg || !bg) return null;
    return {
      foreground: foreground,
      background: background,
      foregroundLab: rgbToOklab(fg),
      backgroundLab: rgbToOklab(bg),
    };
  }

  function adaptSolidPaint(value, sourceDark, theme, textPaint) {
    var parsed = parseHexColor(value);
    if (!parsed || !theme) return null;

    var original = rgbToOklab(parsed);
    var chroma = Math.hypot(original.a, original.b);
    var neutralWeight =
      1 - smoothstep(ACHROMATIC_EPSILON, NEUTRAL_CHROMA_CEILING, chroma);
    if (neutralWeight <= 0) return value;

    var sourceBackgroundLightness = sourceDark ? 0 : 1;
    if (textPaint && Math.abs(original.l - sourceBackgroundLightness) < 0.15) {
      return value;
    }

    var foregroundAmount = sourceDark ? original.l : 1 - original.l;
    var target = mixOklab(
      theme.backgroundLab,
      theme.foregroundLab,
      foregroundAmount,
    );
    var adapted = mixOklab(original, target, neutralWeight);
    return serializeColor(gamutMapOklab(adapted), parsed.a);
  }

  function findSourceDark(svg) {
    var first = svg.firstElementChild;
    var depth = 0;
    while (first && first.tagName.toLowerCase() === "g" && depth < 32) {
      first = first.firstElementChild;
      depth += 1;
    }
    if (!first || first.tagName.toLowerCase() !== "path") return false;
    var path = first.getAttribute("d") || "";
    if (!/^M\s*0\s+0v/i.test(path)) return false;
    var fill = parseHexColor(first.getAttribute("fill") || "");
    if (!fill) return false;
    var lab = rgbToOklab(fill);
    if (Math.hypot(lab.a, lab.b) > NEUTRAL_CHROMA_CEILING) return false;
    return lab.l < 0.5;
  }

  function safeStaticPaint(value, theme) {
    var normalized = value.trim();
    if (parseHexColor(normalized)) return normalized;
    if (/^url\(\s*#[A-Za-z0-9_.:-]+\s*\)$/.test(normalized)) {
      return normalized;
    }
    if (normalized === "none") return normalized;
    if (normalized.toLowerCase() === "currentcolor" && theme) {
      return theme.foreground;
    }
    return null;
  }

  function isTextPaint(element, property) {
    return (
      property === "fill" &&
      typeof element.closest === "function" &&
      element.closest(".typst-text") !== null
    );
  }

  function readSvgTheme(svg) {
    var styles = window.getComputedStyle(svg);
    var foreground =
      normalizeBrowserColor(
        styles.getPropertyValue("--typmark-svg-fg").trim(),
      ) ||
      normalizeBrowserColor(styles.getPropertyValue("--typmark-fg").trim()) ||
      "#111418";
    var background =
      normalizeBrowserColor(
        styles.getPropertyValue("--typmark-svg-bg").trim(),
      ) ||
      normalizeBrowserColor(styles.getPropertyValue("--typmark-bg").trim()) ||
      "#fbfbf8";
    return makeSvgTheme(foreground, background);
  }

  function applySvgTheme(svg, theme) {
    if (!theme) return;
    var sourceDark = findSourceDark(svg);
    var painted = svg.querySelectorAll("[fill], [stroke], [stop-color]");

    painted.forEach((element) => {
      SVG_PAINT_PROPERTIES.forEach((property) => {
        if (!element.hasAttribute(property)) return;
        var original = element.getAttribute(property);
        var adapted = adaptSolidPaint(
          original,
          sourceDark,
          theme,
          isTextPaint(element, property),
        );
        var protectedPaint = adapted || safeStaticPaint(original, theme);
        if (protectedPaint) {
          element.style.setProperty(property, protectedPaint, "important");
        }
      });
    });

    var graphics = svg.querySelectorAll("*");
    graphics.forEach((element) => {
      if (!element.hasAttribute("fill")) {
        element.style.setProperty("fill", "inherit", "important");
      }
      if (!element.hasAttribute("stroke")) {
        element.style.setProperty("stroke", "inherit", "important");
      }
      if (
        element.tagName.toLowerCase() === "stop" &&
        !element.hasAttribute("stop-color")
      ) {
        element.style.setProperty("stop-color", theme.foreground, "important");
      }
      element.style.setProperty("filter", "none", "important");
      element.style.setProperty("mix-blend-mode", "normal", "important");
    });

    svg.style.setProperty("color", theme.foreground, "important");
    svg.style.setProperty("fill", theme.foreground, "important");
    svg.style.setProperty("stroke", "none", "important");
    svg.style.setProperty("background-color", "transparent", "important");
    svg.style.setProperty("filter", "none", "important");
    svg.style.setProperty("mix-blend-mode", "normal", "important");
    svg.classList.add("TypMark-svg-themed");
  }

  function setupTypstSvgThemes() {
    var frame = 0;
    function applyAll() {
      frame = 0;
      var svgs = document.querySelectorAll(".TypMark-typst-block .typst-doc");
      svgs.forEach((svg) => {
        applySvgTheme(svg, readSvgTheme(svg));
      });
    }
    function schedule() {
      if (frame) return;
      frame = window.requestAnimationFrame(applyAll);
    }

    applyAll();
    var media = window.matchMedia("(prefers-color-scheme: dark)");
    if (typeof media.addEventListener === "function") {
      media.addEventListener("change", schedule);
    } else if (typeof media.addListener === "function") {
      media.addListener(schedule);
    }

    if (typeof MutationObserver !== "undefined") {
      var themeObserver = new MutationObserver(applyAll);
      svgThemeObservers.push(themeObserver);
      themeObserver.observe(document.documentElement, {
        attributes: true,
        attributeFilter: ["class", "style"],
      });
      if (document.body) {
        themeObserver.observe(document.body, {
          attributes: true,
          attributeFilter: ["class", "style"],
        });
        var contentObserver = new MutationObserver(applyAll);
        svgThemeObservers.push(contentObserver);
        contentObserver.observe(document.body, {
          childList: true,
          subtree: true,
        });
      }
    }
    window.addEventListener("beforeprint", applyAll);
  }

  var svgThemeTestApi = {
    adaptPaint: (value, sourceDark, foreground, background, textPaint) =>
      adaptSolidPaint(
        value,
        sourceDark,
        makeSvgTheme(foreground, background),
        textPaint,
      ),
    parseColor: parseHexColor,
    sourceIsDark: findSourceDark,
  };

  if (typeof module !== "undefined" && module.exports) {
    module.exports = svgThemeTestApi;
  }
  if (typeof document === "undefined") return;

  function init() {
    applyBoxAttributes();
    wireLineAnchors();
    setupRefScroll();
    setupMathScrollShadows();
    setupTypstSvgThemes();
  }

  if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", init);
  } else {
    init();
  }
})();

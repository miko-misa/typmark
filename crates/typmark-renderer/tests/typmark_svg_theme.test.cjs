const assert = require("node:assert/strict");
const path = require("node:path");
const test = require("node:test");

const theme = require(path.join(__dirname, "../assets/typmark.js"));
const dark = ["#f7f9fb", "#0e1116"];
const light = ["#111418", "#fbfbf8"];

function adapt(value, sourceDark, palette, textPaint = false) {
  return theme.adaptPaint(value, sourceDark, palette[0], palette[1], textPaint);
}

test("maps a light-source neutral axis onto the dark theme", () => {
  assert.equal(adapt("#000000", false, dark), dark[0]);
  assert.equal(adapt("#ffffff", false, dark), dark[1]);
  assert.equal(adapt("#00000080", false, dark), dark[0] + "80");
});

test("maps a dark-source neutral axis without reversing its roles", () => {
  assert.equal(adapt("#000000", true, dark), dark[1]);
  assert.equal(adapt("#ffffff", true, dark), dark[0]);
});

test("detects page backgrounds nested in Typst groups", () => {
  function element(tagName, attrs = {}, firstElementChild = null) {
    return {
      tagName,
      firstElementChild,
      getAttribute(name) {
        return attrs[name] || null;
      },
    };
  }

  const darkPath = element("path", {
    d: "M 0 0v 20 h 30 v -20 Z",
    fill: "#000000",
  });
  const lightPath = element("path", {
    d: "M 0 0v 20 h 30 v -20 Z",
    fill: "#ffffff",
  });
  assert.equal(
    theme.sourceIsDark(element("svg", {}, element("g", {}, darkPath))),
    true,
  );
  assert.equal(
    theme.sourceIsDark(element("svg", {}, element("g", {}, lightPath))),
    false,
  );
});

test("preserves chromatic paints instead of blanket inversion", () => {
  assert.equal(adapt("#ff4136", false, dark), "#ff4136");
  assert.equal(adapt("#1f5da8", false, dark), "#1f5da8");
});

test("adapts pale neutrals smoothly and preserves local label contrast", () => {
  const paleBlue = adapt("#e8f1fb", false, dark);
  assert.notEqual(paleBlue, "#e8f1fb");
  assert.notEqual(paleBlue, "#170e04");
  assert.equal(adapt("#ffffff", false, dark, true), "#ffffff");
  assert.equal(adapt("#000000", true, dark, true), "#000000");
  assert.equal(adapt("#ffffff", true, dark, true), dark[0]);
});

test("harmonizes black with the light theme foreground", () => {
  assert.equal(adapt("black", false, light), light[0]);
  assert.deepEqual(theme.parseColor("#fff"), {
    r: 1,
    g: 1,
    b: 1,
    a: 1,
  });
});

function mockElement(classes = []) {
  const values = new Set(classes);
  const listeners = new Map();
  const element = {
    children: [],
    parentNode: null,
    scrollLeft: 0,
    scrollWidth: 100,
    clientWidth: 100,
    classList: {
      add: (...names) => names.forEach((name) => values.add(name)),
      remove: (...names) => names.forEach((name) => values.delete(name)),
      contains: (name) => values.has(name),
      toggle(name, force) {
        if (force === undefined ? !values.has(name) : force) values.add(name);
        else values.delete(name);
      },
    },
    appendChild(child) {
      if (child.parentNode) {
        const siblings = child.parentNode.children;
        siblings.splice(siblings.indexOf(child), 1);
      }
      child.parentNode = element;
      element.children.push(child);
      return child;
    },
    addEventListener(name, listener) {
      listeners.set(name, listener);
    },
  };
  Object.defineProperties(element, {
    firstChild: { get: () => element.children[0] || null },
    className: {
      get: () => [...values].join(" "),
      set: (value) => {
        values.clear();
        value.split(/\s+/).filter(Boolean).forEach((name) => values.add(name));
      },
    },
  });
  return element;
}

test("initializes math scroll shadows only near the viewport", () => {
  const original = {
    document: global.document,
    window: global.window,
    IntersectionObserver: global.IntersectionObserver,
    ResizeObserver: global.ResizeObserver,
    setTimeout: global.setTimeout,
  };
  const blocks = [mockElement(["TypMark-math-block"]), mockElement(["TypMark-math-block"])];
  blocks.forEach((block) => block.appendChild(mockElement()));
  const windowListeners = new Map();
  let intersection;

  global.document = {
    querySelectorAll: () => blocks,
    createElement: () => mockElement(),
  };
  global.window = {
    location: { hash: "" },
    addEventListener(name, listener) {
      windowListeners.set(name, listener);
    },
    requestAnimationFrame(callback) {
      callback();
      return 1;
    },
  };
  global.IntersectionObserver = class {
    constructor(callback, options) {
      this.callback = callback;
      this.options = options;
      this.observed = new Set();
      intersection = this;
    }
    observe(target) {
      this.observed.add(target);
    }
    unobserve(target) {
      this.observed.delete(target);
    }
    disconnect() {
      this.disconnected = true;
      this.observed.clear();
    }
  };
  global.ResizeObserver = class {
    observe() {}
  };
  global.setTimeout = (callback) => {
    callback();
    return 1;
  };

  try {
    theme.setupMathScrollShadows();
    assert.equal(intersection.options.rootMargin, "1000px 0px");
    assert.equal(intersection.observed.size, 2);
    assert.equal(
      blocks[0].classList.contains("TypMark-math-block--scroll"),
      false,
    );

    intersection.callback([{ isIntersecting: true, target: blocks[0] }]);
    assert.equal(
      blocks[0].classList.contains("TypMark-math-block--scroll"),
      true,
    );
    assert.equal(
      blocks[1].classList.contains("TypMark-math-block--scroll"),
      false,
    );
    assert.equal(intersection.observed.has(blocks[0]), false);
    intersection.callback([{ isIntersecting: true, target: blocks[0] }]);
    assert.equal(blocks[0].children.length, 1);

    windowListeners.get("hashchange")();
    assert.equal(intersection.disconnected, true);
    assert.equal(
      blocks[1].classList.contains("TypMark-math-block--scroll"),
      true,
    );
    windowListeners.get("beforeprint")();
    assert.equal(blocks[1].children.length, 1);
  } finally {
    Object.assign(global, original);
  }
});

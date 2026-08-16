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

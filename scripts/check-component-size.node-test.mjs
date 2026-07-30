import assert from "node:assert/strict";
import test from "node:test";

import { analyzeSource } from "./check-component-size.mjs";

test("excludes imports, blank lines, and pure comments", () => {
  const result = analyzeSource("Fixture.tsx", `
import { Button } from "@/components/ui/button";

// Rendering is intentionally small.
export function Fixture() {
  return <Button />;
}
`);

  assert.deepEqual(result[0], {
    exception: false,
    lines: 3,
    name: "Fixture",
    startLine: 5,
  });
});

test("recognizes an adjacent documented exception", () => {
  const result = analyzeSource("Fixture.tsx", `
// component-size-exception: native accessibility contract prevents safe extraction
export function Fixture() {
  return <div />;
}
`);

  assert.equal(result[0]?.exception, true);
});

test("reports a component above the effective-line threshold", () => {
  const body = Array.from({length: 200}, (_, index) => `  void ${index};`).join("\n");
  const result = analyzeSource("Oversized.tsx", `
export function Oversized() {
${body}
  return <div />;
}
`);

  assert.ok(result[0].lines > 200);
  assert.equal(result[0].exception, false);
});

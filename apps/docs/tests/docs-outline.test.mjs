import assert from "node:assert/strict";
import test from "node:test";
import { activeOutlineIndex } from "../lib/docs-outline.ts";

test("outline follows the last heading above the reading line in both scroll directions", () => {
	assert.equal(activeOutlineIndex([220, 800, 1600], 120, 2200, 900), 0);
	assert.equal(activeOutlineIndex([-500, 119, 950], 120, 1800, 900), 1);
	assert.equal(activeOutlineIndex([-900, -200, 120], 120, 1300, 900), 2);
	assert.equal(activeOutlineIndex([-100, 121, 500], 120, 1500, 900), 0);
});
test("short final sections become active when the end of the article is visible", () => {
	assert.equal(activeOutlineIndex([-500, 400, 650], 120, 820, 900), 2);
	assert.equal(activeOutlineIndex([200, 400], 120, 820, 900), 0);
	assert.equal(activeOutlineIndex([], 120, 820, 900), -1);
});

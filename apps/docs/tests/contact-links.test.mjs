import assert from "node:assert/strict";
import test from "node:test";
import { publicContactUrl } from "../lib/contact-links.ts";

test("contact configuration supports explicit email and HTTPS private forms", () => {
	assert.equal(publicContactUrl(undefined), null);
	assert.equal(publicContactUrl("  "), null);
	assert.equal(
		publicContactUrl("mailto:contact@triooo.com.br"),
		"mailto:contact@triooo.com.br",
	);
	assert.equal(
		publicContactUrl("https://example.com/private-report"),
		"https://example.com/private-report",
	);
});
test("contact configuration rejects executable schemes, credentials and email header injection", () => {
	for (const value of [
		"javascript:alert(1)",
		"http://example.com",
		"data:text/html,hello",
		"https://user:password@example.com",
		"mailto:not-an-address",
		"mailto:contact@example.com?bcc=other@example.com",
		"mailto:contact@example.com%0d%0aBcc:other@example.com",
		"mailto:contact@example.com\n",
	]) {
		assert.throws(() => publicContactUrl(value), value);
	}
});

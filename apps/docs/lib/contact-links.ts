/** Contact links are published as anchors; never accept executable schemes or credentials. */
export function publicContactUrl(value: string | undefined): string | null {
	if (!value?.trim()) return null;
	if (
		Array.from(decodeURIComponent(value)).some(
			(character) =>
				character.charCodeAt(0) < 32 || character.charCodeAt(0) === 127,
		)
	)
		throw new Error("Contact URLs cannot contain control characters.");
	const url = new URL(value.trim());
	if (url.protocol === "https:" && !url.username && !url.password)
		return url.href;
	if (
		url.protocol === "mailto:" &&
		!url.search &&
		!url.hash &&
		/^[^\s<>?@]+@[^\s<>?@]+\.[^\s<>?@]+$/.test(url.pathname)
	)
		return url.href;
	throw new Error("Contact URLs must use HTTPS or a plain mailto address.");
}

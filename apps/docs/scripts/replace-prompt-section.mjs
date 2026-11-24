const startMarker = "<!-- planned-stack-services:start -->";
const endMarker = "<!-- planned-stack-services:end -->";

/** Replace only the generated service catalog, preserving handwritten instructions. */
export function replacePromptServices(current, generated) {
	const start = current.indexOf(startMarker);
	if (start === -1) return `${current.trimEnd()}\n\n${generated.trim()}\n`;
	const end = current.indexOf(endMarker, start);
	if (end === -1)
		throw new Error(
			"Missing service catalog end marker; refusing to overwrite PROMPT.md.",
		);
	return (
		current.slice(0, start) +
		generated.trim() +
		current.slice(end + endMarker.length)
	);
}

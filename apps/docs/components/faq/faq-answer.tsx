/** Render plain-text FAQ paragraphs from next-intl's raw messages.
 * No HTML parsing: translations remain text, including CLI command examples.
 */
export function FAQAnswer({ value }: { value: unknown }) {
	const paragraphs =
		typeof value === "string"
			? [value]
			: value && typeof value === "object" && !Array.isArray(value)
				? Object.entries(value as Record<string, unknown>)
						.filter(
							([key, text]) => /^p\d+$/.test(key) && typeof text === "string",
						)
						.sort(([a], [b]) => Number(a.slice(1)) - Number(b.slice(1)))
						.map(([, text]) => text as string)
				: [];

	return (
		<div className="space-y-3 text-sm leading-7 text-muted-foreground">
			{paragraphs.map((paragraph, i) => (
				<p key={`${i}-${paragraph.slice(0, 16)}`}>{paragraph}</p>
			))}
		</div>
	);
}

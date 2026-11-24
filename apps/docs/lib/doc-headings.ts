export function getDocHeadings(content: string) {
	let fenced = false;
	const counts = new Map<string, number>();
	return content.split("\n").flatMap((line) => {
		if (/^\s*(```|~~~)/.test(line)) {
			fenced = !fenced;
			return [];
		}
		const match = !fenced && line.match(/^(#{2,3})\s+(.+?)\s*#*$/);
		if (!match) return [];
		const title = match[2]
			.replace(/\[([^\]]+)\]\([^)]*\)/g, "$1")
			.replace(/[*`_]/g, "");
		const base =
			title
				.toLowerCase()
				.normalize("NFKD")
				.replace(/\p{M}/gu, "")
				.replace(/[^\p{L}\p{N}\s-]/gu, "")
				.trim()
				.replace(/\s+/g, "-") || "section";
		const count = counts.get(base) ?? 0;
		counts.set(base, count + 1);
		return [
			{ title, id: count ? `${base}-${count}` : base, level: match[1].length },
		];
	});
}

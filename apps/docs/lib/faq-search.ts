export function faqMatches(question: string, answer: unknown, query: string) {
	const normalize = (text: string) =>
		text.normalize("NFKD").replace(/\p{M}/gu, "").toLowerCase();
	const paragraphs =
		typeof answer === "string"
			? answer
			: answer && typeof answer === "object"
				? Object.values(answer)
						.filter((value) => typeof value === "string")
						.join(" ")
				: "";
	const text = normalize(`${question} ${paragraphs}`);
	return normalize(query)
		.trim()
		.split(/\s+/)
		.every((word) => text.includes(word));
}

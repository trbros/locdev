export function parseLicenseDocument(text: string) {
	const markers = [
		...text.matchAll(/^ {3}(?:(\d)\. ([^\n]+)|APPENDIX: ([^\n]+))$/gm),
	];
	return {
		introduction: text.slice(0, markers[0]?.index ?? text.length),
		sections: markers.map((marker, index) => ({
			id: marker[1] ? `section-${marker[1]}` : "appendix",
			title: marker[1]
				? `${marker[1]}. ${marker[2].split(". ")[0].replace(/\.$/, "")}`
				: `APPENDIX: ${marker[3]}`,
			raw: text.slice(marker.index, markers[index + 1]?.index ?? text.length),
		})),
	};
}

/** Last heading above the reading line, including short final sections. */
export function activeOutlineIndex(
	tops: number[],
	readingLine: number,
	articleBottom: number,
	viewportHeight: number,
) {
	if (!tops.length) return -1;
	if (articleBottom <= viewportHeight && tops[0] < readingLine)
		return tops.length - 1;
	let active = 0;
	for (let index = 0; index < tops.length; index++) {
		if (tops[index] > readingLine) break;
		active = index;
	}
	return active;
}

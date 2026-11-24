/** Frontend/API examples from lib/stacks.ts, communicating over HTTP. */
export const storyPairs = [
	{
		frontend: "React",
		frontendIcon: "React",
		api: "FastAPI",
		apiIcon: "FastAPI",
		frontendPort: 5173,
		apiPort: 8000,
	},
	{
		frontend: "Vue",
		frontendIcon: "Vue",
		api: "Laravel",
		apiIcon: "Laravel",
		frontendPort: 5173,
		apiPort: 9000,
	},
	{
		frontend: "Svelte",
		frontendIcon: "Svelte",
		api: "Express",
		apiIcon: "Expressjs",
		frontendPort: 5173,
		apiPort: 4000,
	},
	{
		frontend: "Next.js",
		frontendIcon: "Nextjs",
		api: "NestJS",
		apiIcon: "Nestjs",
		frontendPort: 3000,
		apiPort: 4000,
	},
	{
		frontend: "Nuxt",
		frontendIcon: "Nuxt",
		api: "Django",
		apiIcon: "Django",
		frontendPort: 3000,
		apiPort: 8000,
	},
] as const;

export const storyDurations = [
	1100, 1450, 950, 1250, 1800, 1900, 1600, 1400, 1750, 2100, 1300, 0,
];
export const storyMilestones = storyDurations.map((_, index) =>
	storyDurations
		.slice(0, index + 1)
		.reduce((sum, duration) => sum + duration, 0),
);
export const storyTotalMs = storyMilestones[storyMilestones.length - 1];

/** Cubic curves arrive and leave horizontally, with no sharp branch junction. */
export function storyConnection(index: number) {
	return `M 0 50 C 45 50, 55 ${10 + index * 20}, 100 ${10 + index * 20}`;
}

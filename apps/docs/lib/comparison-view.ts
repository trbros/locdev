import {
	type Feature,
	groups,
	type ToolId,
	tools,
} from "./comparison-matrix.ts";

export const comparisonViews = ["all", "differences", "advantages"] as const;
export type ComparisonView = (typeof comparisonViews)[number];
export type CompetitorId = Exclude<ToolId, "locdev">;
export const competitors = tools.filter(
	(tool): tool is (typeof tools)[number] & { id: CompetitorId } =>
		tool.id !== "locdev",
);

export const brandImages: Record<ToolId, string> = {
	locdev: "/logo.png",
	herd: "/brands/comparison/herd.svg",
	yerd: "/brands/comparison/yerd.svg",
	lerd: "/brands/comparison/lerd.svg",
	laragon: "/brands/comparison/laragon.svg",
	ddev: "/brands/comparison/ddev.png",
	lando: "/brands/comparison/lando.png",
	sail: "/brands/comparison/laravel.png",
	valet: "/brands/comparison/laravel.png",
	compose: "/brands/comparison/docker.png",
};

export function readComparisonLocation(params: URLSearchParams) {
	const tool = params.get("app");
	const view = params.get("view");
	const category = params.get("category");
	return {
		competitor: competitors.find((item) => item.id === tool) ?? competitors[0],
		view: comparisonViews.find((item) => item === view) ?? "all",
		category: groups.some((group) => group.id === category) ? category : "all",
		query: (params.get("q") ?? "").slice(0, 160),
	};
}

export function isLocdevAdvantage(feature: Feature, competitor: CompetitorId) {
	return (
		feature.cells.locdev.status === "yes" &&
		["no", "partial"].includes(feature.cells[competitor].status)
	);
}

export function matchesComparisonView(
	feature: Feature,
	competitor: CompetitorId,
	view: ComparisonView,
) {
	if (view === "advantages") return isLocdevAdvantage(feature, competitor);
	if (view === "differences") {
		const left = feature.cells.locdev.status;
		const right = feature.cells[competitor].status;
		return left !== "unknown" && right !== "unknown" && left !== right;
	}
	return true;
}

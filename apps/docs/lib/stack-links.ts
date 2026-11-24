import { type StackSupport, stacks } from "./stacks.ts";
export function readStackLocation(params: URLSearchParams) {
	const index = stacks.findIndex((stack) => stack.id === params.get("stack"));
	const rawFilter = params.get("filter");
	const filter: StackSupport | "all" =
		rawFilter === "detected" || rawFilter === "script" || rawFilter === "custom"
			? rawFilter
			: "all";
	return {
		index: Math.max(index, 0),
		query: (params.get("q") ?? "").slice(0, 160),
		filter,
		open: index >= 0 && params.get("workflow") !== "0",
	};
}
export function stackLocationHref(
	pathname: string,
	params: URLSearchParams,
	patch: Record<string, string | null>,
) {
	const next = new URLSearchParams(params);
	for (const [key, value] of Object.entries(patch)) {
		if (value) next.set(key, value);
		else next.delete(key);
	}
	const query = next.toString();
	return `${pathname}${query ? `?${query}` : ""}`;
}

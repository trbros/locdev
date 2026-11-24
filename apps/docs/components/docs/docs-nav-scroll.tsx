"use client";

import { type ReactNode, useEffect, useRef } from "react";

/** Scroll only the navigation pane; never move the article or document viewport. */
export function DocsNavScroll({
	children,
	current,
}: {
	children: ReactNode;
	current: string;
}) {
	const ref = useRef<HTMLDivElement>(null);
	useEffect(() => {
		const pane = ref.current;
		if (!pane) return;
		const reveal = () => {
			if (!pane.clientHeight) return;
			const active = Array.from(
				pane.querySelectorAll<HTMLElement>('[aria-current="page"]'),
			).find((link) => link.dataset.docSlug === current);
			if (!active) return;
			const bounds = pane.getBoundingClientRect();
			const item = active.getBoundingClientRect();
			if (item.top < bounds.top + 12 || item.bottom > bounds.bottom - 12) {
				pane.scrollTop +=
					item.top - bounds.top - (pane.clientHeight - item.height) / 2;
			}
		};
		const frame = requestAnimationFrame(reveal);
		const mobile = pane.closest("details");
		mobile?.addEventListener("toggle", reveal);
		const observer =
			typeof ResizeObserver !== "undefined" ? new ResizeObserver(reveal) : null;
		observer?.observe(pane);
		return () => {
			cancelAnimationFrame(frame);
			mobile?.removeEventListener("toggle", reveal);
			observer?.disconnect();
		};
	}, [current]);
	return (
		<div ref={ref} className="docs-nav-scroll">
			{children}
		</div>
	);
}

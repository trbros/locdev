"use client";

import { useEffect, useRef, useState } from "react";
import { activeOutlineIndex } from "@/lib/docs-outline";

type Heading = { id: string; title: string; level: number };

export function DocsToc({
	headings,
	label,
}: {
	headings: Heading[];
	label: string;
}) {
	const [active, setActive] = useState(headings[0]?.id ?? "");
	const pane = useRef<HTMLDivElement>(null);
	useEffect(() => {
		const article = document.getElementById("doc-content");
		if (!article) return;
		const targets = headings.flatMap((heading) => {
			const element = document.getElementById(heading.id);
			return element ? [{ id: heading.id, element }] : [];
		});
		let frame = 0;
		const update = () => {
			frame = 0;
			const header = document.querySelector(".site-header");
			const line = Math.max(
				120,
				(header?.getBoundingClientRect().bottom ?? 0) + 32,
			);
			const index = activeOutlineIndex(
				targets.map(({ element }) => element.getBoundingClientRect().top),
				line,
				article.getBoundingClientRect().bottom,
				window.innerHeight,
			);
			if (index >= 0) setActive(targets[index].id);
		};
		const schedule = () => {
			if (!frame) frame = requestAnimationFrame(update);
		};
		schedule();
		window.addEventListener("scroll", schedule, { passive: true });
		window.addEventListener("resize", schedule);
		window.addEventListener("hashchange", schedule);
		const observer =
			typeof ResizeObserver !== "undefined"
				? new ResizeObserver(schedule)
				: null;
		observer?.observe(article);
		return () => {
			cancelAnimationFrame(frame);
			window.removeEventListener("scroll", schedule);
			window.removeEventListener("resize", schedule);
			window.removeEventListener("hashchange", schedule);
			observer?.disconnect();
		};
	}, [headings]);
	useEffect(() => {
		const container = pane.current;
		const link = container
			? Array.from(container.querySelectorAll<HTMLAnchorElement>("a")).find(
					(item) => item.getAttribute("href") === `#${active}`,
				)
			: null;
		if (!container || !link || !container.clientHeight) return;
		const box = container.getBoundingClientRect();
		const item = link.getBoundingClientRect();
		if (item.top < box.top) container.scrollTop -= box.top - item.top + 8;
		else if (item.bottom > box.bottom)
			container.scrollTop += item.bottom - box.bottom + 8;
	}, [active]);
	return (
		<aside className="docs-toc">
			<nav aria-label={label}>
				<p>{label}</p>
				<div className="docs-toc-scroll" ref={pane}>
					{headings.map((heading) => (
						<a
							key={heading.id}
							href={`#${heading.id}`}
							aria-current={active === heading.id ? "location" : undefined}
							className={heading.level === 3 ? "docs-toc-nested" : undefined}
						>
							{heading.title}
						</a>
					))}
				</div>
			</nav>
		</aside>
	);
}

"use client";

import { type ReactNode, useEffect, useRef } from "react";

/** Progressive enhancement: the server content remains visible without JavaScript. */
export function PageReveal({
	children,
	className = "",
}: {
	children: ReactNode;
	className?: string;
}) {
	const ref = useRef<HTMLDivElement>(null);
	useEffect(() => {
		const root = ref.current;
		if (!root || !("IntersectionObserver" in window)) return;
		const observer = new IntersectionObserver(
			(entries) => {
				for (const entry of entries) {
					const element = entry.target as HTMLElement;
					element.dataset.inView = String(entry.isIntersecting);
					if (entry.isIntersecting) element.dataset.revealed = "true";
				}
			},
			{ threshold: 0.08 },
		);
		root.dataset.motion = "ready";
		Array.from(root.children).forEach((child, index) => {
			(child as HTMLElement).style.setProperty(
				"--reveal-delay",
				`${(index % 4) * 70}ms`,
			);
			observer.observe(child);
		});
		return () => {
			observer.disconnect();
			delete root.dataset.motion;
		};
	}, []);
	return (
		<div ref={ref} className={`page-reveal ${className}`}>
			{children}
		</div>
	);
}

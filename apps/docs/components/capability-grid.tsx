"use client";

import { type ReactNode, useEffect, useRef } from "react";

export function CapabilityGrid({ children }: { children: ReactNode }) {
	const ref = useRef<HTMLDivElement>(null);

	useEffect(() => {
		const grid = ref.current;
		if (!grid || !("IntersectionObserver" in window)) return;

		const cards = grid.querySelectorAll<HTMLElement>(".cap-card");
		const observer = new IntersectionObserver(
			(entries) => {
				for (const entry of entries) {
					const card = entry.target as HTMLElement;
					card.dataset.active = String(entry.isIntersecting);
					if (entry.isIntersecting) card.dataset.revealed = "true";
				}
			},
			{ threshold: 0.12 },
		);
		grid.dataset.motion = "ready";
		cards.forEach((card, index) => {
			card.style.setProperty("--cap-delay", `${(index % 3) * 80}ms`);
			observer.observe(card);
		});

		return () => {
			observer.disconnect();
			delete grid.dataset.motion;
		};
	}, []);

	return (
		<div ref={ref} className="cap-grid mt-12">
			{children}
		</div>
	);
}

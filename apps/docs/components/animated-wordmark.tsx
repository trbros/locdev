"use client";

import { type CSSProperties, useEffect, useRef } from "react";

type AnimatedWordmarkProps = {
	className?: string;
	/** Hide the wordmark from assistive technology when it is only decoration. */
	decorative?: boolean;
};

export function AnimatedWordmark({
	className = "",
	decorative = false,
}: AnimatedWordmarkProps) {
	const ref = useRef<HTMLSpanElement>(null);

	useEffect(() => {
		const element = ref.current;
		if (!element || !("IntersectionObserver" in window)) return;
		const observer = new IntersectionObserver(
			([entry]) => {
				element.dataset.active = String(entry.isIntersecting);
				if (entry.isIntersecting) element.dataset.revealed = "true";
			},
			{ threshold: 0.15 },
		);
		observer.observe(element);
		return () => observer.disconnect();
	}, []);

	return (
		<span
			ref={ref}
			className={`animated-wordmark select-none pointer-events-none ${className}`}
			role="img"
			aria-label={decorative ? undefined : "locdev"}
			aria-hidden={decorative || undefined}
		>
			<span className="wordmark-letters" aria-hidden="true">
				{Array.from("locdev").map((letter, index) => (
					<span
						key={letter}
						className="wordmark-letter"
						data-letter={letter}
						style={{ "--letter-index": index } as CSSProperties}
					>
						{letter}
					</span>
				))}
				<span className="wordmark-dot">.</span>
			</span>
		</span>
	);
}

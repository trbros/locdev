"use client";

import { ArrowLeft, ArrowRight } from "lucide-react";
import { useTranslations } from "next-intl";
import { useEffect, useId, useRef, useState } from "react";
import { Button } from "@/components/ui/button";
import { type Stack, stacks } from "@/lib/stacks";
import { StackIcon } from "./stack-icon";

export function StackSelector({
	activeId,
	onSelect,
	onBrowse,
}: {
	activeId: Stack["id"];
	onSelect: (id: Stack["id"]) => void;
	onBrowse: () => void;
}) {
	const t = useTranslations("Stacks");
	const list = useRef<HTMLFieldSetElement>(null);
	const id = useId();
	const [edges, setEdges] = useState({ previous: false, next: true });
	useEffect(() => {
		const element = list.current;
		if (!element) return;
		function updateEdges() {
			if (!element) return;
			const next = {
				previous: element.scrollLeft > 2,
				next:
					element.scrollLeft + element.clientWidth < element.scrollWidth - 2,
			};
			setEdges((current) =>
				current.previous === next.previous && current.next === next.next
					? current
					: next,
			);
		}
		function alignActive() {
			if (!element) return;
			const active = element.querySelector<HTMLElement>(
				`[data-stack-id="${activeId}"]`,
			);
			if (!active) return;
			const left =
				element.scrollLeft +
				active.getBoundingClientRect().left -
				element.getBoundingClientRect().left -
				(element.clientWidth - active.offsetWidth) / 2;
			element.scrollTo({
				left,
				behavior: window.matchMedia("(prefers-reduced-motion: reduce)").matches
					? "instant"
					: "smooth",
			});
			updateEdges();
		}
		alignActive();
		element.addEventListener("scroll", updateEdges, { passive: true });
		const observer = new ResizeObserver(alignActive);
		observer.observe(element);
		return () => {
			element.removeEventListener("scroll", updateEdges);
			observer.disconnect();
		};
	}, [activeId]);
	function browse(direction: number) {
		const element = list.current;
		if (!element) return;
		onBrowse();
		element.scrollBy({
			left: direction * Math.max(180, element.clientWidth * 0.75),
			behavior: window.matchMedia("(prefers-reduced-motion: reduce)").matches
				? "instant"
				: "smooth",
		});
	}
	return (
		<div className="stack-selector-controls">
			<Button
				variant="outline"
				size="icon"
				className="stack-scroll-previous"
				aria-label={t("scrollPrevious")}
				aria-controls={id}
				disabled={!edges.previous}
				onClick={() => browse(-1)}
			>
				<ArrowLeft size={16} />
			</Button>
			<fieldset
				ref={list}
				id={id}
				className="stack-selector"
				aria-label={t("selectStack")}
			>
				{stacks.map((stack) => (
					<Button
						key={stack.id}
						variant="ghost"
						className="stack-selector-item"
						aria-label={stack.name}
						aria-pressed={stack.id === activeId}
						title={stack.name}
						data-stack-id={stack.id}
						onClick={() => onSelect(stack.id)}
					>
						<StackIcon stack={stack} />
						<span>{stack.name}</span>
					</Button>
				))}
			</fieldset>
			<Button
				variant="outline"
				size="icon"
				className="stack-scroll-next"
				aria-label={t("scrollNext")}
				aria-controls={id}
				disabled={!edges.next}
				onClick={() => browse(1)}
			>
				<ArrowRight size={16} />
			</Button>
		</div>
	);
}

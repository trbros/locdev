"use client";

import { ArrowRight, ArrowUpRight } from "lucide-react";
import { useTranslations } from "next-intl";
import { useCallback, useState } from "react";
import { Link } from "@/i18n/navigation";
import { frameworkRequestUrl, stacks } from "@/lib/stacks";
import { StackPlayer } from "./stack-player";
import { StackSelector } from "./stack-selector";

export function StackShowcase() {
	const t = useTranslations("Stacks");
	const servicesT = useTranslations("Services");
	const [index, setIndex] = useState(0);
	const [playing, setPlaying] = useState(true);
	const next = useCallback(() => setIndex((i) => (i + 1) % stacks.length), []);
	const previous = useCallback(
		() => setIndex((i) => (i - 1 + stacks.length) % stacks.length),
		[],
	);
	return (
		<section
			className="section-pad stack-showcase border-t border-border"
			aria-labelledby="stack-showcase-title"
		>
			<div className="page-container">
				<div className="section-topline">
					<p className="section-eyebrow">{t("eyebrow")}</p>
					<span className="stack-catalog-count">
						{t("count", { count: stacks.length })}
					</span>
				</div>
				<div className="stack-showcase-intro">
					<div>
						<h2 className="section-heading" id="stack-showcase-title">
							{t("homeTitle")}
						</h2>
						<p className="section-description mt-6">{t("homeBody")}</p>
					</div>
					<Link href="/stacks" className="btn-outline">
						{t("viewAll")}
						<ArrowRight size={16} />
					</Link>
				</div>
				<StackPlayer
					stack={stacks[index]}
					playing={playing}
					onToggle={() => setPlaying((p) => !p)}
					onNext={next}
					onPrevious={previous}
				/>
				<StackSelector
					activeId={stacks[index].id}
					onBrowse={() => setPlaying(false)}
					onSelect={(id) => {
						const nextIndex = stacks.findIndex((stack) => stack.id === id);
						if (nextIndex >= 0) setIndex(nextIndex);
						setPlaying(false);
					}}
				/>
				<div className="stack-showcase-footer">
					<Link href="/services">
						{servicesT("homeLink")}
						<ArrowRight size={14} />
					</Link>
					<p>{t("scopeNote")}</p>
					<a
						href={frameworkRequestUrl()}
						target="_blank"
						rel="noopener noreferrer"
					>
						{t("request")}
						<ArrowUpRight size={14} />
					</a>
				</div>
			</div>
		</section>
	);
}

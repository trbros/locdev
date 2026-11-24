"use client";

import { ArrowRight, ArrowUpRight, Search, X } from "lucide-react";
import { useSearchParams } from "next/navigation";
import { useTranslations } from "next-intl";
import { useCallback, useState } from "react";
import { Button } from "@/components/ui/button";
import { Dialog, DialogTrigger } from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Link } from "@/i18n/navigation";
import { route } from "@/lib/routes.generated";
import { readStackLocation, stackLocationHref } from "@/lib/stack-links";
import { frameworkRequestUrl, type StackSupport, stacks } from "@/lib/stacks";
import { StackIcon } from "./stack-icon";
import { StackPlayer } from "./stack-player";
import { StackWorkflowDialog } from "./stack-workflow-dialog";

export function StackCatalog() {
	const t = useTranslations("Stacks");
	const searchParams = useSearchParams();
	const location = readStackLocation(new URLSearchParams(searchParams));
	const { index, filter, query } = location;
	const [playing, setPlaying] = useState(!searchParams.has("stack"));
	const changeLocation = useCallback(
		(patch: Record<string, string | null>, push = false) => {
			const href = stackLocationHref(
				window.location.pathname,
				new URLSearchParams(window.location.search),
				patch,
			);
			if (push) window.history.pushState(null, "", href);
			else window.history.replaceState(null, "", href);
		},
		[],
	);
	const setQuery = (value: string) =>
		changeLocation({ q: value.slice(0, 160) || null });
	const setFilter = (value: StackSupport | "all") =>
		changeLocation({ filter: value === "all" ? null : value }, true);
	const next = useCallback(
		() =>
			changeLocation({
				stack: stacks[(index + 1) % stacks.length].id,
				workflow: "0",
			}),
		[index, changeLocation],
	);
	const previous = useCallback(
		() =>
			changeLocation({
				stack: stacks[(index - 1 + stacks.length) % stacks.length].id,
				workflow: "0",
			}),
		[index, changeLocation],
	);
	const selected = stacks[index];
	const results = stacks.filter(
		(stack) =>
			(filter === "all" || stack.support === filter) &&
			`${stack.name} ${stack.icon} ${t(`categories.${stack.category}`)}`
				.toLocaleLowerCase()
				.includes(query.trim().toLocaleLowerCase()),
	);
	return (
		<Dialog
			open={location.open}
			onOpenChange={(open) => {
				if (!open) changeLocation({ workflow: "0" });
			}}
		>
			<div id="stack-preview" className="stack-catalog-preview">
				<StackPlayer
					stack={selected}
					playing={playing}
					onToggle={() => setPlaying((p) => !p)}
					onNext={next}
					onPrevious={previous}
				/>
			</div>
			<section
				className="route-section"
				aria-labelledby="stack-directory-title"
			>
				<div className="route-section-heading">
					<div>
						<p className="section-eyebrow">{t("directoryEyebrow")}</p>
						<h2 id="stack-directory-title">{t("directoryTitle")}</h2>
					</div>
					<span className="stack-catalog-count" aria-live="polite">
						{t("count", { count: results.length })}
					</span>
				</div>
				<div className="stack-catalog-tools">
					<div className="stack-search">
						<Search size={17} aria-hidden="true" />
						<Input
							aria-label={t("search")}
							placeholder={t("search")}
							value={query}
							onChange={(e) => setQuery(e.target.value)}
						/>
						{query && (
							<Button
								variant="ghost"
								size="icon"
								aria-label={t("clear")}
								onClick={() => setQuery("")}
							>
								<X size={15} />
							</Button>
						)}
					</div>
					<fieldset className="stack-filters" aria-label={t("filterLabel")}>
						{(["all", "detected", "script", "custom"] as const).map((value) => (
							<Button
								key={value}
								variant={value === filter ? "secondary" : "ghost"}
								aria-pressed={value === filter}
								onClick={() => setFilter(value)}
							>
								{t(value === "all" ? "all" : `support.${value}`)}
							</Button>
						))}
					</fieldset>
				</div>
				<p className="stack-directory-note">{t("supportNote")}</p>
				<div className="stack-catalog-grid">
					{results.map((stack) => (
						<article
							key={stack.id}
							className="stack-catalog-card"
							data-selected={selected.id === stack.id}
						>
							<div className="stack-card-heading">
								<StackIcon stack={stack} className="size-11" />
								<span className="stack-support" data-support={stack.support}>
									{t(`support.${stack.support}`)}
								</span>
							</div>
							<h3>{stack.name}</h3>
							<p>{t(`categories.${stack.category}`)}</p>
							<div className="stack-card-actions">
								<DialogTrigger
									render={
										<Button
											variant="ghost"
											onClick={() => {
												changeLocation(
													{ stack: stack.id, workflow: "1" },
													true,
												);
												setPlaying(false);
											}}
											aria-label={`${t("viewWorkflow")}: ${stack.name}`}
										/>
									}
								>
									{t("viewWorkflow")}
									<ArrowRight size={14} />
								</DialogTrigger>
								<a
									href={stack.docs}
									aria-label={`${t("officialDocs")}: ${stack.name}`}
									target="_blank"
									rel="noopener noreferrer"
								>
									<ArrowUpRight size={16} />
								</a>
							</div>
						</article>
					))}
				</div>
				{results.length === 0 && (
					<div className="stack-empty">
						<Search size={28} />
						<h3>{t("emptyTitle")}</h3>
						<p>{t("emptyBody")}</p>
						<Button
							variant="outline"
							onClick={() => {
								setQuery("");
								setFilter("all");
							}}
						>
							{t("reset")}
						</Button>
					</div>
				)}
			</section>
			<section
				className="stack-request-panel"
				aria-labelledby="stack-request-title"
			>
				<div>
					<p className="section-eyebrow">{t("requestEyebrow")}</p>
					<h2 id="stack-request-title">{t("requestTitle")}</h2>
					<p>{t("requestBody")}</p>
				</div>
				<a
					className="btn-ember"
					href={frameworkRequestUrl(query.trim() || undefined)}
					target="_blank"
					rel="noopener noreferrer"
				>
					{t("request")}
					<ArrowUpRight size={16} />
				</a>
			</section>
			<div className="stack-catalog-help">
				<p>{t("scopeNote")}</p>
				<Link href={route("docs.$slug", { slug: "sites" })}>
					{t("setup")}
					<ArrowRight size={14} />
				</Link>
			</div>
			<StackWorkflowDialog stack={selected} />
		</Dialog>
	);
}

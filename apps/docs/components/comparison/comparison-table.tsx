"use client";

import {
	ArrowDown,
	ArrowUpRight,
	Check,
	ChevronDown,
	Clock3,
	HelpCircle,
	Minus,
	Search,
	X,
} from "lucide-react";
import Image from "next/image";
import { useSearchParams } from "next/navigation";
import { useFormatter, useTranslations } from "next-intl";
import { useRef, useState } from "react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import {
	groups,
	reviewedOn,
	type Status,
	type ToolId,
	tools,
} from "@/lib/comparison-matrix";
import {
	brandImages,
	comparisonViews,
	competitors,
	isLocdevAdvantage,
	matchesComparisonView,
	readComparisonLocation,
} from "@/lib/comparison-view";

const statusIcons = {
	yes: Check,
	partial: Minus,
	no: X,
	planned: Clock3,
	unknown: HelpCircle,
};

function Brand({ id, large = false }: { id: ToolId; large?: boolean }) {
	return (
		<span className={`compare-brand ${large ? "compare-brand-large" : ""}`}>
			<Image
				src={brandImages[id]}
				alt=""
				width={large ? 72 : 28}
				height={large ? 72 : 28}
				unoptimized
			/>
		</span>
	);
}

function StatusValue({ status, name }: { status: Status; name: string }) {
	const t = useTranslations("ComparisonMatrix");
	const Icon = statusIcons[status];
	return (
		<span className="compare-status" data-status={status}>
			<span className="compare-status-icon">
				<Icon size={17} aria-hidden="true" />
			</span>
			<span>
				<span className="compare-mobile-name">{name}</span>
				{t(`status.${status}`)}
			</span>
		</span>
	);
}

export function ComparisonTable() {
	const t = useTranslations("ComparisonMatrix");
	const format = useFormatter();
	const params = useSearchParams();
	const { competitor, view, category, query } = readComparisonLocation(
		new URLSearchParams(params),
	);
	const [draft, setDraft] = useState<string | null>(null);
	const input = useRef<HTMLInputElement>(null);
	const change = (patch: Record<string, string | null>, push = false) => {
		const url = new URL(window.location.href);
		for (const [key, value] of Object.entries(patch)) {
			if (value) url.searchParams.set(key, value);
			else url.searchParams.delete(key);
		}
		window.history[push ? "pushState" : "replaceState"](
			null,
			"",
			`${url.pathname}${url.search}${url.hash}`,
		);
	};
	const setQuery = (value: string) =>
		change({ q: value.slice(0, 160) || null });
	const term = query.trim().toLocaleLowerCase();
	const filtered = groups
		.filter((group) => category === "all" || category === group.id)
		.map((group) => ({
			...group,
			features: group.features.filter(
				(feature) =>
					matchesComparisonView(feature, competitor.id, view) &&
					`${t(`features.${feature.id}.label`)} ${t(`features.${feature.id}.description`)} ${t(`groups.${group.id}.title`)}`
						.toLocaleLowerCase()
						.includes(term),
			),
		}))
		.filter((group) => group.features.length);
	const count = filtered.reduce((sum, group) => sum + group.features.length, 0);
	const total = groups.reduce((sum, group) => sum + group.features.length, 0);
	const locdev = tools[0];

	return (
		<div className="compare-explorer">
			<section
				className="compare-picker"
				aria-labelledby="compare-picker-title"
			>
				<div className="compare-section-line">
					<h2 id="compare-picker-title">{t("experience.choose")}</h2>
					<span className="compare-caption">
						{t("experience.alternatives", { count: competitors.length })}
					</span>
				</div>
				<div
					className="compare-apps"
					role="group"
					aria-label={t("experience.choose")}
				>
					{competitors.map((tool) => (
						<button
							key={tool.id}
							type="button"
							className="compare-app"
							aria-pressed={competitor.id === tool.id}
							onClick={() =>
								change({ app: tool.id === "herd" ? null : tool.id }, true)
							}
						>
							<Brand id={tool.id} />
							<span>{tool.name}</span>
							{competitor.id === tool.id && (
								<Check
									className="compare-app-check"
									size={13}
									aria-hidden="true"
								/>
							)}
						</button>
					))}
				</div>
			</section>

			<section
				className="compare-matchup"
				aria-label={t("experience.pair", { name: competitor.name })}
			>
				<div className="compare-contender compare-contender-locdev">
					<span className="compare-caption">
						{t("experience.yourEnvironment")}
					</span>
					<Brand id="locdev" large />
					<h2>LocDev</h2>
					<p>{t("experience.tools.locdev")}</p>
					<a href={locdev.url} target="_blank" rel="noopener noreferrer">
						{t("experience.officialSite")}
						<ArrowUpRight size={14} aria-hidden="true" />
					</a>
				</div>
				<span className="compare-versus" aria-hidden="true">
					vs
				</span>
				<div className="compare-contender">
					<span className="compare-caption">{t("experience.selected")}</span>
					<Brand id={competitor.id} large />
					<h2>{competitor.name}</h2>
					<p>{t(`experience.tools.${competitor.id}`)}</p>
					<a href={competitor.url} target="_blank" rel="noopener noreferrer">
						{t("experience.officialSite")}
						<ArrowUpRight size={14} aria-hidden="true" />
					</a>
				</div>
				<div className="compare-matchup-foot">
					<span>
						{t("experience.overview", { count: total, groups: groups.length })}
					</span>
					<a href="#compare-results">
						{t("experience.explore")}
						<ArrowDown size={14} aria-hidden="true" />
					</a>
				</div>
			</section>

			<section
				id="compare-results"
				className="compare-results"
				aria-labelledby="compare-results-title"
			>
				<div className="compare-results-heading">
					<div>
						<p className="section-eyebrow">{t("experience.resultsEyebrow")}</p>
						<h2 id="compare-results-title">
							{t("experience.pair", { name: competitor.name })}
						</h2>
					</div>
					<p className="compare-caption">
						{t("ui.reviewed", {
							date: format.dateTime(new Date(`${reviewedOn}T12:00:00Z`), {
								dateStyle: "medium",
								timeZone: "UTC",
							}),
						})}
					</p>
				</div>
				<p className="compare-results-intro">{t("experience.resultsBody")}</p>
				<div className="compare-toolbar">
					<div className="compare-search">
						<Search size={17} aria-hidden="true" />
						<Input
							ref={input}
							type="search"
							aria-label={t("ui.searchLabel")}
							placeholder={t("ui.search")}
							value={draft ?? query}
							onCompositionStart={(event) =>
								setDraft(event.currentTarget.value)
							}
							onCompositionEnd={(event) => {
								setDraft(null);
								setQuery(event.currentTarget.value);
							}}
							onChange={(event) => {
								if (draft !== null) setDraft(event.target.value);
								else setQuery(event.target.value);
							}}
						/>
						{(draft ?? query) && (
							<Button
								variant="ghost"
								size="icon"
								aria-label={t("experience.clearSearch")}
								onClick={() => {
									setDraft(null);
									setQuery("");
									input.current?.focus();
								}}
							>
								<X size={16} />
							</Button>
						)}
					</div>
					<div
						className="compare-view-filters"
						role="group"
						aria-label={t("experience.filterLabel")}
					>
						{comparisonViews.map((item) => (
							<Button
								key={item}
								variant={item === view ? "secondary" : "ghost"}
								aria-pressed={item === view}
								onClick={() =>
									change({ view: item === "all" ? null : item }, true)
								}
							>
								{t(`experience.views.${item}`)}
							</Button>
						))}
					</div>
				</div>
				<div
					className="compare-categories"
					role="group"
					aria-label={t("experience.categoryLabel")}
				>
					{["all", ...groups.map((group) => group.id)].map((id) => (
						<button
							key={id}
							type="button"
							aria-pressed={category === id}
							onClick={() =>
								change({ category: id === "all" ? null : id }, true)
							}
						>
							{id === "all"
								? t("experience.allCategories")
								: t(`groups.${id}.title`)}
						</button>
					))}
				</div>
				<div className="compare-result-meta">
					<p role="status" aria-live="polite">
						{t("experience.resultCount", { count, total })}
					</p>
					<span>{t("experience.detailsHint")}</span>
				</div>
				<div className="compare-grid-head" aria-hidden="true">
					<span>{t("ui.feature")}</span>
					<span>
						<Brand id="locdev" />
						LocDev
					</span>
					<span>
						<Brand id={competitor.id} />
						{competitor.name}
					</span>
				</div>
				<div key={competitor.id} className="compare-feature-groups">
					{filtered.map((group) => (
						<section
							key={group.id}
							className="compare-feature-group"
							aria-labelledby={`compare-group-${group.id}`}
						>
							<div className="compare-group-title">
								<h3 id={`compare-group-${group.id}`}>
									{t(`groups.${group.id}.title`)}
								</h3>
								<span>{format.number(group.features.length)}</span>
							</div>
							<div>
								{group.features.map((feature) => (
									<details
										key={feature.id}
										className="compare-feature"
										data-advantage={isLocdevAdvantage(feature, competitor.id)}
									>
										<summary className="compare-feature-summary">
											<span className="compare-feature-name">
												<span>{t(`features.${feature.id}.label`)}</span>
												<span className="compare-feature-description">
													{t(`features.${feature.id}.description`)}
												</span>
												<span className="compare-detail-cue">
													<ChevronDown size={13} aria-hidden="true" />
													{t("experience.details")}
												</span>
											</span>
											<StatusValue
												status={feature.cells.locdev.status}
												name="LocDev"
											/>
											<StatusValue
												status={feature.cells[competitor.id].status}
												name={competitor.name}
											/>
										</summary>
										<div className="compare-feature-detail">
											{[locdev, competitor].map((tool) => (
												<div key={tool.id}>
													<h4>
														<Brand id={tool.id} />
														{tool.name}
													</h4>
													{t.has(
														`notes.${group.id}.${feature.id}.${tool.id}`,
													) && (
														<p>
															{t(`notes.${group.id}.${feature.id}.${tool.id}`)}
														</p>
													)}
													<a
														href={feature.cells[tool.id].source}
														target="_blank"
														rel="noopener noreferrer"
													>
														{t("experience.checkSource")}
														<ArrowUpRight size={13} aria-hidden="true" />
													</a>
												</div>
											))}
										</div>
									</details>
								))}
							</div>
						</section>
					))}
				</div>
				{count === 0 && (
					<div className="compare-empty">
						<Search size={25} aria-hidden="true" />
						<h3>{t("ui.empty")}</h3>
						<p>{t("experience.emptyHelp")}</p>
						<Button
							variant="outline"
							onClick={() => {
								setDraft(null);
								change({ q: null, view: null, category: null });
							}}
						>
							{t("experience.reset")}
						</Button>
					</div>
				)}
				<div className="compare-legend">
					{(Object.keys(statusIcons) as Status[]).map((status) => {
						const Icon = statusIcons[status];
						return (
							<span key={status} data-status={status}>
								<Icon size={14} aria-hidden="true" />
								{t(`status.${status}`)}
							</span>
						);
					})}
				</div>
				<p className="compare-method-note">{t("experience.filterNote")}</p>
			</section>
		</div>
	);
}

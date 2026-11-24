"use client";
import { ArrowRight, ArrowUpRight, Search, X } from "lucide-react";
import { useTranslations } from "next-intl";
import { useState } from "react";
import { Button } from "@/components/ui/button";
import { ChoiceSelect } from "@/components/ui/choice-select";
import {
	Dialog,
	DialogContent,
	DialogDescription,
	DialogTitle,
	DialogTrigger,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import {
	getStackServices,
	locdevSupport,
	serviceRequestUrl,
	services,
} from "@/lib/stack-services";
import { stacks } from "@/lib/stacks";
import { ServiceDetails } from "./service-details";
import { ServiceIcon } from "./service-icon";

const categories = [
	"all",
	"database",
	"background",
	"realtime",
	"tooling",
] as const;
export function ServiceCatalog() {
	const t = useTranslations("Services");
	const [query, setQuery] = useState("");
	const [category, setCategory] = useState<string>("all");
	const [stackId, setStackId] = useState("all");
	const [selected, setSelected] = useState(services[0]);
	const stack = stacks.find((s) => s.id === stackId);
	const allowed = stack ? getStackServices(stack.id) : services;
	const results = allowed.filter(
		(s) =>
			(category === "all" || s.category === category) &&
			`${s.name} ${s.requires}`
				.toLocaleLowerCase()
				.includes(query.trim().toLocaleLowerCase()),
	);
	const related = stacks.filter((s) =>
		getStackServices(s.id).some((item) => item.id === selected.id),
	);
	return (
		<section
			className="service-directory"
			aria-labelledby="service-directory-title"
		>
			<div className="section-topline">
				<h2 id="service-directory-title">{t("directoryTitle")}</h2>
				<span className="stack-catalog-count" role="status">
					{t("count", { count: results.length })}
				</span>
			</div>
			<div className="stack-catalog-tools">
				<div className="stack-search">
					<Search size={17} />
					<Input
						value={query}
						onChange={(e) => setQuery(e.target.value)}
						aria-label={t("search")}
						placeholder={t("search")}
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
				<div className="service-stack-picker">
					<ChoiceSelect
						id="service-stack"
						label={t("select")}
						value={stackId}
						onChange={setStackId}
						options={[
							{ value: "all", label: t("allStacks") },
							...stacks.map((s) => ({ value: s.id, label: s.name })),
						]}
					/>
				</div>
			</div>
			<fieldset className="stack-filters" aria-label={t("filterLabel")}>
				{categories.map((value) => (
					<Button
						key={value}
						variant={category === value ? "secondary" : "ghost"}
						aria-pressed={category === value}
						onClick={() => setCategory(value)}
					>
						{t(value === "all" ? "all" : `categories.${value}`)}
					</Button>
				))}
			</fieldset>
			<p className="stack-directory-note">{t("scope")}</p>
			<Dialog>
				<div className="stack-catalog-grid service-catalog-grid">
					{results.map((service) => (
						<article
							className="stack-catalog-card service-catalog-card"
							key={service.id}
						>
							<div className="stack-card-heading">
								<ServiceIcon service={service} />
								<SupportBadge id={service.id} />
							</div>
							<h3>{service.name}</h3>
							<p>{t(`categories.${service.category}`)}</p>
							<span className="service-mode">{t(`modes.${service.mode}`)}</span>
							<div className="stack-card-actions">
								<DialogTrigger
									render={
										<Button
											variant="ghost"
											onClick={() => setSelected(service)}
											aria-label={`${t("viewDetails")}: ${service.name}`}
										/>
									}
								>
									{t("viewDetails")}
									<ArrowRight size={14} />
								</DialogTrigger>
								<a
									href={service.docs}
									aria-label={`${t("docs")}: ${service.name}`}
									target="_blank"
									rel="noopener noreferrer"
								>
									<ArrowUpRight size={16} />
								</a>
							</div>
						</article>
					))}
				</div>
				<DialogContent className="service-dialog" closeLabel={t("close")}>
					<div className="service-dialog-heading">
						<ServiceIcon service={selected} />
						<div>
							<p className={supportClass(selected.id)}>
								{t(`support.${locdevSupport(selected.id)}`)}
							</p>
							<DialogTitle>{selected.name}</DialogTitle>
						</div>
					</div>
					<DialogDescription>
						{t(`categories.${selected.category}`)} ·{" "}
						{t(`modes.${selected.mode}`)}
					</DialogDescription>
					<ServiceDetails service={selected} />
					<h3>{t("relatedStacks")}</h3>
					<div className="service-related-stacks">
						{related.map((s) => (
							<span key={s.id}>{s.name}</span>
						))}
					</div>
					<a
						className="btn-outline"
						href={serviceRequestUrl(selected.name)}
						target="_blank"
						rel="noopener noreferrer"
					>
						{t("request")}
						<ArrowUpRight size={14} />
					</a>
				</DialogContent>
			</Dialog>
			{!results.length && (
				<div className="service-empty">
					<h3>{t("empty")}</h3>
					<Button
						variant="outline"
						onClick={() => {
							setQuery("");
							setCategory("all");
							setStackId("all");
						}}
					>
						{t("reset")}
					</Button>
				</div>
			)}
			<p className="service-footnote">{t("note")}</p>
			<p className="service-footnote">{t("logoNote")}</p>
		</section>
	);
}

function supportClass(id: string) {
	return locdevSupport(id) === "proposed"
		? "service-planned"
		: "service-available";
}

function SupportBadge({ id }: { id: string }) {
	const t = useTranslations("Services");
	return (
		<span className={supportClass(id)}>
			{t(`supportShort.${locdevSupport(id)}`)}
		</span>
	);
}

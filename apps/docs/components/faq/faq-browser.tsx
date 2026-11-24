"use client";
import {
	ArrowRight,
	BookOpen,
	CircleHelp,
	Search,
	Wrench,
	X,
} from "lucide-react";
import { useTranslations } from "next-intl";
import { useState } from "react";
import {
	Accordion,
	AccordionContent,
	AccordionItem,
	AccordionTrigger,
} from "@/components/ui/accordion";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Link } from "@/i18n/navigation";
import { faqMatches } from "@/lib/faq-search";
import { route } from "@/lib/routes.generated";
import { FAQAnswer } from "./faq-answer";

type Group = {
	id: "basics" | "daily";
	items: {
		id: string;
		question: string;
		answer: unknown;
		href: string;
		action: string;
	}[];
};
export function FaqBrowser({ groups }: { groups: readonly Group[] }) {
	const t = useTranslations("FaqPage");
	const n = useTranslations("PageImprovements");
	const [query, setQuery] = useState("");
	const results = groups.map((group) => ({
		...group,
		items: group.items.filter((item) =>
			faqMatches(item.question, item.answer, query),
		),
	}));
	const count = results.reduce((total, group) => total + group.items.length, 0);
	return (
		<>
			<div className="faq-search">
				<Search size={18} aria-hidden="true" />
				<Input
					type="search"
					value={query}
					onChange={(event) => setQuery(event.target.value.slice(0, 160))}
					aria-label={n("faqSearch")}
					placeholder={n("faqSearch")}
				/>
				{query && (
					<Button
						variant="ghost"
						size="icon"
						aria-label={n("clear")}
						onClick={() => setQuery("")}
					>
						<X size={16} />
					</Button>
				)}
			</div>
			<p className="faq-results-count" role="status">
				{n("faqCount", { count })}
			</p>
			<div className="faq-page-layout">
				<nav className="faq-page-outline" aria-label={t("topics")}>
					{results
						.filter((group) => group.items.length > 0)
						.map((group) => {
							const Icon = group.id === "basics" ? CircleHelp : Wrench;
							return (
								<a key={group.id} href={`#${group.id}`}>
									<Icon size={17} aria-hidden="true" />
									{t(group.id)}
									<span>{group.items.length}</span>
								</a>
							);
						})}
					<Link href={route("docs")}>
						<BookOpen size={17} aria-hidden="true" />
						{t("docs")}
					</Link>
				</nav>
				<div className="faq-page-groups">
					{count ? (
						results
							.filter((group) => group.items.length)
							.map((group) => (
								<section
									key={group.id}
									id={group.id}
									className="faq-page-group"
								>
									<h2>{t(group.id)}</h2>
									<Accordion className="faq-list" multiple>
										{group.items.map((item) => (
											<AccordionItem
												key={item.id}
												value={item.id}
												className="faq-item"
											>
												<AccordionTrigger className="hover:no-underline">
													<strong>{item.question}</strong>
												</AccordionTrigger>
												<AccordionContent>
													<FAQAnswer value={item.answer} />
													<Link href={item.href} className="faq-answer-link">
														{item.action}
														<ArrowRight size={14} />
													</Link>
												</AccordionContent>
											</AccordionItem>
										))}
									</Accordion>
								</section>
							))
					) : (
						<div className="tool-empty">
							<h2>{n("faqEmpty")}</h2>
							<p>{n("faqEmptyBody")}</p>
							<Button variant="outline" onClick={() => setQuery("")}>
								{n("clear")}
							</Button>
						</div>
					)}
				</div>
			</div>
		</>
	);
}

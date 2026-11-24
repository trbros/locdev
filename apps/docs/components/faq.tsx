import { ArrowUpRight } from "lucide-react";
import { getTranslations } from "next-intl/server";
import {
	Accordion,
	AccordionContent,
	AccordionItem,
	AccordionTrigger,
} from "@/components/ui/accordion";
import { Link } from "@/i18n/navigation";
import { route } from "@/lib/routes.generated";
import { FAQAnswer } from "./faq/faq-answer";

const links = [
	"/license",
	route("docs.$slug", { slug: "architecture" }),
	route("docs.$slug", { slug: "installation" }),
	"/download",
] as const;

export async function FAQ() {
	const t = await getTranslations("FAQ");
	const nav = await getTranslations("Nav");
	return (
		<section className="section-pad border-t border-border">
			<div className="page-container grid gap-10 lg:grid-cols-[.8fr_1.2fr]">
				<div>
					<p className="section-eyebrow">{t("eyebrow")}</p>
					<h2 className="section-heading mt-5 max-w-100">{t("title")}</h2>
					<Link href="/faq" className="faq-home-link">
						{nav("faq")} <ArrowUpRight size={15} />
					</Link>
				</div>
				<Accordion className="faq-list">
					{([1, 2, 3, 4] as const).map((i) => (
						<AccordionItem key={i} value={String(i)} className="faq-item">
							<AccordionTrigger className="hover:no-underline">
								<span>{String(i).padStart(2, "0")}</span>
								<strong>{t(`q${i}`)}</strong>
							</AccordionTrigger>
							<AccordionContent>
								<FAQAnswer value={t.raw(`a${i}`)} />
								<Link className="faq-answer-link" href={links[i - 1]}>
									{t(`link${i}`)} <ArrowUpRight size={14} />
								</Link>
							</AccordionContent>
						</AccordionItem>
					))}
				</Accordion>
			</div>
		</section>
	);
}

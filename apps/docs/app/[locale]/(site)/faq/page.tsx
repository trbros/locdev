import { ArrowRight, LifeBuoy } from "lucide-react";
import type { Metadata } from "next";
import { getTranslations } from "next-intl/server";
import { FaqBrowser } from "@/components/faq/faq-browser";

import { Link } from "@/i18n/navigation";
import { route } from "@/lib/routes.generated";

export async function generateMetadata(): Promise<Metadata> {
	const t = await getTranslations("FaqPage");
	return { title: t("title"), description: t("body") };
}

export default async function FaqPage() {
	const t = await getTranslations("FaqPage");
	const common = await getTranslations("FAQ");
	const basicLinks = [
		"/license",
		route("docs.$slug", { slug: "architecture" }),
		route("docs.$slug", { slug: "installation" }),
		"/download",
	] as const;
	const groups = [
		{
			id: "basics",
			items: ([1, 2, 3, 4] as const).map((i) => ({
				id: `question-${i}`,
				question: common(`q${i}`),
				answer: common.raw(`a${i}`) as unknown,
				href: basicLinks[i - 1],
				action: common(`link${i}`),
			})),
		},
		{
			id: "daily",
			items: (
				[
					{ id: "services", href: "/services" },
					{
						id: "offline",
						href: route("docs.$slug", { slug: "architecture" }),
					},
					{ id: "updates", href: route("docs.$slug", { slug: "updating" }) },
					{
						id: "help",
						href: route("docs.$slug", { slug: "troubleshooting" }),
					},
				] as const
			).map(({ id, href }) => ({
				id,
				href,
				question: t(`${id}Question`),
				answer: t.raw(`${id}Answer`) as unknown,
				action: t(`${id}Action`),
			})),
		},
	] as const;

	return (
		<div className="page-container editorial-page faq-page">
			<header className="route-hero">
				<p className="section-eyebrow">{t("eyebrow")}</p>
				<h1 className="section-heading">{t("title")}</h1>
				<p className="route-description">{t("body")}</p>
			</header>
			<FaqBrowser groups={groups} />
			<section className="faq-page-help">
				<LifeBuoy size={25} aria-hidden="true" />
				<div>
					<h2>{t("supportTitle")}</h2>
					<p>{t("supportBody")}</p>
				</div>
				<Link href="/support" className="btn-outline">
					{t("supportAction")}
					<ArrowRight size={16} />
				</Link>
			</section>
		</div>
	);
}

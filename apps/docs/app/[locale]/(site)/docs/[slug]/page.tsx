import { ArrowLeft, ArrowRight, BookOpen, ChevronRight } from "lucide-react";
import type { Metadata } from "next";
import { notFound } from "next/navigation";
import { DocArticle } from "@/components/docs/doc-article";
import { DocsSidebar } from "@/components/docs/docs-sidebar";
import { DocsToc } from "@/components/docs/docs-toc";
import { Link } from "@/i18n/navigation";
import { routing } from "@/i18n/routing";
import { getDocHeadings } from "@/lib/doc-headings";
import { docSlugs, getDoc, getDocTitle, isDocSlug } from "@/lib/docs";
import { getDocLabels } from "@/lib/docs-labels";
import { route } from "@/lib/routes.generated";

export function generateStaticParams() {
	return routing.locales.flatMap((locale) =>
		docSlugs.map((slug) => ({ locale, slug })),
	);
}
export async function generateMetadata({
	params,
}: {
	params: Promise<{ locale: string; slug: string }>;
}): Promise<Metadata> {
	const { locale, slug } = await params;
	if (!isDocSlug(slug)) notFound();
	const content = await getDoc(locale, slug);
	return {
		title: getDocTitle(content),
		description: getDocLabels(locale).descriptions[slug],
	};
}
export default async function DocsDetail({
	params,
}: {
	params: Promise<{ locale: string; slug: string }>;
}) {
	const { locale, slug } = await params;
	if (!isDocSlug(slug)) notFound();
	const labels = getDocLabels(locale);
	const content = await getDoc(locale, slug);
	const headings = getDocHeadings(content);
	const tocLabel = (
		{
			"pt-BR": "Nesta página",
			en: "On this page",
			es: "En esta página",
			ja: "このページの内容",
		} as Record<string, string>
	)[locale];
	const index = docSlugs.indexOf(slug);
	const previous = index > 0 ? docSlugs[index - 1] : null;
	const next = index < docSlugs.length - 1 ? docSlugs[index + 1] : null;
	return (
		<div className="page-container docs-detail-shell">
			<DocsSidebar locale={locale} slug={slug} />
			<div className="docs-main-column">
				<div className="docs-breadcrumb">
					<Link href={route("docs")}>
						<BookOpen size={14} />
						{labels.all}
					</Link>
					<ChevronRight size={13} />
					<span>{labels.titles[slug]}</span>
				</div>
				<DocArticle content={content} locale={locale} />
				<div className="docs-page-pager">
					{previous ? (
						<Link href={route("docs.$slug", { slug: previous })}>
							<ArrowLeft size={17} />
							<span>
								<small>{labels.previous}</small>
								<strong>{labels.titles[previous]}</strong>
							</span>
						</Link>
					) : (
						<span />
					)}
					{next ? (
						<Link href={route("docs.$slug", { slug: next })}>
							<span>
								<small>{labels.next}</small>
								<strong>{labels.titles[next]}</strong>
							</span>
							<ArrowRight size={17} />
						</Link>
					) : (
						<span />
					)}
				</div>
			</div>
			{headings.length > 0 && (
				<DocsToc
					key={`${locale}-${slug}`}
					headings={headings}
					label={tocLabel}
				/>
			)}
		</div>
	);
}

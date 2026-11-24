import { ArrowUpRight, BookOpen, LifeBuoy } from "lucide-react";
import { getTranslations } from "next-intl/server";
import { docIcons } from "@/components/docs/doc-icons";
import { DocsNavScroll } from "@/components/docs/docs-nav-scroll";
import { DocsVersionSelector } from "@/components/docs/docs-version-selector";
import { Link } from "@/i18n/navigation";
import { appVersion } from "@/lib/app-version";
import { docGroups } from "@/lib/docs";
import { getDocLabels } from "@/lib/docs-labels";
import { repository } from "@/lib/repository";
import { route } from "@/lib/routes.generated";

export async function DocsSidebar({
	locale,
	slug,
}: {
	locale: string;
	slug?: string;
}) {
	const labels = getDocLabels(locale);
	const versionLabels = await getTranslations("DocsVersion");
	const support = await getTranslations("Support");
	const navigation = (
		<nav aria-label={labels.all} className="docs-sidebar-navigation">
			<DocsVersionSelector
				version={appVersion}
				label={versionLabels("label")}
				current={versionLabels("current")}
			/>
			<DocsNavScroll current={slug ?? "index"}>
				<Link
					href={route("docs")}
					className="docs-back"
					data-doc-slug="index"
					aria-current={!slug ? "page" : undefined}
				>
					<BookOpen size={16} />
					{labels.all}
				</Link>
				{docGroups.map((group) => (
					<div key={group.id} className="docs-detail-navgroup">
						<p>{labels.groups[group.id]}</p>
						{group.slugs.map((item) => {
							const Icon = docIcons[item];
							return (
								<Link
									href={route("docs.$slug", { slug: item })}
									key={item}
									data-doc-slug={item}
									aria-current={item === slug ? "page" : undefined}
									className={`docs-detail-link ${item === slug ? "docs-detail-link-active" : ""}`}
								>
									<Icon size={15} aria-hidden="true" />
									{labels.titles[item]}
								</Link>
							);
						})}
					</div>
				))}
				<div className="docs-sidebar-resources">
					<Link href="/support">
						<LifeBuoy size={16} />
						{support("title")}
						<ArrowUpRight size={13} />
					</Link>
					<a href={repository.url} target="_blank" rel="noreferrer">
						<BookOpen size={16} />
						GitHub
						<ArrowUpRight size={13} />
					</a>
				</div>
			</DocsNavScroll>
		</nav>
	);
	return (
		<aside className="docs-detail-sidebar">
			<div className="docs-detail-sticky docs-desktop-nav">{navigation}</div>
			<details className="docs-mobile-nav" key={slug ?? "index"}>
				<summary>
					{slug
						? labels.titles[slug as keyof typeof labels.titles]
						: labels.all}
				</summary>
				{navigation}
			</details>
		</aside>
	);
}

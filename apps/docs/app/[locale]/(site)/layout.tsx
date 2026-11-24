import type { ReactNode } from "react";
import { SiteFooter } from "@/components/site-footer";
import { SiteHeader } from "@/components/site-header";
import { getDocsSearchIndex } from "@/lib/docs-search-index";
export default async function SiteLayout({
	children,
	params,
}: {
	children: ReactNode;
	params: Promise<{ locale: string }>;
}) {
	const { locale } = await params;
	return (
		<div className="site-shell">
			<SiteHeader searchEntries={await getDocsSearchIndex(locale)} />
			<main id="main-content" tabIndex={-1}>
				{children}
			</main>
			<SiteFooter />
		</div>
	);
}

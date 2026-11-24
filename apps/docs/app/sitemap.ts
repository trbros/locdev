import type { MetadataRoute } from "next";
import { languageTags, localizedPathname } from "@/i18n/locales";
import { routing } from "@/i18n/routing";
import { docSlugs } from "@/lib/docs";
import { route } from "@/lib/routes.generated";
import { privacyConfirmed, siteInformation } from "@/lib/site-information";

const origin =
	process.env.NEXT_PUBLIC_SITE_URL?.replace(/\/$/, "") ??
	"http://localhost:3000";

export default function sitemap(): MetadataRoute.Sitemap {
	const pages = [
		"",
		"/features",
		"/compare",
		"/stacks",
		"/services",
		"/download",
		"/get-started",
		"/configure",
		"/changelog",
		"/contribute",
		"/about",
		"/contact",
		"/support",
		"/faq",
		...(privacyConfirmed ? ["/privacy"] : []),
		...(siteInformation.securityContact ? ["/security"] : []),
		"/license",
		"/donate",
		route("docs"),
		...docSlugs.map((slug) => route("docs.$slug", { slug })),
	];
	return pages.flatMap((path) =>
		routing.locales.map((locale) => ({
			url: `${origin}${localizedPathname(path || "/", locale)}`,
			changeFrequency: "weekly" as const,
			priority: path === "" ? 1 : path === "/docs" ? 0.8 : 0.7,
			alternates: {
				languages: Object.fromEntries(
					routing.locales.map((l) => [
						languageTags[l],
						`${origin}${localizedPathname(path || "/", l)}`,
					]),
				),
			},
		})),
	);
}

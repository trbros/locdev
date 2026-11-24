import { getTranslations } from "next-intl/server";
export async function FeatureSectionNav() {
	const t = await getTranslations("PageImprovements");
	return (
		<nav
			className="feature-section-nav page-container"
			aria-label={t("featureSections")}
		>
			{["architecture", "workflows", "control"].map((id) => (
				<a key={id} href={`#${id}`}>
					{t(id)}
				</a>
			))}
		</nav>
	);
}

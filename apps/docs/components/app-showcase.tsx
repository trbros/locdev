import { getTranslations } from "next-intl/server";
import { ThemedShot } from "@/components/themed-shot";

export async function AppShowcase({ hero = false }: { hero?: boolean }) {
	const t = await getTranslations("Gallery");
	return (
		<div className={`app-demo app-capture ${hero ? "app-demo-hero" : ""}`}>
			<ThemedShot
				name="sites"
				alt={t("sites")}
				width={1280}
				height={800}
				sizes={
					hero
						? "(max-width: 1024px) 100vw, 60vw"
						: "(max-width: 768px) 100vw, 1000px"
				}
				priority={hero}
			/>
		</div>
	);
}

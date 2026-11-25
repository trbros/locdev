import { ArrowUpRight } from "lucide-react";
import { getTranslations } from "next-intl/server";
import { Link } from "@/i18n/navigation";
import { route } from "@/lib/routes.generated";
import { Screenshot } from "./screenshot";

export async function ScreenshotGallery() {
	const t = await getTranslations("Gallery");
	const l = await getTranslations("Labels");
	return (
		<section className="section-pad border-t border-border bg-chrome/30">
			<div className="page-container">
				<div className="section-topline">
					<p className="section-eyebrow">{t("eyebrow")}</p>
					<span className="font-mono text-xs text-muted-foreground">
						{l("galleryTag")}
					</span>
				</div>
				<div className="section-intro">
					<h2 className="section-heading">{t("title")}</h2>
					<p className="section-description">{t("body")}</p>
				</div>
				<div className="screens-gallery mt-12">
					<div className="screens-gallery-item">
						<Screenshot name="sites" alt={t("sites")} />
					</div>
					<div className="screens-gallery-item">
						<Screenshot name="runtimes" alt={t("runtimes")} />
					</div>
					<div className="screens-gallery-item">
						<Screenshot name="doctor" alt={t("doctor")} />
					</div>
				</div>
				<div className="mt-8 flex justify-end">
					<Link
						href={route("docs.$slug", { slug: "app" })}
						className="inline-flex gap-2 text-ember text-sm font-semibold items-center"
					>
						{t("link")} <ArrowUpRight size={16} />
					</Link>
				</div>
			</div>
		</section>
	);
}

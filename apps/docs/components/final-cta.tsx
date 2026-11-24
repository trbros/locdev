import { ArrowRight, ArrowUpRight } from "lucide-react";
import { getTranslations } from "next-intl/server";
import { GitHub } from "@/components/icons";
import { Link } from "@/i18n/navigation";
import { githubRepo } from "@/lib/repository";

export async function FinalCTA() {
	const t = await getTranslations("CTA");
	return (
		<section className="cta-section">
			<div className="page-container relative overflow-hidden py-24 md:py-32">
				<div className="cta-grid" aria-hidden="true" />
				<div className="relative z-10 max-w-4xl">
					<p className="section-eyebrow">{t("eyebrow")}</p>
					<h2 className="mt-6 text-[clamp(3rem,6.5vw,6rem)] leading-[.99] font-semibold tracking-[-.075em]">
						{t("title")} <span className="text-ember">{t("accent")}</span>
					</h2>
					<p className="mt-7 max-w-xl text-lg leading-7 text-muted-foreground">
						{t("body")}
					</p>
					<div className="mt-9 flex flex-wrap items-center gap-3">
						<Link className="btn-ember" href="/changelog">
							{t("primary")} <ArrowRight size={16} />
						</Link>
						<a
							className="btn-outline"
							href={githubRepo}
							target="_blank"
							rel="noopener noreferrer"
						>
							<GitHub className="size-4" />
							{t("secondary")}
							<ArrowUpRight size={14} />
						</a>
					</div>
				</div>
			</div>
		</section>
	);
}

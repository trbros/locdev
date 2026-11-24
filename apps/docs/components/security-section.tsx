import { ArrowUpRight, ShieldCheck } from "lucide-react";
import { getTranslations } from "next-intl/server";
import { Link } from "@/i18n/navigation";
import { route } from "@/lib/routes.generated";

export async function SecuritySection() {
	const t = await getTranslations("Security");
	return (
		<section className="security-band">
			<div className="page-container security-inner">
				<div className="security-orb" aria-hidden="true">
					<div className="security-orb-inner">
						<ShieldCheck size={60} strokeWidth={1.2} />
					</div>
					<div className="orbit-mark mark-a">+ </div>
					<div className="orbit-mark mark-b">+</div>
				</div>
				<div className="relative z-10 max-w-2xl">
					<p className="section-eyebrow text-ember">{t("eyebrow")}</p>
					<h2 className="section-heading mt-5 text-[#f4efe6]">{t("title")}</h2>
					<p className="mt-6 max-w-xl text-base leading-7 text-[#a7a49d]">
						{t("body")}
					</p>
					<Link
						href={route("docs")}
						className="mt-8 inline-flex items-center gap-2 border-b border-ember/60 pb-2 text-sm font-semibold text-[#f4efe6] transition-colors hover:text-ember"
					>
						{t("cta")} <ArrowUpRight size={16} />
					</Link>
				</div>
				<div className="security-bottom">
					{[t("item1"), t("item2"), t("item3")].map((x) => (
						<span key={x}>
							<span className="led led-green" />
							{x}
						</span>
					))}
				</div>
			</div>
		</section>
	);
}

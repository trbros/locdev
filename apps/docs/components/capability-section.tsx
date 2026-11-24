import {
	ArrowDownRight,
	ArrowUpRight,
	Braces,
	CircleCheck,
	Cpu,
	Fingerprint,
	Globe2,
	LockKeyhole,
	Radar,
	ShieldCheck,
} from "lucide-react";
import { getTranslations } from "next-intl/server";
import { Link } from "@/i18n/navigation";
import { route } from "@/lib/routes.generated";
import { CapabilityGrid } from "./capability-grid";

export async function CapabilitySection() {
	const t = await getTranslations("Capabilities");
	const common = await getTranslations("Common");
	return (
		<section id="capabilities" className="section-pad border-t border-border">
			<div className="page-container">
				<div className="section-topline">
					<p className="section-eyebrow">{t("eyebrow")}</p>
					<span className="font-mono text-[11px] text-muted-foreground">
						[ 01 — 06 ]
					</span>
				</div>
				<div className="section-intro">
					<h2 className="section-heading">
						{t("title")} <span className="text-ember">{t("titleAccent")}</span>
					</h2>
					<p className="section-description">{t("body")}</p>
				</div>
				<CapabilityGrid>
					<article className="cap-card cap-card-domains">
						<div className="cap-meta">
							<span>{t("domainsTag")}</span>
							<Globe2 size={18} />
						</div>
						<div className="domains-art" aria-hidden="true">
							<span className="domain-node domain-node-root">
								<span className="led led-green" /> acme.test
							</span>
							<svg
								className="domain-connections"
								viewBox="0 0 400 190"
								preserveAspectRatio="none"
								aria-hidden="true"
							>
								{[
									"M200 64V86Q200 96 190 96H94Q80 96 80 110V134",
									"M200 64V86Q200 96 210 96H306Q320 96 320 110V134",
								].map((path, index) => (
									<g key={path}>
										<path
											className="domain-track"
											d={path}
											vectorEffect="non-scaling-stroke"
										/>
										<path
											className={`domain-signal domain-signal-${index}`}
											d={path}
											pathLength={100}
											vectorEffect="non-scaling-stroke"
										/>
									</g>
								))}
							</svg>
							<span className="domain-node domain-node-child">
								api.acme.test
							</span>
							<span className="domain-node domain-node-child child-two">
								admin.acme.test
							</span>
							<div className="domain-ghost">*.test</div>
						</div>
						<div className="cap-bottom">
							<div>
								<h3>{t("domainsTitle")}</h3>
								<p>{t("domainsBody")}</p>
							</div>
							<span className="cap-detail">{t("domainsFooter")}</span>
						</div>
					</article>
					<article className="cap-card cap-card-https">
						<div className="cap-meta">
							<span>{t("httpsTag")}</span>
							<ShieldCheck size={18} />
						</div>
						<div className="security-art" aria-hidden="true">
							<div className="security-ring security-ring-inner" />
							<div className="security-ring security-ring-outer" />
							<div className="security-core">
								<LockKeyhole size={30} strokeWidth={1.4} />
							</div>
							<div className="security-label">
								<span className="led led-green" /> TLS / SECURE
							</div>
						</div>
						<div className="cap-bottom">
							<h3>{t("httpsTitle")}</h3>
							<p>{t("httpsBody")}</p>
						</div>
					</article>
					<article className="cap-card cap-card-process">
						<div className="cap-meta">
							<span>{t("processTag")}</span>
							<Cpu size={18} />
						</div>
						<div className="process-art" aria-hidden="true">
							{[
								19, 48, 31, 69, 53, 37, 66, 44, 83, 58, 92, 73, 45, 80, 65, 88,
								51, 72, 95, 78,
							].map((h, i) => (
								<i key={i} style={{ height: `${h}%` }} />
							))}
						</div>
						<div className="cap-bottom">
							<h3>{t("processTitle")}</h3>
							<p>{t("processBody")}</p>
						</div>
					</article>
					<article className="cap-card cap-card-privileges">
						<div className="cap-meta">
							<span>{t("privilegeTag")}</span>
							<Fingerprint size={18} />
						</div>
						<div className="privilege-art" aria-hidden="true">
							<span className="font-mono text-xs text-muted-foreground">
								$ locdev tls trust
							</span>
							<div className="flex items-center gap-2 text-[12px]">
								<CircleCheck className="text-phosphor" size={16} />
								<span>APPROVAL REQUIRED</span>
							</div>
							<div className="privilege-action">
								<span>ALLOW ONCE</span>
								<ArrowDownRight size={14} />
							</div>
						</div>
						<div className="cap-bottom">
							<h3>{t("privilegeTitle")}</h3>
							<p>{t("privilegeBody")}</p>
						</div>
					</article>
					<article className="cap-card cap-card-runtimes">
						<div className="cap-meta">
							<span>{t("runtimesTag")}</span>
							<Braces size={18} />
						</div>
						<div className="runtimes-art" aria-hidden="true">
							<span>
								node <b>22.14.0</b>
								<i className="led led-green" />
							</span>
							<span>
								php <b>8.4.4</b>
								<i className="led led-green" />
							</span>
							<span>
								node <b>20.19.0</b>
								<i className="led led-green" />
							</span>
						</div>
						<div className="cap-bottom">
							<h3>{t("runtimesTitle")}</h3>
							<p>{t("runtimesBody")}</p>
						</div>
					</article>
					<article className="cap-card cap-card-doctor">
						<div className="cap-meta">
							<span>{t("doctorTag")}</span>
							<Radar size={18} />
						</div>
						<div className="doctor-art" aria-hidden="true">
							<div>
								<span>DNS RESOLVER</span>
								<CircleCheck size={14} />
							</div>
							<div>
								<span>LOCAL CA</span>
								<CircleCheck size={14} />
							</div>
							<div>
								<span>PROXY</span>
								<CircleCheck size={14} />
							</div>
							<div className="doctor-success">3 / 3 CHECKS PASSED</div>
						</div>
						<div className="cap-bottom">
							<h3>{t("doctorTitle")}</h3>
							<p>{t("doctorBody")}</p>
							<Link
								href={route("docs.$slug", { slug: "doctor" })}
								className="cap-docs-link"
							>
								{common("viewDocs")}
								<ArrowUpRight size={14} />
							</Link>
						</div>
					</article>
				</CapabilityGrid>
			</div>
		</section>
	);
}

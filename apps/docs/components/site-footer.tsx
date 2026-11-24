import { ArrowUp, ArrowUpRight } from "lucide-react";
import { getTranslations } from "next-intl/server";
import { Link } from "@/i18n/navigation";
import { githubRepo } from "@/lib/repository";
import { route } from "@/lib/routes.generated";
import { siteInformation } from "@/lib/site-information";
import { AnimatedWordmark } from "./animated-wordmark";
import { Brand } from "./brand";
import { GitHub, Instagram } from "./icons";
import { TrioooLogo } from "./triooo-logo";

export async function SiteFooter() {
	const t = await getTranslations("Footer");
	const nav = await getTranslations("Nav");
	const license = await getTranslations("License");

	return (
		<footer id="site-footer" className="site-footer">
			<div className="page-container">
				<div className="footer-main">
					<div className="footer-brand">
						<Brand />
						<p className="footer-tagline">{t("description")}</p>
						<Link href="/about" className="footer-about-link">
							{nav("about")}
							<ArrowUpRight size={13} aria-hidden="true" />
						</Link>
						<ul className="footer-socials">
							<li>
								<a
									target="_blank"
									rel="noopener noreferrer"
									href={`https://instagram.com/${process.env.NEXT_PUBLIC_TRIOOO_INSTAGRAM}`}
								>
									<Instagram className="size-4 shrink-0" aria-hidden="true" />
									Instagram
								</a>
							</li>
							<li>
								<a
									target="_blank"
									rel="noopener noreferrer"
									href={`https://github.com/${process.env.NEXT_PUBLIC_TRIOOO_GITHUB}`}
								>
									<GitHub className="size-4 shrink-0" aria-hidden="true" />
									GitHub
								</a>
							</li>
						</ul>
					</div>

					<div className="footer-navigation">
						<nav className="footer-nav" aria-labelledby="footer-product">
							<h2 id="footer-product">{t("product")}</h2>
							<ul>
								<li>
									<Link href="/features">{nav("features")}</Link>
								</li>
								<li>
									<Link href="/compare">{nav("compare")}</Link>
								</li>
								<li>
									<Link href="/stacks">{nav("stacks")}</Link>
								</li>
								<li>
									<Link href="/services">{nav("services")}</Link>
								</li>
								<li>
									<Link href="/download">{nav("download")}</Link>
								</li>
							</ul>
						</nav>
						<nav className="footer-nav" aria-labelledby="footer-help">
							<h2 id="footer-help">{t("help")}</h2>
							<ul>
								<li>
									<Link href={route("docs")}>{nav("docs")}</Link>
								</li>
								<li>
									<Link href="/get-started">{nav("getStarted")}</Link>
								</li>
								<li>
									<Link href="/configure">{nav("configure")}</Link>
								</li>
								<li>
									<Link href="/support">{nav("support")}</Link>
								</li>
								<li>
									<Link href="/faq">{nav("faq")}</Link>
								</li>
							</ul>
						</nav>
						<nav className="footer-nav" aria-labelledby="footer-project">
							<h2 id="footer-project">{t("project")}</h2>
							<ul>
								<li>
									<Link href="/about">{nav("about")}</Link>
								</li>
								<li>
									<Link href="/contact">{nav("contact")}</Link>
								</li>
								<li>
									<Link href="/changelog">{nav("changelog")}</Link>
								</li>
								<li>
									<Link href="/contribute">{nav("contribute")}</Link>
								</li>
								<li>
									<Link href="/donate">{nav("donate")}</Link>
								</li>
								<li>
									<a
										href={githubRepo}
										target="_blank"
										rel="noopener noreferrer"
									>
										<GitHub className="size-3.5" />
										{t("external")}
										<ArrowUpRight size={12} />
									</a>
								</li>
							</ul>
						</nav>
						<nav className="footer-nav" aria-labelledby="footer-legal">
							<h2 id="footer-legal">{t("legal")}</h2>
							<ul>
								<li>
									<Link href="/privacy">{nav("privacy")}</Link>
								</li>
								<li>
									<Link href="/license">{license("label")} · Apache-2.0</Link>
								</li>
								{siteInformation.securityContact && (
									<li>
										<Link href="/security">{nav("securityReporting")}</Link>
									</li>
								)}
							</ul>
						</nav>
					</div>
					<div className="footer-maintainer">
						<a
							href={`${process.env.NEXT_PUBLIC_TRIOOO_SITE_URL}?ref=${encodeURIComponent("https://locdev.app")}`}
							target="_blank"
							rel="noopener noreferrer"
						>
							<span className="footer-maintainer-caption">
								{t("maintainedBy")}
							</span>
							<span className="footer-maintainer-logo">
								<TrioooLogo className="w-20" />
								<ArrowUpRight className="arrow" size={14} aria-hidden="true" />
							</span>
							<span className="footer-maintainer-domain">
								{process.env.NEXT_PUBLIC_TRIOOO_SITE_URL?.replace(
									"https://",
									"",
								)}
							</span>
						</a>
					</div>
				</div>

				<div className="footer-bottom">
					<span>© {new Date().getFullYear()} LocDev</span>
					<span className="footer-license">{t("rights")}</span>
					<a href="#main-content" className="footer-back">
						{t("back")}
						<ArrowUp size={14} />
					</a>
				</div>
				<div className="footer-wordmark-stage">
					<AnimatedWordmark className="footer-wordmark" decorative />
				</div>
			</div>
		</footer>
	);
}

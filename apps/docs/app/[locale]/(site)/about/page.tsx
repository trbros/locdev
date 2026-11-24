import {
	ArrowDownToLine,
	ArrowUpRight,
	BookOpen,
	History,
	Scale,
} from "lucide-react";
import type { Metadata } from "next";
import { getTranslations } from "next-intl/server";
import { LogoMark } from "@/components/brand";
import { TrioooLogo } from "@/components/triooo-logo";
import {
	Accordion,
	AccordionContent,
	AccordionItem,
	AccordionTrigger,
} from "@/components/ui/accordion";
import { Link } from "@/i18n/navigation";
import { appVersion } from "@/lib/app-version";
import thirdParty from "@/lib/generated/app-third-party.json";
import { githubReleases, githubRepo } from "@/lib/repository";
import { route } from "@/lib/routes.generated";

export async function generateMetadata(): Promise<Metadata> {
	const t = await getTranslations("About");
	const a = await getTranslations("AboutApp");
	return { title: t("title"), description: a("description") };
}
export default async function AboutPage() {
	const t = await getTranslations("About");
	const a = await getTranslations("AboutApp");
	return (
		<div className="page-container about-app">
			<header className="about-app-heading">
				<h1 className="section-heading">{t("title")}</h1>
				<p className="route-description">{a("subtitle")}</p>
			</header>
			<section className="about-app-intro" aria-label="LocDev">
				<div className="about-app-product">
					<LogoMark className="about-app-logo" />
					<div>
						<h2>LocDev</h2>
						<p className="route-description">{a("description")}</p>
						<p className="about-app-version">
							app <span>{appVersion}</span> <span aria-hidden="true">·</span>{" "}
							daemon <span aria-hidden="true">·</span> CLI
						</p>
					</div>
				</div>
				<div className="route-actions">
					<Link href="/download" className="btn-outline">
						<ArrowDownToLine size={14} />
						{a("updates")}
					</Link>
					<Link href="/changelog" className="btn-outline">
						<History size={14} />
						{a("history")}
					</Link>
					<Link href={route("docs")} className="btn-outline">
						<BookOpen size={14} />
						{a("documentation")}
					</Link>
					<a
						href={githubRepo}
						target="_blank"
						rel="noopener noreferrer"
						className="btn-outline"
					>
						<ArrowUpRight size={14} />
						GitHub
					</a>
				</div>
			</section>
			<section className="about-app-section">
				<div>
					<h2>{a("build")}</h2>
					<p>{a("releaseNote")}</p>
					<dl className="about-app-facts">
						<div>
							<dt>{a("app")}</dt>
							<dd>{appVersion}</dd>
						</div>
						<div>
							<dt>{a("channel")}</dt>
							<dd>
								<a
									href={githubReleases}
									target="_blank"
									rel="noopener noreferrer"
								>
									{a("stable")}
								</a>
							</dd>
						</div>
						<div>
							<dt>{a("license")}</dt>
							<dd>
								<Link href="/license">Apache-2.0</Link>
							</dd>
						</div>
					</dl>
				</div>
				<p className="about-app-installation-note">{a("installationBody")}</p>
			</section>
			<section className="about-app-section">
				<div>
					<h2>{a("credits")}</h2>
					<p>{a("creditsBody")}</p>
					<div className="route-actions">
						<Link href="/license" className="btn-outline">
							Apache License 2.0
							<Scale size={16} />
						</Link>
						<a
							href="https://locdev.app"
							target="_blank"
							rel="noopener noreferrer"
							className="btn-outline"
						>
							locdev.app
							<ArrowUpRight size={16} />
						</a>
						<a
							href={`${githubRepo}/blob/main/CHANGELOG.md`}
							target="_blank"
							rel="noopener noreferrer"
							className="btn-outline"
						>
							CHANGELOG.md
							<ArrowUpRight size={16} />
						</a>
					</div>
				</div>
			</section>
			<Accordion className="about-app-credits-disclosure">
				<AccordionItem value="third-party">
					<AccordionTrigger>{a("thirdParty")}</AccordionTrigger>
					<AccordionContent>
						<p className="about-app-packages-description">
							{a("packagesBody")}
						</p>
						<Accordion
							aria-label={a("thirdParty")}
							className="about-app-packages"
							multiple
						>
							{(["rust", "npm"] as const).map((kind) => (
								<AccordionItem key={kind} value={kind}>
									<AccordionTrigger>
										{a(kind)} <span>{thirdParty[kind].length}</span>
									</AccordionTrigger>
									<AccordionContent>
										<div
											className="about-app-table-scroll"
											role="region"
											aria-label={a(kind)}
											tabIndex={0}
										>
											<table>
												<caption className="sr-only">{a(kind)}</caption>
												<thead>
													<tr>
														<th scope="col">{a("package")}</th>
														<th scope="col">{a("version")}</th>
														<th scope="col">{a("license")}</th>
													</tr>
												</thead>
												<tbody>
													{thirdParty[kind].map((item) => (
														<tr key={`${item.name}-${item.version}`}>
															<th scope="row">
																{item.url && /^https?:\/\//.test(item.url) ? (
																	<a
																		href={item.url}
																		target="_blank"
																		rel="noopener noreferrer"
																	>
																		{item.name}
																	</a>
																) : (
																	item.name
																)}
															</th>
															<td>{item.version}</td>
															<td>{item.license}</td>
														</tr>
													))}
												</tbody>
											</table>
										</div>
									</AccordionContent>
								</AccordionItem>
							))}
						</Accordion>
					</AccordionContent>
				</AccordionItem>
			</Accordion>
			<section className="about-app-section">
				<h2>{a("cli")}</h2>
				<p>{a("cliBody")}</p>
				<p>{a("cliInstall")}</p>
				<div className="route-actions">
					<Link
						href={route("docs.$slug", { slug: "cli" })}
						className="btn-outline"
					>
						<BookOpen size={16} />
						{a("commands")}
					</Link>
					<Link href="/download" className="about-app-text-action">
						<ArrowDownToLine size={14} />
						{a("cliDownload")}
					</Link>
				</div>
			</section>
			<footer className="about-app-maintainer">
				<p>{a("maintained")}</p>
				<a
					href="https://triooo.com.br"
					target="_blank"
					rel="noopener noreferrer"
					aria-label="Triooo"
				>
					<TrioooLogo className="w-24" />
				</a>
				<p>{a("contributors")}</p>
			</footer>
		</div>
	);
}

import {
	ArrowUpRight,
	BookOpen,
	Bug,
	MessagesSquare,
	Send,
} from "lucide-react";
import type { Metadata } from "next";
import { getTranslations } from "next-intl/server";
import { LogoMark } from "@/components/brand";
import { PageReveal } from "@/components/page-reveal";
import { TrioooLogo } from "@/components/triooo-logo";
import { Link } from "@/i18n/navigation";
import { repository } from "@/lib/repository";

export async function generateMetadata(): Promise<Metadata> {
	const t = await getTranslations("Contact");
	return { title: t("title"), description: t("body") };
}

export default async function ContactPage() {
	const t = await getTranslations("Contact");
	const l = await getTranslations("Labels");
	return (
		<PageReveal className="page-container contact-page">
			<header className="contact-hero">
				<div>
					<p className="section-eyebrow">{t("eyebrow")}</p>
					<h1>
						{t("title")}
						<span>{t("accent")}</span>
					</h1>
					<p className="contact-intro">{t("body")}</p>
					<a
						href={repository.issues}
						className="btn-ember"
						target="_blank"
						rel="noopener noreferrer"
					>
						{t("primary")}
						<ArrowUpRight size={16} />
					</a>
				</div>
				<div className="contact-signal" aria-hidden="true">
					<div className="contact-signal-top">
						<span />
						locdev / contact
					</div>
					<div className="contact-signal-route">
						<div className="contact-signal-node">
							<LogoMark className="size-12" />
						</div>
						<div className="contact-signal-wire">
							<i />
							<i />
							<i />
							<Send size={24} />
						</div>
						<div className="contact-signal-node">
							<MessagesSquare size={30} />
						</div>
					</div>
					<div className="contact-signal-copy">
						<span>{l("contactHello")}</span>
						<span>↳ {t("signal")}</span>
					</div>
					<div className="contact-signal-bottom">{t("signature")}</div>
				</div>
			</header>
			<section
				className="contact-channels"
				aria-labelledby="contact-channels-title"
			>
				<div className="contact-section-heading">
					<h2 id="contact-channels-title">{t("channels")}</h2>
					<p>{t("channelsBody")}</p>
				</div>
				<a
					className="contact-channel"
					href={repository.issues}
					target="_blank"
					rel="noopener noreferrer"
				>
					<Bug size={24} aria-hidden="true" />
					<div>
						<h3>{t("githubTitle")}</h3>
						<p>{t("githubBody")}</p>
						<span>
							{t("githubAction")}
							<ArrowUpRight size={14} />
						</span>
					</div>
				</a>
				<Link className="contact-channel" href="/support">
					<BookOpen size={24} aria-hidden="true" />
					<div>
						<h3>{t("supportTitle")}</h3>
						<p>{t("supportBody")}</p>
						<span>
							{t("supportAction")}
							<ArrowUpRight size={14} />
						</span>
					</div>
				</Link>
				<a
					className="contact-channel"
					href="https://triooo.com.br"
					target="_blank"
					rel="noopener noreferrer"
				>
					<MessagesSquare size={24} aria-hidden="true" />
					<div>
						<h3>{t("trioooTitle")}</h3>
						<p>{t("trioooBody")}</p>
						<span>
							{t("trioooAction")}
							<ArrowUpRight size={14} />
						</span>
					</div>
				</a>
			</section>
			<aside className="contact-note">
				<Send size={20} aria-hidden="true" />
				<p>{t("note")}</p>
			</aside>
		</PageReveal>
	);
}

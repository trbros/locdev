import {
	ArrowDown,
	ArrowUpRight,
	Asterisk,
	Check,
	ChevronRight,
	Terminal,
} from "lucide-react";
import { getTranslations } from "next-intl/server";
import { AppShowcase } from "@/components/app-showcase";
import { CapabilitySection } from "@/components/capability-section";
import { ScreenshotGallery } from "@/components/docs/screenshot-gallery";
import { FAQ } from "@/components/faq";
import { FinalCTA } from "@/components/final-cta";
import { PlatformSection } from "@/components/platform-section";
import { SecuritySection } from "@/components/security-section";
import { StackIcon } from "@/components/stacks/stack-icon";
import { StackShowcase } from "@/components/stacks/stack-showcase";
import { TerminalDemo } from "@/components/terminal-demo";
import { Link } from "@/i18n/navigation";
import { appVersion } from "@/lib/app-version";
import { route } from "@/lib/routes.generated";
import { stacks } from "@/lib/stacks";

export default async function Home() {
	const [h, tick, demo, flow, gallery] = await Promise.all([
		getTranslations("Hero"),
		getTranslations("Ticker"),
		getTranslations("Demo"),
		getTranslations("Workflow"),
		getTranslations("Gallery"),
	]);
	return (
		<>
			<section className="hero-section">
				<div className="hero-noise" aria-hidden="true" />
				<div className="hero-halo" aria-hidden="true" />
				<div className="hero-right-outline" aria-hidden="true">
					01 / LOCDEV
				</div>
				<div className="page-container relative z-10 pt-16 sm:pt-24 pb-20 lg:pt-28 lg:pb-28">
					<div className="hero-grid">
						<div className="hero-copy">
							<div className="inline-flex flex-wrap items-center gap-3 text-[10px] sm:text-[11px] font-mono tracking-[.12em]">
								<span className="text-ember inline-flex items-center gap-2">
									<Asterisk size={14} />
									{h("eyebrow")}
								</span>
								<span className="rounded-md border border-border px-2.5 py-1 text-muted-foreground">
									{h("version", { version: appVersion })}
								</span>
							</div>
							<h1 className="hero-title mt-8">
								{h("titlePrimary")}{" "}
								<span className="hero-title-accent">{h("titleAccent")}</span>
							</h1>
							<p className="hero-description mt-7">{h("body")}</p>
							<div className="mt-9 flex flex-wrap gap-3">
								<Link href="/get-started" className="btn-ember">
									{h("primary")} <ArrowUpRight size={17} />
								</Link>
								<Link href={route("docs")} className="btn-outline">
									<Terminal size={16} />
									{h("secondary")} <ChevronRight size={16} />
								</Link>
							</div>
							<div className="hero-micro-footer">
								<span>
									<Check size={13} className="text-phosphor" />
									{h("badge1")}
								</span>
								<span>
									<Check size={13} className="text-phosphor" />
									{h("badge2")}
								</span>
							</div>
						</div>
						<div className="hero-visual">
							<div className="hero-visual-deco" aria-hidden="true">
								<span className="corner-plus plus-one">+</span>
								<span className="corner-plus plus-two">+</span>
								<span className="hero-visual-index">[ 001 / INTERFACE ]</span>
							</div>
							<AppShowcase hero />
							<div className="hero-floating-tag">
								<span className="led led-green" /> localhost → <b>acme.test</b>{" "}
								<ArrowUpRight size={13} />
							</div>
						</div>
					</div>
					<div className="hero-scroll">
						<span className="h-px w-14 bg-ember" />
						{h("scroll")}
						<ArrowDown size={14} />
						<span className="ml-auto hidden md:block">{h("caption")}</span>
					</div>
				</div>
			</section>
			<section
				className="stack-strip border-y border-border"
				aria-label={tick("label")}
			>
				<div className="page-container flex flex-wrap items-center justify-between gap-x-5 gap-y-5 py-6">
					<span className="font-mono text-[10px] tracking-[.15em] text-muted-foreground">
						{tick("label")}
					</span>
					<div className="flex flex-wrap items-center gap-x-7 gap-y-3 text-[13px] font-semibold text-foreground/65 sm:gap-x-10">
						{["laravel", "nextjs", "vue", "rails", "rust", "wordpress"].map(
							(id) => {
								const stack = stacks.find((stack) => stack.id === id);
								if (!stack) return null;
								return (
									<Link href="/stacks" key={id} className="stack-strip-item">
										<StackIcon stack={stack} />
										<span>{stack.name}</span>
									</Link>
								);
							},
						)}
					</div>
				</div>
			</section>
			<CapabilitySection />
			<StackShowcase />
			<section className="section-pad demo-section border-t border-border">
				<div className="page-container">
					<div className="section-topline">
						<p className="section-eyebrow">{demo("eyebrow")}</p>
						<span className="font-mono text-xs text-muted-foreground">
							[ APP / DESKTOP ]
						</span>
					</div>
					<div className="section-intro">
						<h2 className="section-heading max-w-3xl">{demo("title")}</h2>
						<p className="section-description">{demo("body")}</p>
					</div>
					<div className="demo-stage mt-12">
						<div className="demo-stage-meta">
							<span>
								<span className="led led-green" />
								{demo("cliLabel")}
							</span>
							<span>{gallery("sites")}</span>
						</div>
						<div className="demo-stage-window">
							<AppShowcase />
						</div>
						<div className="demo-stage-grid" aria-hidden="true" />
					</div>
				</div>
			</section>
			<section className="section-pad border-t border-border">
				<div className="page-container grid items-center gap-14 lg:grid-cols-[.86fr_1.14fr]">
					<div>
						<p className="section-eyebrow">{flow("eyebrow")}</p>
						<h2 className="section-heading mt-5">
							{flow("title")}{" "}
							<span className="text-ember">{flow("titleAccent")}</span>
						</h2>
						<p className="mt-6 max-w-md text-base leading-7 text-muted-foreground">
							{flow("body")}
						</p>
						<div className="mt-10 space-y-5">
							{[flow("step1"), flow("step2"), flow("step3")].map((x, i) => (
								<div className="flex items-start gap-4" key={i}>
									<span className="inline-flex size-9 shrink-0 items-center justify-center border border-border rounded-lg bg-card font-mono text-xs text-ember">
										0{i + 1}
									</span>
									<span className="pt-2 text-sm font-medium">{x}</span>
								</div>
							))}
						</div>
						<p className="mt-10 max-w-sm font-mono text-[10px] leading-6 text-muted-foreground">
							* {flow("note")}
						</p>
					</div>
					<TerminalDemo />
				</div>
			</section>
			<ScreenshotGallery />
			<SecuritySection />
			<PlatformSection />
			<FAQ />
			<FinalCTA />
		</>
	);
}

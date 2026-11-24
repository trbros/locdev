"use client";
import { ArrowDownToLine, ArrowUpRight, FileCheck2 } from "lucide-react";
import { useLocale, useTranslations } from "next-intl";
import { useEffect, useState } from "react";
import { ClientHighlightedCode } from "@/components/docs/client-highlighted-code";
import { CodeBlock } from "@/components/docs/code-block";
import { Apple, Linux, Windows } from "@/components/icons";
import {
	Accordion,
	AccordionContent,
	AccordionItem,
	AccordionTrigger,
} from "@/components/ui/accordion";
import { Button } from "@/components/ui/button";
import { ChoiceSelect } from "@/components/ui/choice-select";
import { Link } from "@/i18n/navigation";
import { cliInstallCommands } from "@/lib/install-commands";
import {
	assetFormat,
	detectPlatform,
	matchingAssets,
	type ReleaseArchitecture,
	type ReleaseFeed,
	type ReleasePlatform,
	selectRelease,
} from "@/lib/releases";
import { ReleaseStatus } from "./release-status";

/** The systems' own marks (svgl.app: windows.svg, apple.svg, linux.svg). */
const SYSTEMS = [
	{ value: "windows", label: "Windows", Icon: Windows },
	{ value: "macos", label: "macOS", Icon: Apple },
	{ value: "linux", label: "Linux", Icon: Linux },
] as const;
const systemIcon = (platform: ReleasePlatform | null) =>
	SYSTEMS.find((system) => system.value === platform)?.Icon ?? FileCheck2;

export function DownloadPicker({ feed }: { feed: ReleaseFeed }) {
	const t = useTranslations("Releases");
	const locale = useLocale();
	const n = useTranslations("InstallationGuide");
	const p = useTranslations("PageImprovements");
	const [platform, setPlatform] = useState<ReleasePlatform>("linux");
	const [arch, setArch] = useState<ReleaseArchitecture>("x64");
	const [channel, setChannel] = useState<"stable" | "preview">(
		feed.releases.some((r) => !r.prerelease)
			? "stable"
			: feed.releases.length
				? "preview"
				: "stable",
	);
	const [kind, setKind] = useState<"desktop" | "cli">("desktop");
	// Start on the visitor's own system (after hydration: the server
	// renders the same default for everyone).
	useEffect(() => {
		const hint =
			(navigator as Navigator & { userAgentData?: { platform?: string } })
				.userAgentData?.platform ?? navigator.platform;
		const found = detectPlatform(navigator.userAgent, hint);
		if (found.platform) setPlatform(found.platform);
		// Every Mac sold today is Apple silicon, and Safari says "Intel" anyway.
		if (found.platform === "macos") setArch("arm64");
		else if (found.architecture) setArch(found.architecture);
	}, []);
	const release = selectRelease(feed.releases, channel);
	const assets = matchingAssets(release, platform, arch, kind);
	const cli = cliInstallCommands(platform, release?.tag);
	const sums = release?.assets.find((a) => a.kind === "checksum");
	return (
		<div className="download-picker">
			<div className="download-picker-heading">
				<h2>{p("downloadTitle")}</h2>
				<p>{p("downloadBody")}</p>
			</div>
			<div
				className="download-kind-tabs"
				role="group"
				aria-label={t("package")}
			>
				{(["desktop", "cli"] as const).map((value) => (
					<Button
						key={value}
						variant={kind === value ? "secondary" : "ghost"}
						aria-pressed={kind === value}
						onClick={() => setKind(value)}
					>
						{t(value)}
					</Button>
				))}
			</div>
			<p className="download-kind-description">
				{p(kind === "desktop" ? "desktopDescription" : "cliDescription")}
			</p>
			<ReleaseStatus feed={feed} />
			<div className="download-os-tabs" role="group" aria-label={t("system")}>
				{SYSTEMS.map(({ value, label, Icon }) => (
					<button
						key={value}
						type="button"
						aria-pressed={platform === value}
						onClick={() => setPlatform(value)}
					>
						<Icon className="download-os-icon" aria-hidden="true" />
						<span>{label}</span>
					</button>
				))}
			</div>
			<div className="tool-form-grid">
				<label htmlFor="download-choice-2">
					{t("architecture")}
					<ChoiceSelect
						id="download-choice-2"
						label={t("architecture")}
						value={arch}
						onChange={(v) => setArch(v as ReleaseArchitecture)}
						options={[
							{ value: "x64", label: "x64 / Intel / AMD" },
							{ value: "arm64", label: "ARM64 / Apple Silicon" },
						]}
					/>
				</label>
				<label htmlFor="download-choice-3">
					{t("channel")}
					<ChoiceSelect
						id="download-choice-3"
						label={t("channel")}
						value={channel}
						onChange={(v) => setChannel(v as "stable" | "preview")}
						options={[
							{ value: "stable", label: t("stable") },
							{ value: "preview", label: t("preview") },
						]}
					/>
				</label>
			</div>
			{release && (
				<div className="release-selected">
					<span className="release-channel" data-preview={release.prerelease}>
						{t(release.prerelease ? "preview" : "stable")}
					</span>
					<strong>{release.tag}</strong>
					<time dateTime={release.publishedAt}>
						{new Intl.DateTimeFormat(locale, {
							dateStyle: "medium",
							timeZone: "UTC",
						}).format(new Date(release.publishedAt))}
					</time>
					<Link href="/changelog">
						{t("notes")}
						<ArrowUpRight size={14} />
					</Link>
				</div>
			)}
			{assets.length ? (
				<div className="release-asset-grid">
					{assets.map((asset) => (
						<article key={asset.id} className="release-asset">
							<AssetIcon platform={asset.platform} />
							<h3>{assetFormat(asset.name)}</h3>
							<p>
								<span className="release-asset-file">{asset.name}</span>
								<br />
								{new Intl.NumberFormat(locale, {
									maximumFractionDigits: 1,
								}).format(asset.size / 1024 / 1024)}{" "}
								MiB · {arch}
							</p>
							<a className="btn-ember" href={asset.url}>
								{t("download")}
								<ArrowDownToLine size={15} />
							</a>
							{asset.sha256 ? (
								<>
									<span className="tool-caption">SHA-256</span>
									<CodeBlock code={asset.sha256} />
								</>
							) : sums ? (
								<a href={sums.url} className="tool-text-link">
									{t("checksums")}
									<ArrowUpRight size={13} />
								</a>
							) : (
								<p>{t("noChecksum")}</p>
							)}
						</article>
					))}
				</div>
			) : (
				<div className="tool-empty">
					<h3>
						{t(
							feed.status === "unavailable"
								? "unavailableTitle"
								: release
									? "noAssetTitle"
									: "noChannelTitle",
						)}
					</h3>
					<p>{t("noAssetBody")}</p>
					<Link href="/get-started" className="btn-outline">
						{t("guide")}
						<ArrowUpRight size={15} />
					</Link>
				</div>
			)}
			{kind === "cli" && (
				<section
					className="download-cli-install"
					aria-labelledby="download-cli-title"
				>
					<h3 id="download-cli-title">{n("terminalTitle")}</h3>
					<p>{n("terminalBody")}</p>
					{platform === "windows" && arch === "arm64" ? (
						<p>{n("windowsArm")}</p>
					) : channel === "preview" && !release ? (
						<p>{n("previewMissing")}</p>
					) : (
						<>
							<p className="tool-caption">{cli.shell}</p>
							<CodeBlock code={cli.code} language={cli.language}>
								<ClientHighlightedCode
									code={cli.code}
									language={cli.language}
								/>
							</CodeBlock>
							{cli.cmd && (
								<>
									<h4>{n("cmd")}</h4>
									<CodeBlock code={cli.cmd} language="text" />
								</>
							)}
							{!release && <p>{n("requiresRelease")}</p>}
							<p>{n(platform === "windows" ? "windowsPath" : "unixPath")}</p>
						</>
					)}
					<Link href="/get-started" className="tool-text-link">
						{t("guide")}
						<ArrowUpRight size={14} />
					</Link>
				</section>
			)}
			{release && (
				<Accordion className="release-other-assets">
					<AccordionItem value="all-assets">
						<AccordionTrigger>
							{t("allAssets")} ({release.assets.length})
						</AccordionTrigger>
						<AccordionContent>
							<ul>
								{release.assets.map((asset) => (
									<li key={asset.id}>
										<a href={asset.url}>
											{asset.name}
											<ArrowDownToLine size={13} />
										</a>
										{asset.sha256 && <code>{asset.sha256}</code>}
									</li>
								))}
							</ul>
						</AccordionContent>
					</AccordionItem>
				</Accordion>
			)}
		</div>
	);
}

function AssetIcon({ platform }: { platform: ReleasePlatform | null }) {
	const Icon = systemIcon(platform);
	return <Icon className="download-os-icon" aria-hidden="true" />;
}

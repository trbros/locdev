"use client";
import {
	ArrowRight,
	ArrowUpRight,
	BookOpen,
	Check,
	Code2,
	Download,
	Package,
	Terminal,
} from "lucide-react";
import { useTranslations } from "next-intl";
import { useState } from "react";
import { ClientHighlightedCode } from "@/components/docs/client-highlighted-code";
import { CodeBlock } from "@/components/docs/code-block";
import { Button } from "@/components/ui/button";
import { ChoiceSelect } from "@/components/ui/choice-select";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { Link } from "@/i18n/navigation";
import {
	cliInstallCommands,
	executableInstallCommand,
	sourceInstallCommands,
} from "@/lib/install-commands";
import {
	matchingAssets,
	type ReleaseArchitecture,
	type ReleaseFeed,
	type ReleasePlatform,
	selectRelease,
} from "@/lib/releases";
import { route } from "@/lib/routes.generated";
import { getLocdevCommands } from "@/lib/stack-workflow";
import { stacks } from "@/lib/stacks";

function Command({ code, language }: { code: string; language: string }) {
	return (
		<CodeBlock code={code} language={language}>
			<ClientHighlightedCode code={code} language={language} />
		</CodeBlock>
	);
}

export function InstallGuide({ feed }: { feed: ReleaseFeed }) {
	const t = useTranslations("Onboarding");
	const n = useTranslations("InstallationGuide");
	const r = useTranslations("Releases");
	const [system, setSystem] = useState<ReleasePlatform>("linux");
	const [arch, setArch] = useState<ReleaseArchitecture>("x64");
	const [channel, setChannel] = useState<"stable" | "preview">("stable");
	const [mode, setMode] = useState<"desktop" | "cli">("desktop");
	const [stackId, setStackId] = useState("laravel");
	const release = selectRelease(feed.releases, channel);
	const assets = matchingAssets(release, system, arch, "desktop");
	const cli = cliInstallCommands(system, release?.tag);
	const source = sourceInstallCommands(system, mode);
	const stack = stacks.find((s) => s.id === stackId) ?? stacks[0];
	const language = system === "windows" ? "powershell" : "bash";
	// Existing commands use POSIX quoting; PowerShell escapes apostrophes differently.
	const project = getLocdevCommands(stack)
		.join("\n")
		.replaceAll("'\\''", system === "windows" ? "''" : "'\\''");
	return (
		<div className="installation-guide">
			<div className="installation-setup">
				<section
					className="installation-environment"
					aria-labelledby="installation-environment-title"
				>
					<div className="installation-section-title">
						<span>01</span>
						<div>
							<h2 id="installation-environment-title">{n("environment")}</h2>
							<p>{n("environmentBody")}</p>
						</div>
					</div>
					<div
						role="group"
						aria-label={r("system")}
						className="installation-platform-list"
					>
						{(["linux", "macos", "windows"] as const).map((os) => (
							<Button
								key={os}
								variant="outline"
								aria-pressed={system === os}
								data-active={system === os ? "" : undefined}
								onClick={() => setSystem(os)}
							>
								{os === "macos"
									? "macOS"
									: os === "windows"
										? "Windows"
										: "Linux"}
							</Button>
						))}
					</div>
					<div className="installation-selectors">
						<label htmlFor="install-arch">
							{r("architecture")}
							<ChoiceSelect
								id="install-arch"
								label={r("architecture")}
								value={arch}
								onChange={(v) => setArch(v as ReleaseArchitecture)}
								options={[
									{
										value: "x64",
										label:
											system === "macos" ? "Intel · x64" : "Intel / AMD · x64",
									},
									{
										value: "arm64",
										label:
											system === "macos" ? "Apple Silicon · ARM64" : "ARM64",
									},
								]}
							/>
						</label>
						<label htmlFor="install-channel">
							{r("channel")}
							<ChoiceSelect
								id="install-channel"
								label={r("channel")}
								value={channel}
								onChange={(v) => setChannel(v as "stable" | "preview")}
								options={[
									{ value: "stable", label: r("stable") },
									{ value: "preview", label: r("preview") },
								]}
							/>
						</label>
					</div>
					<p className="installation-release-note" role="status">
						{release
							? `${release.tag} · ${r(channel)}`
							: n(feed.status === "unavailable" ? "unavailable" : "noRelease")}
					</p>
				</section>
				<div className="installation-content">
					<section
						className="installation-methods"
						aria-labelledby="installation-methods-title"
					>
						<div className="installation-section-title">
							<span>02</span>
							<div>
								<h2 id="installation-methods-title">{n("method")}</h2>
								<p>{n("methodBody")}</p>
							</div>
						</div>
						<Tabs defaultValue="desktop" className="installation-method-tabs">
							<TabsList
								aria-label={n("method")}
								className="installation-method-list"
							>
								<TabsTrigger value="desktop">
									<Download size={18} />
									{n("app")}
								</TabsTrigger>
								<TabsTrigger value="terminal">
									<Terminal size={18} />
									{n("terminal")}
								</TabsTrigger>
								<TabsTrigger value="git">
									<Code2 size={18} />
									Git
								</TabsTrigger>
							</TabsList>
							<TabsContent
								value="desktop"
								className="installation-method-content"
							>
								<div className="installation-method-heading">
									<Package size={24} />
									<div>
										<h3>{n("appTitle")}</h3>
										<p>{n(`app_${system}`)}</p>
									</div>
								</div>
								<p className="installation-hint">{n("included")}</p>
								{assets.length ? (
									<div className="installation-downloads">
										{assets.map((asset) => {
											const command = executableInstallCommand(asset, system);
											return (
												<article key={asset.id}>
													<a href={asset.url} className="installation-download">
														<span>
															<Download size={18} />
															{asset.name}
														</span>
														<ArrowUpRight size={16} />
													</a>
													{command && (
														<>
															<p>{n("downloadThenRun")}</p>
															<Command code={command} language={language} />
														</>
													)}
												</article>
											);
										})}
									</div>
								) : (
									<div className="installation-empty">
										<p>{t("noPackage")}</p>
										<Link href="/download" className="tool-text-link">
											{n("allDownloads")}
											<ArrowRight size={14} />
										</Link>
									</div>
								)}
								<Link href="/download" className="tool-text-link">
									{r("checksums")}
									<ArrowRight size={14} />
								</Link>
							</TabsContent>
							<TabsContent
								value="terminal"
								className="installation-method-content"
							>
								<div className="installation-method-heading">
									<Terminal size={24} />
									<div>
										<h3>{n("terminalTitle")}</h3>
										<p>{n("terminalBody")}</p>
									</div>
								</div>
								{channel === "preview" && !release ? (
									<p className="installation-empty">{n("previewMissing")}</p>
								) : system === "windows" && arch === "arm64" ? (
									<p className="installation-empty">{n("windowsArm")}</p>
								) : (
									<>
										<p className="installation-hint">
											{cli.shell} ·{" "}
											{n(system === "windows" ? "windowsPath" : "unixPath")}
										</p>
										<Command code={cli.code} language={cli.language} />
										{cli.cmd && (
											<>
												<h4>{n("cmd")}</h4>
												<Command code={cli.cmd} language="text" />
											</>
										)}
										{!release && (
											<p className="installation-hint">
												{n(
													channel === "preview"
														? "previewMissing"
														: "requiresRelease",
												)}
											</p>
										)}
										{!(channel === "preview" && !release) && (
											<>
												<p>{n("verify")}</p>
												<Command
													code={
														system === "windows"
															? "locdev --version\nlocdev start\nlocdev doctor"
															: 'export PATH="$HOME/.local/bin:$PATH"\nlocdev --version\nlocdev start\nlocdev doctor'
													}
													language={language}
												/>
											</>
										)}
									</>
								)}
							</TabsContent>
							<TabsContent value="git" className="installation-method-content">
								<div className="installation-method-heading">
									<Code2 size={24} />
									<div>
										<h3>{t("sourceTitle")}</h3>
										<p>{n("sourceBody")}</p>
									</div>
								</div>
								<label htmlFor="install-build-mode">
									{r("package")}
									<ChoiceSelect
										id="install-build-mode"
										label={r("package")}
										value={mode}
										onChange={(v) => setMode(v as "desktop" | "cli")}
										options={[
											{ value: "desktop", label: r("desktop") },
											{ value: "cli", label: r("cli") },
										]}
									/>
								</label>
								<p className="installation-hint">
									{n(mode === "desktop" ? "desktopPrereqs" : "cliPrereqs")}
								</p>
								<a
									className="tool-text-link"
									href={`https://v2.tauri.app/start/prerequisites/#${system}`}
									target="_blank"
									rel="noopener noreferrer"
								>
									{t("platformPrerequisites")}
									<ArrowUpRight size={14} />
								</a>
								<Command code={source.code} language={source.language} />
								<p>{n(mode === "desktop" ? "desktopOutput" : "cliOutput")}</p>
								<p className="installation-hint">{n("nativeBuild")}</p>
							</TabsContent>
						</Tabs>
					</section>
					<section
						className="installation-first-project"
						aria-labelledby="installation-project-title"
					>
						<div className="installation-section-title">
							<span>03</span>
							<div>
								<h2 id="installation-project-title">{t("firstProject")}</h2>
								<p>{t("pathNote")}</p>
							</div>
						</div>
						<label htmlFor="install-stack">
							{t("stack")}
							<ChoiceSelect
								id="install-stack"
								label={t("stack")}
								value={stackId}
								onChange={setStackId}
								options={stacks.map((s) => ({ value: s.id, label: s.name }))}
							/>
						</label>
						<Command code={project} language={language} />
						{!stack.commands.run && (
							<p className="tool-warning">{t("missingCommand")}</p>
						)}
						<p className="installation-hint">
							<Check size={16} />
							{t("finishNote")}
						</p>
						<div className="route-actions">
							<Link href={`/configure?stack=${stack.id}`} className="btn-ember">
								{t("customize")}
								<ArrowRight size={15} />
							</Link>
							<Link
								href={route("docs.$slug", { slug: "installation" })}
								className="btn-outline"
							>
								<BookOpen size={15} />
								{t("readGuide")}
							</Link>
						</div>
					</section>
				</div>
			</div>
		</div>
	);
}

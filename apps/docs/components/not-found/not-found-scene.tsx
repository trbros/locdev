"use client";

import {
	ArrowRight,
	Check,
	Pause,
	Play,
	RotateCcw,
	Terminal,
	X,
} from "lucide-react";
import { useFormatter, useTranslations } from "next-intl";
import { useEffect, useRef, useState } from "react";
import { LogoMark } from "@/components/brand";
import {
	Django,
	Expressjs,
	FastAPI,
	Laravel,
	Nestjs,
	Nextjs,
	Nuxt,
	React as ReactIcon,
	Svelte,
	Vue,
} from "@/components/icons";
import { Button } from "@/components/ui/button";
import {
	storyDurations as durations,
	storyConnection,
	storyMilestones,
	storyPairs,
	storyTotalMs,
} from "./story-presets";

const routes = [
	{ path: "/api/health", status: 200 },
	{ path: "/api/sites", status: 200 },
	{ path: "/api/legacy", status: 404 },
	{ path: "/api/routes", status: 200 },
	{ path: "/dashboard", status: 404 },
] as const;
const lastStep = durations.length - 1;
const icons = {
	Django,
	Expressjs,
	FastAPI,
	Laravel,
	Nestjs,
	Nextjs,
	Nuxt,
	React: ReactIcon,
	Svelte,
	Vue,
};

/** A fictional HTTP trace. No diagnostics, commands or network requests run here. */
export function NotFoundScene() {
	const t = useTranslations("NotFound");
	const root = useRef<HTMLDivElement>(null);
	const [elapsedMs, setElapsedMs] = useState(0);
	const [pairIndex, setPairIndex] = useState(0);
	const selectedPair = useRef(false);
	const format = useFormatter();
	const pair = storyPairs[pairIndex];
	const FrontendIcon = icons[pair.frontendIcon];
	const ApiIcon = icons[pair.apiIcon];
	const pendingStep = storyMilestones.findIndex((time) => elapsedMs < time);
	const step = pendingStep < 0 ? lastStep : pendingStep;
	const milliseconds = (value: number) =>
		`${format.number(Math.round(value))} ms`;
	const seconds = (value: number) =>
		`${format.number(value / 1000, { minimumFractionDigits: 1, maximumFractionDigits: 1 })} s`;
	useEffect(() => {
		if (selectedPair.current) return;
		selectedPair.current = true;
		const key = "docs-404-stack-pair";
		let previous = -1;
		try {
			const saved = sessionStorage.getItem(key);
			if (saved !== null) previous = Number(saved);
		} catch {
			/* Storage is optional for this illustration. */
		}
		const valid =
			Number.isInteger(previous) &&
			previous >= 0 &&
			previous < storyPairs.length;
		const next = valid
			? (previous + 1 + Math.floor(Math.random() * (storyPairs.length - 1))) %
				storyPairs.length
			: Math.floor(Math.random() * storyPairs.length);
		setPairIndex(next);
		try {
			sessionStorage.setItem(key, String(next));
		} catch {
			/* The scene also works without storage. */
		}
	}, []);
	const [paused, setPaused] = useState(false);
	const [reduced, setReduced] = useState<boolean | null>(null);
	const [visible, setVisible] = useState(false);
	const [inView, setInView] = useState(false);
	useEffect(() => {
		const preference = window.matchMedia("(prefers-reduced-motion: reduce)");
		const syncMotion = () => setReduced(preference.matches);
		const syncVisibility = () =>
			setVisible(document.visibilityState === "visible");
		syncMotion();
		syncVisibility();
		preference.addEventListener("change", syncMotion);
		document.addEventListener("visibilitychange", syncVisibility);
		const observer = new IntersectionObserver(([entry]) =>
			setInView(entry.isIntersecting),
		);
		if (root.current) observer.observe(root.current);
		return () => {
			preference.removeEventListener("change", syncMotion);
			document.removeEventListener("visibilitychange", syncVisibility);
			observer.disconnect();
		};
	}, []);
	const running = !paused && reduced === false && visible && inView;
	const clockRunning = running && step < lastStep;
	useEffect(() => {
		if (!clockRunning) return;
		let previous = performance.now();
		const timer = window.setInterval(() => {
			const now = performance.now();
			const delta = now - previous;
			previous = now;
			setElapsedMs((value) => Math.min(value + delta, storyTotalMs));
		}, 50);
		return () => window.clearInterval(timer);
	}, [clockRunning]);
	const current = reduced ? lastStep : step;
	const phase =
		current < 6 ? "probe" : current < lastStep ? "recover" : "result";
	const shownMs = reduced ? storyTotalMs : elapsedMs;
	const phaseIndex = phase === "probe" ? 0 : phase === "recover" ? 1 : 2;
	const phaseKeys = ["storyProbe", "storyRecover", "traceResult"] as const;
	const title =
		phase === "probe"
			? current < 5
				? "storyLooking"
				: "storyMainMissing"
			: phase === "recover"
				? "storyReconnecting"
				: "routeNotFound";
	return (
		<div
			className="route-story"
			ref={root}
			data-phase={phase}
			data-step={current}
			data-running={running}
			data-elapsed={shownMs}
			data-pair={`${pair.frontend}/${pair.api}`}
		>
			<div className="story-masthead" aria-hidden="true">
				<span>
					<LogoMark className="size-5" /> Acme <i>/ {t("router")}</i>
				</span>
				<span>
					<i className="story-led" /> acme.test{" "}
					<span className="story-host-ip">· 127.0.0.1</span>
				</span>
			</div>
			<ol className="story-progress" aria-hidden="true">
				{phaseKeys.map((key, index) => (
					<li
						key={key}
						data-active={phaseIndex === index}
						data-complete={phaseIndex > index}
					>
						<span className="story-timeline-rail">
							<i
								style={{
									transform: `scaleX(${phaseIndex > index || (index === 2 && phaseIndex === 2) ? 1 : phaseIndex < index ? 0 : index === 0 ? shownMs / storyMilestones[5] : (shownMs - storyMilestones[5]) / (storyTotalMs - storyMilestones[5])})`,
								}}
							/>
						</span>
						<span className="story-progress-number">
							{phaseIndex > index ? <Check size={12} /> : `0${index + 1}`}
						</span>
						<span className="story-timeline-label">
							{t(key)}
							<small>
								{seconds(
									index === 0
										? 0
										: index === 1
											? storyMilestones[5]
											: storyTotalMs,
								)}
							</small>
						</span>
					</li>
				))}
			</ol>
			<div className="story-heading" aria-hidden="true">
				<span>{t("storyEyebrow")}</span>
				<h2>{t(title)}</h2>
				<p>
					{t(
						phase === "probe"
							? "storyProbeHint"
							: phase === "recover"
								? "storyRecoverHint"
								: "storyResultHint",
					)}
				</p>
			</div>
			<div className="story-stage" aria-hidden="true">
				{phase === "probe" && (
					<div className="story-network">
						<div className="story-source">
							<div className="story-framework">
								<FrontendIcon className="size-8" />
								<span>
									{pair.frontend}
									<small>acme.test :{pair.frontendPort}</small>
								</span>
							</div>
							<div className="story-source-request" key={Math.min(current, 4)}>
								<span>GET</span>
								<code>{routes[Math.min(current, 4)].path}</code>
								<i />
							</div>
							<div className="story-api">
								<ApiIcon className="size-5" />
								<span>
									{pair.api.endsWith("API") ? pair.api : `${pair.api} API`}
								</span>
								<code>:{pair.apiPort}</code>
							</div>
							<span className="story-source-caption">
								{t("storyLocalRequest")}
							</span>
						</div>
						<svg
							className="story-connections"
							viewBox="0 0 100 100"
							preserveAspectRatio="none"
							fill="none"
						>
							{routes.map((route, index) => (
								<g key={route.path}>
									<path d={storyConnection(index)} />
									<path
										className="story-packet"
										data-active={Math.min(current, 4) === index}
										data-error={route.status === 404}
										d={storyConnection(index)}
										pathLength="1"
									/>
								</g>
							))}
						</svg>
						<div className="story-routes">
							{routes.map((route, index) => {
								const done = current > index;
								const active = Math.min(current, 4) === index;
								return (
									<div
										className="story-route"
										key={route.path}
										data-done={done}
										data-active={active}
										data-error={route.status === 404}
										data-primary={index === 4}
									>
										<span className="story-route-dot" />
										<div>
											<code>{route.path}</code>
											<small>
												{t(
													index === 4
														? "storyPrimary"
														: route.status === 200
															? "storyEndpoint"
															: "storyOptional",
												)}
											</small>
										</div>
										<div className="story-route-metrics">
											<time>
												{done
													? milliseconds(durations[index])
													: active
														? milliseconds(
																Math.max(
																	0,
																	shownMs -
																		(index ? storyMilestones[index - 1] : 0),
																),
															)
														: "—"}
											</time>
											<span className="story-route-status">
												{done ? (
													route.status
												) : active ? (
													<i className="story-spinner" />
												) : (
													"···"
												)}
											</span>
										</div>
									</div>
								);
							})}
						</div>
					</div>
				)}
				{phase === "recover" && (
					<div className="story-terminal">
						<div className="story-terminal-bar">
							<span className="story-window-lights">
								<i />
								<i />
								<i />
							</span>
							<span>
								<Terminal size={14} />
								{t("storyRecoveryTerminal")}
							</span>
							<code>acme.test /dashboard</code>
						</div>
						<div className="story-terminal-content">
							<div className="story-command">
								<span>❯</span>
								<code>curl -i https://acme.test/dashboard</code>
								<i />
							</div>
							<p className="story-terminal-note">{t("storyRetryStart")}</p>
							{[1, 2, 3].map((attempt, index) => {
								const finished = current >= 7 + index;
								const attemptMs = Math.min(
									Math.max(shownMs - storyMilestones[5 + index], 0),
									durations[6 + index],
								);
								return (
									<div
										className="story-attempt"
										key={attempt}
										data-visible={current >= 6 + index}
										data-finished={finished}
									>
										<span>0{attempt}</span>
										<div>
											<strong>
												{t("storyAttempt", { attempt })}
												<time>{milliseconds(attemptMs)}</time>
											</strong>
											<code>
												GET /dashboard →{" "}
												{finished ? "HTTP/1.1 404" : t("storyConnecting")}
											</code>
										</div>
										<span className="story-attempt-fail">
											{finished ? (
												<>
													<X size={12} />
													{t("storyFailed")}
												</>
											) : (
												<i className="story-spinner" />
											)}
										</span>
									</div>
								);
							})}
							<div
								className="story-terminal-conclusion"
								data-visible={current >= 10}
							>
								<span>↳</span>
								<div>
									<strong>{t("storyRetryExhausted")}</strong>
									<span className="story-trace-facts">
										DNS ✓ &nbsp; PROXY ✓ &nbsp; HTTP 404
									</span>
								</div>
							</div>
						</div>
					</div>
				)}
				{phase === "result" && (
					<div className="story-result">
						<span className="story-result-label">{t("traceResult")}</span>
						<div className="story-result-code">
							4
							<span>
								<LogoMark className="size-12" />
							</span>
							4
						</div>
						<code className="story-result-path">
							acme.test/dashboard <ArrowRight size={13} /> <span>404</span>
						</code>
						<div className="story-result-timing">
							{t("storyElapsed", { time: seconds(shownMs) })}
						</div>
						<p>{t("storyFinalMessage")}</p>
						<div className="story-result-summary">
							<span>
								<Check size={13} />
								{t("storyFound", { count: 3 })}
							</span>
							<span>
								<X size={13} />
								{t("storyAttemptsFailed", { count: 3 })}
							</span>
						</div>
					</div>
				)}
			</div>
			<div className="story-status" aria-hidden="true">
				<span className="story-led" />
				<span>
					{t(
						phase === "probe"
							? "storyScanning"
							: phase === "recover"
								? "storyRetrying"
								: "storyFinished",
					)}
				</span>
				<code>
					{phase === "probe"
						? `${Math.min(current + 1, 5)} / 5`
						: phase === "recover"
							? `${Math.max(0, Math.min(current - 5, 3))} / 3`
							: "HTTP 404"}{" "}
					<span className="story-status-time">{seconds(shownMs)}</span>
				</code>
			</div>
			<div className="story-controls">
				<span>{t("simulation")}</span>
				<div>
					<Button
						variant="ghost"
						size="sm"
						disabled={reduced === true}
						onClick={() => {
							setElapsedMs(0);
							setPaused(false);
						}}
					>
						<RotateCcw size={12} />
						{t("replay")}
					</Button>
					<Button
						variant="ghost"
						size="sm"
						disabled={reduced === true || current === lastStep}
						aria-pressed={paused || reduced === true}
						onClick={() => setPaused((value) => !value)}
					>
						{paused || reduced ? <Play size={12} /> : <Pause size={12} />}{" "}
						{t(reduced ? "motionOff" : paused ? "resumeMotion" : "pauseMotion")}
					</Button>
				</div>
			</div>
		</div>
	);
}

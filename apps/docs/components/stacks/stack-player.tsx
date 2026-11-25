"use client";

import {
	ArrowLeft,
	ArrowRight,
	ArrowUpRight,
	Box,
	Check,
	Download,
	Pause,
	Play,
	Rocket,
	Terminal,
	TestTube2,
} from "lucide-react";
import { useTranslations } from "next-intl";
import { useEffect, useRef, useState } from "react";
import { Button } from "@/components/ui/button";
import { type Stack, stages } from "@/lib/stacks";
import { StackIcon } from "./stack-icon";

const stageIcons = {
	install: Download,
	run: Terminal,
	build: Box,
	test: TestTube2,
	deploy: Rocket,
};

/** A workflow illustration; commands are displayed as text and never executed. */
export function StackPlayer({
	stack,
	playing,
	onToggle,
	onNext,
	onPrevious,
}: {
	stack: Stack;
	playing: boolean;
	onToggle: () => void;
	onNext: () => void;
	onPrevious: () => void;
}) {
	const t = useTranslations("Stacks");
	const l = useTranslations("Labels");
	const root = useRef<HTMLDivElement>(null);
	const [progress, setProgress] = useState({ id: stack.id, step: 0 });
	const phase = progress.id === stack.id ? progress.step : 0;
	const [visible, setVisible] = useState(false);
	const [reduced, setReduced] = useState(false);
	const [engaged, setEngaged] = useState(false);
	const [tabVisible, setTabVisible] = useState(true);
	useEffect(() => {
		const media = window.matchMedia("(prefers-reduced-motion: reduce)");
		const motion = () => setReduced(media.matches);
		const visibility = () => setTabVisible(!document.hidden);
		motion();
		visibility();
		media.addEventListener("change", motion);
		document.addEventListener("visibilitychange", visibility);
		const observer = new IntersectionObserver(
			([entry]) => setVisible(entry.isIntersecting),
			{ threshold: 0.15 },
		);
		if (root.current) observer.observe(root.current);
		return () => {
			observer.disconnect();
			media.removeEventListener("change", motion);
			document.removeEventListener("visibilitychange", visibility);
		};
	}, []);
	const running = playing && visible && !reduced && !engaged && tabVisible;
	useEffect(() => {
		if (!running) return;
		const timeout = window.setTimeout(
			() => {
				if (phase >= 5) onNext();
				else setProgress({ id: stack.id, step: phase + 1 });
			},
			phase === 5 ? 2100 : 1200,
		);
		return () => window.clearTimeout(timeout);
	}, [running, phase, onNext, stack.id]);
	const revealAll = !playing || reduced;
	const selectedPhase = Math.min(phase, 4);
	return (
		<div
			className="stack-player"
			ref={root}
			data-running={running}
			data-stack={stack.id}
			onPointerEnter={() => setEngaged(true)}
			onPointerLeave={() => setEngaged(false)}
			onFocusCapture={() => setEngaged(true)}
			onBlurCapture={(event) => {
				if (!event.currentTarget.contains(event.relatedTarget))
					setEngaged(false);
			}}
		>
			<div className="stack-player-toolbar">
				<span>
					<span className="led led-green" /> {t("preview")}
				</span>
				<div className="flex items-center gap-1">
					<Button
						variant="ghost"
						size="icon"
						aria-label={t("previous")}
						onClick={onPrevious}
					>
						<ArrowLeft size={15} />
					</Button>
					<Button
						variant="ghost"
						size="icon"
						aria-label={playing ? t("pause") : t("play")}
						aria-pressed={!playing}
						onClick={onToggle}
					>
						{playing ? <Pause size={15} /> : <Play size={15} />}
					</Button>
					<Button
						variant="ghost"
						size="icon"
						aria-label={t("next")}
						onClick={onNext}
					>
						<ArrowRight size={15} />
					</Button>
				</div>
			</div>
			<div className="stack-player-stage">
				<div className="stack-player-identity" key={stack.id}>
					<div className="stack-player-orbit" aria-hidden="true" />
					<div className="stack-player-logo">
						<StackIcon stack={stack} className="size-24" />
					</div>
					<h3>{stack.name}</h3>
					<span className="stack-support" data-support={stack.support}>
						{t(`support.${stack.support}`)}
					</span>
					<span className="stack-player-domain">
						{stack.id}.test<span className="text-ember"> ↗</span>
					</span>
				</div>
				<div className="stack-player-terminal">
					<div className="stack-terminal-title">
						<Terminal size={14} />
						<span>{l("stackWorkflow", { id: stack.id })}</span>
						<span className="ml-auto">{t("example")}</span>
					</div>
					<ol className="stack-command-list">
						{stages.map((stage, index) => {
							const Icon = stageIcons[stage];
							const command = stack.commands[stage];
							const revealed = revealAll || index <= phase;
							const [executable, ...args] = (command ?? "").split(" ");
							return (
								<li
									key={stage}
									data-revealed={revealed}
									data-current={index === selectedPhase}
								>
									<span className="stack-command-stage">
										<Icon size={14} />
										{t(`stages.${stage}`)}
										{index < phase && !revealAll && (
											<Check size={12} className="ml-auto text-phosphor" />
										)}
									</span>
									<pre>
										<code>
											{command ? (
												<>
													<span
														className="stack-command-prompt"
														aria-hidden="true"
													>
														${" "}
													</span>
													<span className="stack-command-executable">
														{executable}
													</span>
													{args.length > 0 && ` ${args.join(" ")}`}
												</>
											) : (
												<span className="stack-command-comment">
													#{" "}
													{t(
														stage === "deploy"
															? "providerCommand"
															: "projectCommand",
													)}
												</span>
											)}
										</code>
									</pre>
								</li>
							);
						})}
					</ol>
					<a
						href={stack.docs}
						target="_blank"
						rel="noopener noreferrer"
						className="stack-command-docs"
					>
						{t("officialDocs")} <ArrowUpRight size={13} />
					</a>
				</div>
			</div>
			<p className="stack-player-note">
				{t("commandNote", { port: "{port}" })}
			</p>
		</div>
	);
}

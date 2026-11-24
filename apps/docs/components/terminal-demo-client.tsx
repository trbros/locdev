"use client";

import { Check, ClipboardCopy, Terminal } from "lucide-react";
import { useTranslations } from "next-intl";
import type { ReactNode } from "react";
import { useState } from "react";
import { Button } from "@/components/ui/button";

const examples = [
	[
		{ kind: "cmd", text: "locdev start" },
		{ kind: "info", text: "Starting local daemon..." },
		{ kind: "ok", text: "✓ daemon started  ·  127.0.0.1" },
	],
	[
		{ kind: "cmd", text: "locdev add ~/Code/acme --start" },
		{ kind: "info", text: "Detected project: Laravel" },
		{ kind: "ok", text: "✓ registered acme" },
		{ kind: "ok", text: "● https://acme.test   running" },
	],
	[
		{ kind: "cmd", text: "locdev doctor" },
		{ kind: "ok", text: "✓ Proxy            listening on :80" },
		{ kind: "ok", text: "✓ DNS resolver     *.test routed" },
		{ kind: "ok", text: "✓ TLS trust        certificate trusted" },
		{ kind: "info", text: "All checks completed." },
	],
] as const;
const commands = [
	"locdev start",
	"locdev add ~/Code/acme --start",
	"locdev doctor",
];

export function TerminalDemoClient({
	highlightedCommands,
}: {
	highlightedCommands: ReactNode[];
}) {
	const t = useTranslations("Workflow");
	const [tab, setTab] = useState(1);
	const [copied, setCopied] = useState(false);
	async function copy() {
		try {
			await navigator.clipboard.writeText(commands[tab]);
			setCopied(true);
			window.setTimeout(() => setCopied(false), 1800);
		} catch {
			setCopied(false);
		}
	}
	return (
		<div className="terminal-window">
			<div className="terminal-head">
				<div className="inline-flex items-center gap-2">
					<Terminal size={14} className="text-ember" />
					<span>LOCDEV / TERMINAL</span>
				</div>
				<span className="text-muted-foreground/60">⌘_</span>
			</div>
			<div className="flex items-center justify-between gap-2 border-b border-border px-4 py-3">
				<div
					role="tablist"
					aria-label="CLI examples"
					className="flex min-w-0 gap-1 overflow-x-auto"
				>
					{[t("tabOne"), t("tabTwo"), t("tabThree")].map((label, index) => (
						<Button
							variant="ghost"
							key={index}
							role="tab"
							type="button"
							aria-selected={tab === index}
							onClick={() => {
								setTab(index);
								setCopied(false);
							}}
							className={`terminal-tab ${tab === index ? "terminal-tab-active" : ""}`}
						>
							{String(index + 1).padStart(2, "0")} / {label}
						</Button>
					))}
				</div>
				<Button
					variant="ghost"
					size="sm"
					type="button"
					className="terminal-copy"
					onClick={copy}
					aria-label={copied ? t("copied") : t("copy")}
				>
					{copied ? <Check size={16} /> : <ClipboardCopy size={16} />}
					<span className="hidden sm:inline">
						{copied ? t("copied") : t("copy")}
					</span>
				</Button>
			</div>
			<div className="terminal-body" role="tabpanel" aria-live="polite">
				<div className="text-muted-foreground/60 mb-5">
					# {tab === 0 ? t("step1") : tab === 1 ? t("step2") : t("step3")}
				</div>
				{examples[tab].map((line, i) => (
					<div
						key={`${tab}-${i}`}
						className={`mb-2 ${line.kind === "ok" ? "text-phosphor" : line.kind === "cmd" ? "text-foreground" : "text-muted-foreground"}`}
					>
						{line.kind === "cmd" && <span className="text-ember mr-2">$</span>}
						{line.kind === "cmd" ? highlightedCommands[tab] : line.text}
					</div>
				))}
				<div className="text-ember mt-5 animate-caret">_</div>
			</div>
			<div className="flex flex-wrap items-center justify-between gap-2 border-t border-border px-5 py-3 text-[10px] text-muted-foreground">
				<span>bash — locdev@localhost</span>
				<span className="text-phosphor flex items-center gap-2">
					<i className="led led-green" /> EXIT 0
				</span>
			</div>
		</div>
	);
}

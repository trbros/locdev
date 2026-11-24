"use client";

import { Check, ClipboardCopy, Code2, Terminal } from "lucide-react";
import { useTranslations } from "next-intl";
import type { ReactNode } from "react";
import { useEffect, useState } from "react";
import { Button } from "@/components/ui/button";

export function CodeBlock({
	code,
	language = "text",
	children,
}: {
	code: string;
	language?: string;
	children?: ReactNode;
}) {
	const t = useTranslations("DocCode");
	const [copyState, setCopyState] = useState<"idle" | "copied" | "failed">(
		"idle",
	);
	const daemon = language === "daemon";
	const shell = ["bash", "sh", "shell", "console", "powershell"].includes(
		language,
	);
	useEffect(() => {
		if (copyState === "idle") return;
		const timeout = window.setTimeout(() => setCopyState("idle"), 2500);
		return () => window.clearTimeout(timeout);
	}, [copyState]);
	async function copy() {
		try {
			await navigator.clipboard.writeText(code.replace(/\n$/, ""));
			setCopyState("copied");
		} catch {
			setCopyState("failed");
		}
	}
	return (
		<div className={`doc-code-block ${daemon ? "doc-daemon-terminal" : ""}`}>
			<div className="doc-code-header">
				<span>
					{shell || daemon ? <Terminal size={14} /> : <Code2 size={14} />}
					{daemon ? "locdevd" : shell ? t("terminal") : language.toUpperCase()}
				</span>
				{daemon ? (
					<span className="doc-daemon-state">
						<i className="led led-green" />
						{t("running")}
					</span>
				) : (
					<Button
						variant="ghost"
						size="sm"
						onClick={copy}
						aria-label={t("copy")}
					>
						{copyState === "copied" ? (
							<Check size={14} />
						) : (
							<ClipboardCopy size={14} />
						)}
						{copyState === "copied" ? t("copied") : t("copy")}
					</Button>
				)}
			</div>
			{daemon && (
				<div className="doc-daemon-command">
					<span aria-hidden="true">$ </span>
					<code>locdev status</code>
				</div>
			)}
			{children ?? (
				<pre>
					<code className={`language-${daemon ? "text" : language}`}>
						{code}
					</code>
				</pre>
			)}
			{daemon && <div className="doc-daemon-caption">{t("example")}</div>}
			<span className="doc-copy-status" role="status">
				{copyState === "failed"
					? t("copyFailed")
					: copyState === "copied"
						? t("copied")
						: ""}
			</span>
		</div>
	);
}

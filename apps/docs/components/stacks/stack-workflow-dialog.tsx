"use client";

import { ArrowUpRight } from "lucide-react";
import { useLocale, useTranslations } from "next-intl";
import { useState } from "react";
import { CodeBlock } from "@/components/docs/code-block";
import { Button } from "@/components/ui/button";
import {
	DialogClose,
	DialogContent,
	DialogDescription,
	DialogTitle,
} from "@/components/ui/dialog";
import { type AppLocale, localizedPathname } from "@/i18n/locales";
import { Link } from "@/i18n/navigation";
import { stackLocationHref } from "@/lib/stack-links";
import { frameworkCommands, getLocdevCommands } from "@/lib/stack-workflow";
import { type Stack, stages } from "@/lib/stacks";
import { StackIcon } from "./stack-icon";

function CommandBlock({ command }: { command: string }) {
	return (
		<CodeBlock code={command} language="bash">
			<pre>
				<code>
					{command.split("\n").map((line, index) => {
						const [executable, ...args] = line.split(" ");
						return (
							<span
								className="stack-workflow-code-line"
								key={`${index}-${line}`}
							>
								<span className="stack-command-executable">{executable}</span>
								{args.length > 0 && ` ${args.join(" ")}`}
								{index < command.split("\n").length - 1 && "\n"}
							</span>
						);
					})}
				</code>
			</pre>
		</CodeBlock>
	);
}

export function StackWorkflowDialog({ stack }: { stack: Stack }) {
	const t = useTranslations("Stacks");
	const locale = useLocale();
	const servicesT = useTranslations("Services");
	const [shareStatus, setShareStatus] = useState<{
		id: string;
		status: "copied" | "failed";
	} | null>(null);
	async function share() {
		try {
			const params = new URLSearchParams(window.location.search);
			const path = stackLocationHref(
				localizedPathname("/stacks", locale as AppLocale),
				params,
				{ stack: stack.id, workflow: "1" },
			);
			await navigator.clipboard.writeText(
				new URL(path, window.location.origin).href,
			);
			setShareStatus({ id: stack.id, status: "copied" });
		} catch {
			setShareStatus({ id: stack.id, status: "failed" });
		}
	}
	return (
		<DialogContent
			className="stack-workflow-dialog"
			closeLabel={t("closeWorkflow")}
		>
			<div className="stack-workflow-heading">
				<StackIcon stack={stack} className="size-12" />
				<div>
					<span className="stack-support" data-support={stack.support}>
						{t(`support.${stack.support}`)}
					</span>
					<DialogTitle>{t("workflowTitle", { name: stack.name })}</DialogTitle>
				</div>
			</div>
			<DialogDescription>{t("workflowDescription")}</DialogDescription>
			<div className="workflow-share-actions">
				<Button variant="outline" onClick={share}>
					{t("share")}
				</Button>
				<Link href={`/configure?stack=${stack.id}`} className="btn-outline">
					{t("configure")}
				</Link>
				<span role="status">
					{shareStatus?.id === stack.id && shareStatus.status === "copied"
						? t("linkCopied")
						: shareStatus?.id === stack.id && shareStatus.status === "failed"
							? t("linkFailed")
							: ""}
				</span>
			</div>
			<section className="stack-workflow-section">
				<h3>{t("nativeCommands")}</h3>
				<div className="stack-workflow-native-grid">
					{stages.map((stage) => (
						<div key={stage}>
							<h4>{t(`stages.${stage}`)}</h4>
							{stack.commands[stage] ? (
								<CommandBlock command={stack.commands[stage]} />
							) : (
								<p className="stack-workflow-fallback">
									{t(stage === "deploy" ? "providerCommand" : "projectCommand")}
								</p>
							)}
						</div>
					))}
				</div>
			</section>
			<section className="stack-workflow-section">
				<h3>{t("specificCommand")}</h3>
				<CommandBlock command={frameworkCommands[stack.id]} />
				<p>{t("toolsNote")}</p>
			</section>
			<section className="stack-workflow-section">
				<h3>{t("locdevCommands")}</h3>
				<p>{t("locdevDescription")}</p>
				<CommandBlock command={getLocdevCommands(stack).join("\n")} />
				{!stack.commands.run && (
					<p className="stack-workflow-warning">{t("missingServer")}</p>
				)}
				<p>{t("portNote", { port: "{port}" })}</p>
			</section>
			<Link href="/services" className="btn-outline">
				{servicesT("homeLink")}
			</Link>
			<a
				className="stack-workflow-docs"
				href={stack.docs}
				target="_blank"
				rel="noopener noreferrer"
			>
				{t("officialDocs")}
				<ArrowUpRight size={14} />
			</a>
			<DialogClose
				render={<Button variant="outline" className="justify-self-end" />}
			>
				{t("closeWorkflow")}
			</DialogClose>
		</DialogContent>
	);
}

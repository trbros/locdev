"use client";
import { Download } from "lucide-react";
import { useTranslations } from "next-intl";
import { useState } from "react";
import { ClientHighlightedCode } from "@/components/docs/client-highlighted-code";
import { CodeBlock } from "@/components/docs/code-block";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import { ChoiceSelect } from "@/components/ui/choice-select";
import { Input } from "@/components/ui/input";
import { buildProjectRecipe, type RecipeInput } from "@/lib/project-recipe";
import { stacks } from "@/lib/stacks";

export function ProjectGenerator({
	initialStack = "laravel",
}: {
	initialStack?: string;
}) {
	const t = useTranslations("Generator");
	const n = useTranslations("PageImprovements");
	const first = stacks.find((s) => s.id === initialStack) ?? stacks[0];
	const [stackId, setStackId] = useState(first.id as string);
	const [name, setName] = useState("my-app");
	const [path, setPath] = useState("/home/user/Code/my-app");
	const [command, setCommand] = useState(first.commands.run ?? "");
	const [port, setPort] = useState("8100");
	const [autoStart, setAutoStart] = useState(false);
	const [shell, setShell] = useState<RecipeInput["shell"]>("posix");
	const [tab, setTab] = useState<"cli" | "toml">("cli");
	const stack = stacks.find((s) => s.id === stackId) ?? first;
	const result = buildProjectRecipe({
		stack,
		name,
		path,
		command,
		port,
		autoStart,
		shell,
	});
	function download() {
		const blob = new Blob([`${result.toml}\n`], {
			type: "application/toml;charset=utf-8",
		});
		const url = URL.createObjectURL(blob);
		const a = document.createElement("a");
		a.href = url;
		a.download = "projects.toml";
		a.click();
		setTimeout(() => URL.revokeObjectURL(url), 1000);
	}
	return (
		<div className="project-generator">
			<form
				className="tool-form-panel"
				onSubmit={(event) => event.preventDefault()}
			>
				<h2>{t("project")}</h2>
				<label htmlFor="generator-choice-1">
					{t("stack")}
					<ChoiceSelect
						id="generator-choice-1"
						label={t("stack")}
						value={stackId}
						onChange={(value) => {
							setStackId(value);
							setCommand(
								stacks.find((s) => s.id === value)?.commands.run ?? "",
							);
						}}
						options={stacks.map((s) => ({ value: s.id, label: s.name }))}
					/>
				</label>
				<label htmlFor="project-name">
					{t("name")}
					<Input
						id="project-name"
						value={name}
						onChange={(e) => setName(e.target.value)}
						aria-invalid={result.errors.includes("nameError")}
						aria-describedby={
							result.errors.includes("nameError") ? "recipe-errors" : undefined
						}
					/>
				</label>
				<label htmlFor="project-path">
					{t("path")}
					<Input
						id="project-path"
						value={path}
						onChange={(e) => setPath(e.target.value)}
						aria-invalid={result.errors.includes("pathError")}
						aria-describedby={
							result.errors.includes("pathError") ? "recipe-errors" : undefined
						}
					/>
				</label>
				<label htmlFor="project-command">
					{t("command")}
					<Input
						id="project-command"
						value={command}
						onChange={(e) => setCommand(e.target.value)}
						aria-invalid={result.errors.includes("commandError")}
						aria-describedby={
							result.errors.includes("commandError")
								? "recipe-errors"
								: undefined
						}
					/>
				</label>
				<label htmlFor="project-port">
					{t("port")}
					<Input
						id="project-port"
						inputMode="numeric"
						value={port}
						onChange={(e) => setPort(e.target.value)}
						aria-invalid={result.errors.includes("portError")}
						aria-describedby={
							result.errors.includes("portError") ? "recipe-errors" : undefined
						}
					/>
				</label>
				<label htmlFor="generator-choice-2">
					{t("shell")}
					<ChoiceSelect
						id="generator-choice-2"
						label={t("shell")}
						value={shell}
						onChange={(value) => {
							setShell(value as RecipeInput["shell"]);
							if (path === "/home/user/Code/my-app" && value === "powershell")
								setPath("C:\\Code\\my-app");
							if (path === "C:\\Code\\my-app" && value === "posix")
								setPath("/home/user/Code/my-app");
						}}
						options={[
							{ value: "posix", label: "Bash / Zsh / POSIX" },
							{ value: "powershell", label: "PowerShell" },
						]}
					/>
				</label>
				<label htmlFor="project-auto-start" className="tool-checkbox">
					<Checkbox
						id="project-auto-start"
						checked={autoStart}
						onCheckedChange={setAutoStart}
					/>
					{t("autoStart")}
				</label>
				<p className="tool-caption">{t("portHelp", { port: "{port}" })}</p>
			</form>
			<section
				className="tool-preview-panel"
				aria-labelledby="recipe-preview-title"
			>
				<p
					className="recipe-readiness"
					role="status"
					data-ready={!result.errors.length}
				>
					{n(result.errors.length ? "outputReview" : "outputReady")}
				</p>
				<div className="tool-preview-heading">
					<h2 id="recipe-preview-title">{t("preview")}</h2>
					<div className="flex gap-2" role="group" aria-label={t("preview")}>
						<Button
							variant={tab === "cli" ? "secondary" : "ghost"}
							aria-pressed={tab === "cli"}
							onClick={() => setTab("cli")}
						>
							CLI
						</Button>
						<Button
							variant={tab === "toml" ? "secondary" : "ghost"}
							aria-pressed={tab === "toml"}
							onClick={() => setTab("toml")}
						>
							TOML
						</Button>
					</div>
				</div>
				{result.errors.length ? (
					<div className="tool-empty" id="recipe-errors" role="status">
						<h3>{t("fixFields")}</h3>
						<ul>
							{result.errors.map((error) => (
								<li key={error}>{t(error)}</li>
							))}
						</ul>
					</div>
				) : (
					<>
						<CodeBlock
							code={tab === "cli" ? result.cli : result.toml}
							language={
								tab === "cli"
									? shell === "powershell"
										? "powershell"
										: "bash"
									: "toml"
							}
						>
							<ClientHighlightedCode
								code={tab === "cli" ? result.cli : result.toml}
								language={
									tab === "cli"
										? shell === "powershell"
											? "powershell"
											: "bash"
										: "toml"
								}
							/>
						</CodeBlock>
						<div className="route-actions">
							<Button variant="outline" onClick={download}>
								<Download size={15} />
								{t("save")}
							</Button>
						</div>
					</>
				)}
				<p className="tool-warning">{t("registryNote")}</p>
				<p>{t("privacy")}</p>
			</section>
		</div>
	);
}

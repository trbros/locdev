/** biome-ignore-all lint/a11y/noNoninteractiveTabindex: Keyboard focus is required for the scrollable license document. */
"use client";

import {
	ArrowDownToLine,
	Check,
	ClipboardCopy,
	FileText,
	Maximize2,
	Type,
	X,
} from "lucide-react";
import { useTranslations } from "next-intl";
import { useEffect, useMemo, useRef, useState } from "react";
import { Button } from "@/components/ui/button";
import {
	Dialog,
	DialogClose,
	DialogContent,
	DialogDescription,
	DialogTitle,
	DialogTrigger,
} from "@/components/ui/dialog";
import { parseLicenseDocument } from "@/lib/license-document";

function LicenseDocument({ text }: { text: string }) {
	const t = useTranslations("LicenseReader");
	const document = useMemo(() => parseLicenseDocument(text), [text]);
	const viewport = useRef<HTMLElement>(null);
	const [active, setActive] = useState("introduction");
	const entries = [
		{ id: "introduction", title: t("introduction") },
		...document.sections,
	];
	function navigate(id: string) {
		const root = viewport.current;
		const target = root?.querySelector<HTMLElement>(
			`[data-license-section="${id}"]`,
		);
		if (!root || !target) return;
		const reduced = window.matchMedia(
			"(prefers-reduced-motion: reduce)",
		).matches;
		root.scrollTo({
			top:
				target.getBoundingClientRect().top -
				root.getBoundingClientRect().top +
				root.scrollTop -
				28,
			behavior: reduced ? "instant" : "smooth",
		});
		setActive(id);
	}
	function trackSection() {
		const root = viewport.current;
		if (!root) return;
		let current = "introduction";
		for (const section of root.querySelectorAll<HTMLElement>(
			"[data-license-section]",
		)) {
			if (
				section.getBoundingClientRect().top -
					root.getBoundingClientRect().top <=
				90
			)
				current = section.dataset.licenseSection ?? current;
		}
		setActive(current);
	}
	return (
		<div className="license-reader-layout">
			<nav className="license-reader-index" aria-label={t("contents")}>
				<p>{t("contents")}</p>
				<div>
					{entries.map((entry) => (
						<button
							key={entry.id}
							type="button"
							aria-current={active === entry.id ? "location" : undefined}
							onClick={() => navigate(entry.id)}
						>
							{entry.title}
						</button>
					))}
				</div>
			</nav>

			<section
				ref={viewport}
				className="license-reader-viewport"
				aria-label={t("document")}
				tabIndex={0}
				onScroll={trackSection}
			>
				<div className="license-reader-document" lang="en">
					<section
						data-license-section="introduction"
						aria-label="Apache License, Version 2.0"
					>
						<pre>{document.introduction}</pre>
					</section>
					{document.sections.map((section) => (
						<section
							key={section.id}
							data-license-section={section.id}
							aria-label={section.title}
						>
							<pre>{section.raw}</pre>
						</section>
					))}
				</div>
			</section>
		</div>
	);
}

export function LicenseReader({ text }: { text: string }) {
	const t = useTranslations("LicenseReader");
	const [large, setLarge] = useState(false);
	const [copyState, setCopyState] = useState<"idle" | "copied" | "error">(
		"idle",
	);
	useEffect(() => {
		if (copyState === "idle") return;
		const timer = window.setTimeout(() => setCopyState("idle"), 2500);
		return () => window.clearTimeout(timer);
	}, [copyState]);
	async function copy() {
		try {
			await navigator.clipboard.writeText(text);
			setCopyState("copied");
		} catch {
			setCopyState("error");
		}
	}
	const controls = (
		<>
			<Button variant="ghost" size="sm" onClick={copy} aria-label={t("copy")}>
				{copyState === "copied" ? (
					<Check size={15} />
				) : (
					<ClipboardCopy size={15} />
				)}
				<span>{copyState === "copied" ? t("copied") : t("copy")}</span>
			</Button>
			<Button
				variant="ghost"
				size="icon"
				onClick={() => setLarge((value) => !value)}
				aria-label={t("textSize")}
				aria-pressed={large}
			>
				<Type size={17} />
			</Button>
			<Button
				variant="ghost"
				size="icon"
				nativeButton={false}
				aria-label={t("download")}
				render={<a href="/licenses/Apache-2.0.txt" download="Apache-2.0.txt" />}
			>
				<ArrowDownToLine size={17} />
			</Button>
		</>
	);
	return (
		<Dialog>
			<div className="license-reader" data-large={large}>
				<div className="license-reader-toolbar">
					<div className="license-reader-filename">
						<FileText size={18} />
						<span>
							<strong>Apache-2.0.txt</strong>
							<small>{t("language")}</small>
						</span>
					</div>
					<div className="license-reader-controls">
						{controls}
						<DialogTrigger
							render={
								<Button variant="ghost" size="icon" aria-label={t("expand")} />
							}
						>
							<Maximize2 size={17} />
						</DialogTrigger>
					</div>
				</div>
				<LicenseDocument text={text} />
				<div className="license-reader-status" role="status">
					{copyState === "error"
						? t("copyError")
						: copyState === "copied"
							? t("copied")
							: t("original")}
				</div>
			</div>
			<DialogContent
				className="license-reader-dialog translate-x-0 translate-y-0"
				showCloseButton={false}
			>
				<DialogTitle className="sr-only">{t("document")}</DialogTitle>
				<DialogDescription className="sr-only">
					{t("language")}
				</DialogDescription>
				<div
					className="license-reader license-reader-expanded"
					data-large={large}
				>
					<div className="license-reader-toolbar">
						<div className="license-reader-filename">
							<FileText size={18} />
							<span>
								<strong>Apache-2.0.txt</strong>
								<small>{t("language")}</small>
							</span>
						</div>
						<div className="license-reader-controls">
							{controls}
							<DialogClose
								render={
									<Button variant="ghost" size="icon" aria-label={t("close")} />
								}
							>
								<X size={18} />
							</DialogClose>
						</div>
					</div>
					<LicenseDocument text={text} />
					<div className="license-reader-status" role="status">
						{copyState === "error"
							? t("copyError")
							: copyState === "copied"
								? t("copied")
								: t("original")}
					</div>
				</div>
			</DialogContent>
		</Dialog>
	);
}

"use client";

import { ArrowUpRight, BookOpen, Search, X } from "lucide-react";
import { useTranslations } from "next-intl";
import { useTopLoader } from "nextjs-toploader";
import { useEffect, useRef, useState } from "react";
import { Button } from "@/components/ui/button";
import {
	Command,
	CommandEmpty,
	CommandGroup,
	CommandInput,
	CommandItem,
	CommandList,
} from "@/components/ui/command";
import {
	Dialog,
	DialogClose,
	DialogContent,
	DialogDescription,
	DialogTitle,
	DialogTrigger,
} from "@/components/ui/dialog";
import { usePathname, useRouter } from "@/i18n/navigation";
import { type DocSearchEntry, normalizeSearch } from "@/lib/docs-search";

export function DocsSearch({ entries }: { entries: DocSearchEntry[] }) {
	const t = useTranslations("DocsSearch");
	const router = useRouter();
	const pathname = usePathname();
	const loader = useTopLoader();
	const [open, setOpen] = useState(false);
	const [query, setQuery] = useState("");
	const input = useRef<HTMLInputElement>(null);
	useEffect(() => {
		function shortcut(event: KeyboardEvent) {
			if (
				event.isComposing ||
				event.key.toLowerCase() !== "k" ||
				!(event.metaKey || event.ctrlKey)
			)
				return;
			event.preventDefault();
			setOpen((value) => !value);
		}
		window.addEventListener("keydown", shortcut);
		return () => window.removeEventListener("keydown", shortcut);
	}, []);
	function changeOpen(value: boolean) {
		setOpen(value);
		if (!value) setQuery("");
	}
	return (
		<Dialog open={open} onOpenChange={changeOpen}>
			<DialogTrigger
				render={
					<Button
						variant="outline"
						className="docs-search-trigger"
						aria-label={t("title")}
					/>
				}
			>
				<Search size={16} />
				<span>{t("placeholder")}</span>
				<kbd>Ctrl / ⌘ K</kbd>
			</DialogTrigger>
			<DialogContent
				className="docs-search-dialog translate-x-0 translate-y-0"
				showCloseButton={false}
			>
				<DialogTitle className="sr-only">{t("title")}</DialogTitle>
				<DialogDescription className="sr-only">
					{t("description")}
				</DialogDescription>
				<Command
					label={t("title")}
					filter={(value, search) =>
						normalizeSearch(value).includes(normalizeSearch(search)) ? 1 : 0
					}
				>
					<div className="docs-search-input-row">
						<CommandInput
							ref={input}
							value={query}
							onValueChange={setQuery}
							placeholder={t("placeholder")}
							aria-label={t("title")}
						/>
						{query && (
							<Button
								variant="ghost"
								size="icon"
								aria-label={t("clear")}
								onClick={() => {
									setQuery("");
									input.current?.focus();
								}}
							>
								<X size={16} />
							</Button>
						)}
						<DialogClose
							render={
								<Button variant="ghost" size="icon" aria-label={t("close")} />
							}
						>
							<X size={16} />
						</DialogClose>
					</div>
					<CommandList>
						<CommandEmpty>{t("empty")}</CommandEmpty>
						{[...new Set(entries.map((entry) => entry.group))].map((group) => (
							<CommandGroup heading={group} key={group}>
								{entries
									.filter(
										(entry) =>
											entry.group === group &&
											(query.trim() || !entry.href.includes("#")),
									)
									.map((entry) => (
										<CommandItem
											key={entry.id}
											value={`${entry.title} ${entry.page} ${entry.href.includes("#") ? "" : entry.description} ${entry.id}`}
											onSelect={() => {
												changeOpen(false);
												if (entry.href.split("#")[0] !== pathname)
													loader.start();
												router.push(entry.href);
											}}
										>
											<BookOpen size={16} />
											<span className="search-result-copy">
												<strong>{entry.title}</strong>
												<small>
													{entry.href.includes("#")
														? entry.page
														: entry.description}
												</small>
											</span>
											<ArrowUpRight size={14} />
										</CommandItem>
									))}
							</CommandGroup>
						))}
					</CommandList>
				</Command>
				<p className="docs-search-footer">{t("hint")}</p>
			</DialogContent>
		</Dialog>
	);
}

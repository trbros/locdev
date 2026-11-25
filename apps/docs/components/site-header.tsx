"use client";

import { ArrowRight, ArrowUpRight, Menu, X } from "lucide-react";
import { useTranslations } from "next-intl";
import { useEffect, useState } from "react";
import { DocsSearch } from "@/components/docs/docs-search";
import { Button } from "@/components/ui/button";
import {
	Dialog,
	DialogClose,
	DialogContent,
	DialogDescription,
	DialogTitle,
	DialogTrigger,
} from "@/components/ui/dialog";
import { Link, usePathname } from "@/i18n/navigation";
import type { DocSearchEntry } from "@/lib/docs-search";
import { githubRepo } from "@/lib/repository";
import { route } from "@/lib/routes.generated";
import { Brand } from "./brand";
import { HeaderMegaMenu, MegaMenuGroups } from "./header-mega-menu";
import { GitHub } from "./icons";
import { LocaleSwitcher, ThemeSwitch } from "./site-controls";

const pages = [
	{ path: "/", key: "product" },
	{ path: "/features", key: "features" },
	{ path: "/stacks", key: "stacks" },
	{ path: "/download", key: "download" },
	{ path: route("docs"), key: "docs" },
] as const;

export function SiteHeader({
	searchEntries,
}: {
	searchEntries: DocSearchEntry[];
}) {
	const t = useTranslations("Nav");
	const l = useTranslations("Labels");
	const pathname = usePathname();
	const [open, setOpen] = useState(false);
	useEffect(() => {
		const desktop = window.matchMedia("(min-width: 1100px)");
		const close = () => {
			if (desktop.matches) setOpen(false);
		};
		desktop.addEventListener("change", close);
		return () => desktop.removeEventListener("change", close);
	}, []);
	const active = (path: string) =>
		pathname === path || (path !== "/" && pathname.startsWith(`${path}/`));
	return (
		<>
			<a className="skip-link" href="#main-content">
				{t("skip")}
			</a>
			<div className="microbar">
				<span className="inline-flex items-center gap-2">
					<i className="led led-green" /> {l("headerTag")}
				</span>
				<span className="hidden sm:inline">{l("poweredBy")}</span>
			</div>
			<header className="site-header">
				<div className="page-container header-row">
					<Brand />
					<nav className="header-desktop-nav" aria-label={t("mainNav")}>
						{pages.map(({ path, key }) => (
							<Link
								key={key}
								href={path}
								aria-current={active(path) ? "page" : undefined}
								className={`nav-link ${active(path) ? "nav-link-active" : ""}`}
							>
								{t(key)}
							</Link>
						))}
						<HeaderMegaMenu />
					</nav>
					<div className="header-actions">
						<DocsSearch entries={searchEntries} />
						<span className="header-desktop-control">
							<LocaleSwitcher />
						</span>
						<ThemeSwitch />
						<Button
							variant="outline"
							className="header-repository"
							nativeButton={false}
							render={
								<a
									href={githubRepo}
									target="_blank"
									rel="noopener noreferrer"
								/>
							}
						>
							<GitHub className="size-4" />
							<span>GitHub</span>
							<ArrowUpRight size={14} />
						</Button>
						<Dialog open={open} onOpenChange={setOpen}>
							<DialogTrigger
								render={
									<Button
										variant="ghost"
										size="icon"
										className="header-mobile-trigger"
										aria-label={t("menu")}
									/>
								}
							>
								<Menu size={21} />
							</DialogTrigger>
							<DialogContent
								className="mobile-nav-sheet translate-x-0 translate-y-0"
								showCloseButton={false}
							>
								<div className="mobile-nav-title">
									<DialogTitle>{t("mainNav")}</DialogTitle>
									<DialogClose
										render={
											<Button
												variant="ghost"
												size="icon"
												aria-label={t("close")}
											/>
										}
									>
										<X size={20} />
									</DialogClose>
								</div>
								<DialogDescription className="sr-only">
									{t("mobileNav")}
								</DialogDescription>
								<div className="mobile-nav-scroll">
									<nav className="mobile-nav-links" aria-label={t("mobileNav")}>
										{pages.map(({ path, key }) => (
											<Link
												key={key}
												href={path}
												onClick={() => setOpen(false)}
												aria-current={active(path) ? "page" : undefined}
											>
												{t(key)}
												<ArrowRight size={16} aria-hidden="true" />
											</Link>
										))}
									</nav>
									<MegaMenuGroups
										collapsible
										onNavigate={() => setOpen(false)}
									/>
								</div>
								<div className="mobile-nav-bottom">
									<LocaleSwitcher />
									<Button
										variant="outline"
										nativeButton={false}
										render={
											<a
												href={githubRepo}
												target="_blank"
												rel="noopener noreferrer"
											/>
										}
									>
										<GitHub className="size-4" />
										GitHub
										<ArrowUpRight size={14} />
									</Button>
								</div>
							</DialogContent>
						</Dialog>
					</div>
				</div>
			</header>
		</>
	);
}

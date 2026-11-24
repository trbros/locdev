"use client";
import { Popover } from "@base-ui/react/popover";
import {
	ArrowRight,
	BookOpen,
	ChevronDown,
	CircleHelp,
	Code2,
	Download,
	FileClock,
	Heart,
	Info,
	Layers,
	LifeBuoy,
	MessagesSquare,
	Scale,
	Settings2,
	Users,
} from "lucide-react";
import { useTranslations } from "next-intl";
import { useEffect, useId, useState } from "react";
import { Button } from "@/components/ui/button";
import { Link, usePathname } from "@/i18n/navigation";
import { route } from "@/lib/routes.generated";
export const resourceGroups = [
	{
		key: "build",
		items: [
			{ path: "/get-started", key: "getStarted", icon: Download },
			{ path: "/configure", key: "configure", icon: Settings2 },
			{ path: route("docs"), key: "docs", icon: BookOpen },
			{ path: "/support", key: "support", icon: LifeBuoy },
			{ path: "/faq", key: "faq", icon: CircleHelp },
		],
	},
	{
		key: "ecosystem",
		items: [
			{ path: "/stacks", key: "stacks", icon: Code2 },
			{ path: "/services", key: "services", icon: Layers },
			{ path: "/compare", key: "compare", icon: Scale },
		],
	},
	{
		key: "community",
		items: [
			{ path: "/about", key: "about", icon: Info },
			{ path: "/contact", key: "contact", icon: MessagesSquare },
			{ path: "/changelog", key: "changelog", icon: FileClock },
			{ path: "/contribute", key: "contribute", icon: Users },
			{ path: "/donate", key: "donate", icon: Heart },
		],
	},
] as const;
export const headerResources = resourceGroups.flatMap((group) => [
	...group.items,
]);
export function MegaMenuGroups({
	onNavigate,
	collapsible = false,
}: {
	onNavigate: () => void;
	collapsible?: boolean;
}) {
	const t = useTranslations("Nav");
	const m = useTranslations("MegaMenu");
	const pathname = usePathname();
	const id = useId();
	const Group = collapsible ? "details" : "section";
	const Heading = collapsible ? "summary" : "h2";
	return (
		<div className="mega-menu-groups">
			{resourceGroups.map((group) => (
				<Group
					key={group.key}
					open={
						collapsible
							? group.items.some(
									({ path }) =>
										pathname === path || pathname.startsWith(`${path}/`),
								)
							: undefined
					}
					className={collapsible ? "mobile-nav-group" : "mega-menu-group"}
				>
					<Heading id={`${id}-${group.key}`}>
						{m(`groups.${group.key}`)}
						{collapsible && <ChevronDown size={16} aria-hidden="true" />}
					</Heading>
					{group.items.map(({ path, key, icon: Icon }) => (
						<Link
							key={key}
							href={path}
							className="mega-menu-link"
							aria-current={
								pathname === path || pathname.startsWith(`${path}/`)
									? "page"
									: undefined
							}
							onClick={onNavigate}
						>
							<Icon size={18} aria-hidden="true" />
							<span>
								<strong>{t(key)}</strong>
								<small>{m(`descriptions.${key}`)}</small>
							</span>
						</Link>
					))}
				</Group>
			))}
		</div>
	);
}
export function HeaderMegaMenu() {
	const t = useTranslations("Nav");
	const m = useTranslations("MegaMenu");
	const pathname = usePathname();
	const [open, setOpen] = useState(false);
	useEffect(() => {
		const desktop = window.matchMedia("(min-width: 1100px)");
		const closeOnMobile = () => {
			if (!desktop.matches) setOpen(false);
		};
		desktop.addEventListener("change", closeOnMobile);
		return () => desktop.removeEventListener("change", closeOnMobile);
	}, []);

	const active = headerResources.some(
		({ path }) => pathname === path || pathname.startsWith(`${path}/`),
	);
	return (
		<Popover.Root open={open} onOpenChange={setOpen}>
			<Popover.Trigger
				render={
					<Button
						variant="ghost"
						className={`nav-link header-explore ${active && !["/stacks", "/docs"].some((path) => pathname.startsWith(path)) ? "nav-link-active" : ""}`}
					/>
				}
			>
				<span className="header-explore-label">{t("explore")}</span>
				<ChevronDown size={12} />
			</Popover.Trigger>
			<Popover.Portal>
				<Popover.Positioner
					sideOffset={16}
					align="center"
					className="mega-menu-positioner"
				>
					<Popover.Popup className="header-mega-menu" aria-label={t("explore")}>
						<MegaMenuGroups onNavigate={() => setOpen(false)} />
						<div className="mega-menu-footer">
							<span>{m("footer")}</span>
							<Link href="/get-started" onClick={() => setOpen(false)}>
								{t("getStarted")}
								<ArrowRight size={14} />
							</Link>
						</div>
					</Popover.Popup>
				</Popover.Positioner>
			</Popover.Portal>
		</Popover.Root>
	);
}

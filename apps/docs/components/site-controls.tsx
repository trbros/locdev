"use client";
import { Check, Moon, Sun } from "lucide-react";
import Image from "next/image";
import { useLocale, useTranslations } from "next-intl";
import { useTheme } from "next-themes";
import { useEffect, useState } from "react";
import { Button } from "@/components/ui/button";
import {
	DropdownMenu,
	DropdownMenuContent,
	DropdownMenuItem,
	DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { localeFileKey, localizedPathname } from "@/i18n/locales";
import { usePathname } from "@/i18n/navigation";
export function ThemeSwitch() {
	const { resolvedTheme, setTheme } = useTheme();
	const t = useTranslations("Nav");
	const [mounted, setMounted] = useState(false);
	useEffect(() => setMounted(true), []);
	const light = mounted && resolvedTheme === "light";
	return (
		<Button
			variant="ghost"
			size="icon"
			className="header-icon"
			aria-label={light ? t("darkTheme") : t("lightTheme")}
			onClick={() => setTheme(light ? "dark" : "light")}
		>
			{light ? <Moon size={18} /> : <Sun size={18} />}
		</Button>
	);
}

export function LocaleSwitcher() {
	const t = useTranslations("Nav");
	const locale = useLocale();
	const pathname = usePathname();
	const [locationSuffix, setLocationSuffix] = useState("");
	const languages = [
		{ code: "pt-BR", name: "Português (Brasil)", flag: "br" },
		{ code: "en", name: "English", flag: "us" },
		{ code: "es", name: "Español", flag: "es" },
		{ code: "ja", name: "日本語", flag: "jp" },
	] as const;
	const current = languages.find(({ code }) => code === locale) ?? languages[0];
	return (
		<DropdownMenu
			onOpenChange={(open) => {
				if (open)
					setLocationSuffix(window.location.search + window.location.hash);
			}}
		>
			<DropdownMenuTrigger
				render={
					<Button
						variant="ghost"
						className="header-language"
						aria-label={t("language")}
					/>
				}
			>
				<Image
					src={`/flags/nucleo/${current.flag}.svg`}
					width={24}
					height={24}
					className="size-6 shrink-0"
					alt=""
					aria-hidden="true"
				/>
				<span>{localeFileKey(locale).toUpperCase()}</span>
			</DropdownMenuTrigger>
			<DropdownMenuContent align="end" className="locale-popup">
				{languages.map(({ code, name, flag }) => (
					<DropdownMenuItem
						key={code}
						nativeButton={false}
						render={
							<a href={localizedPathname(pathname, code) + locationSuffix} />
						}
					>
						<Image
							src={`/flags/nucleo/${flag}.svg`}
							width={24}
							height={24}
							className="size-6 shrink-0"
							alt=""
							aria-hidden="true"
						/>
						{name}
						{locale === code && (
							<Check size={14} className="ml-auto text-ember" />
						)}
					</DropdownMenuItem>
				))}
			</DropdownMenuContent>
		</DropdownMenu>
	);
}

import type { Metadata } from "next";
import { notFound } from "next/navigation";
import { hasLocale, NextIntlClientProvider } from "next-intl";
import { getTranslations, setRequestLocale } from "next-intl/server";
import NextTopLoader from "nextjs-toploader";
import type { ReactNode } from "react";
import { Analytics } from "@/components/analytics";
import { CookieConsent } from "@/components/cookie-consent/cookie-consent";
import { ThemeProvider } from "@/components/theme-provider";
import { languageTags } from "@/i18n/locales";
import { routing } from "@/i18n/routing";
import "../globals.css";

export function generateStaticParams() {
	return routing.locales.map((locale) => ({ locale }));
}

export async function generateMetadata({
	params,
}: {
	params: Promise<{ locale: string }>;
}): Promise<Metadata> {
	const { locale } = await params;
	if (!hasLocale(routing.locales, locale)) notFound();
	const t = await getTranslations({ locale, namespace: "Meta" });
	const origin =
		process.env.NEXT_PUBLIC_SITE_URL?.replace(/\/$/, "") ??
		"http://localhost:3000";
	return {
		title: { default: t("title"), template: `%s · LocDev` },
		description: t("description"),
		metadataBase: new URL(origin),
		openGraph: {
			type: "website",
			title: t("title"),
			description: t("description"),
			images: ["/og.svg"],
		},
		icons: { icon: "/favicon.ico", apple: "/logo.png" },
	};
}

export default async function RootLayout({
	children,
	params,
}: {
	children: ReactNode;
	params: Promise<{ locale: string }>;
}) {
	const { locale } = await params;
	if (!hasLocale(routing.locales, locale)) notFound();
	setRequestLocale(locale);
	return (
		<html
			lang={languageTags[locale]}
			className="dark"
			data-scroll-behavior="smooth"
			suppressHydrationWarning
		>
			<body className="grain min-h-screen bg-background text-foreground antialiased">
				<NextTopLoader
					color="var(--ember)"
					height={2}
					showSpinner={false}
					shadow={false}
					showForHashAnchor={false}
					zIndex={2000}
				/>
				<NextIntlClientProvider>
					<ThemeProvider>
						{children}
						<CookieConsent />
						<Analytics />
					</ThemeProvider>
				</NextIntlClientProvider>
			</body>
		</html>
	);
}

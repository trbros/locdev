"use client";
import { usePathname } from "next/navigation";
import { useTranslations } from "next-intl";
import { NotFoundScene } from "@/components/not-found/not-found-scene";
import "@/components/not-found/not-found.css";

export default function NotFound() {
	const t = useTranslations("NotFound");
	const pathname = usePathname();
	return (
		<section className="not-found-fullscreen" aria-label={t("title")}>
			<h1 className="sr-only">404 — {t("title")}</h1>
			<NotFoundScene key={pathname} />
		</section>
	);
}

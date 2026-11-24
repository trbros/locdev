"use client";
import { useTranslations } from "next-intl";
import { Button } from "@/components/ui/button";
import { Link } from "@/i18n/navigation";
export default function PageError({
	retry,
}: {
	error: unknown;
	retry: () => void;
}) {
	const t = useTranslations("PageError");
	return (
		<section className="page-container page-error" role="alert">
			<h1 className="section-heading">{t("title")}</h1>
			<p>{t("body")}</p>
			<div className="route-actions">
				<Button onClick={retry}>{t("retry")}</Button>
				<Link href="/" className="btn-outline">
					{t("home")}
				</Link>
			</div>
		</section>
	);
}

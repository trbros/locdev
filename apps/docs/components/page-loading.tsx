"use client";
import { useTranslations } from "next-intl";
import { Skeleton } from "@/components/ui/skeleton";
export function PageLoading({ cards = false }: { cards?: boolean }) {
	const t = useTranslations("Loading");
	return (
		<div
			className="page-container page-loading"
			role="status"
			aria-busy="true"
			aria-label={t("page")}
		>
			<span className="sr-only">{t("page")}</span>
			<Skeleton className="loading-eyebrow" />
			<Skeleton className="loading-heading" />
			<Skeleton className="loading-description" />
			<Skeleton className="loading-description loading-description-short" />
			{cards ? (
				<div className="stack-catalog-grid">
					{[1, 2, 3, 4, 5, 6, 7, 8].map((id) => (
						<Skeleton key={id} className="loading-card" />
					))}
				</div>
			) : (
				<>
					<Skeleton className="loading-status" />
					<div className="loading-panels">
						<Skeleton />
						<Skeleton />
					</div>
				</>
			)}
		</div>
	);
}

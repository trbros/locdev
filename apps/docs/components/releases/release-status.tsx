"use client";
import { ArrowUpRight, RefreshCw } from "lucide-react";
import { useRouter } from "next/navigation";
import { useLocale, useTranslations } from "next-intl";
import { useTransition } from "react";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";
import type { ReleaseFeed } from "@/lib/releases";
import { githubReleases } from "@/lib/repository";

export function ReleaseStatus({ feed }: { feed: ReleaseFeed }) {
	const t = useTranslations("Releases");
	const locale = useLocale();
	const router = useRouter();
	const [pending, start] = useTransition();
	return (
		<div
			className={`release-feed-status release-feed-${feed.status}`}
			role="status"
			aria-busy={pending}
		>
			<div>
				{pending && <Skeleton className="release-refresh-shimmer" />}
				<strong>
					{t(
						feed.status === "ok"
							? "connected"
							: feed.status === "empty"
								? "emptyTitle"
								: "unavailableTitle",
					)}
				</strong>
				<p>
					{t(
						feed.status === "ok"
							? "refreshInfo"
							: feed.status === "empty"
								? "emptyBody"
								: feed.reason === "not-public"
									? "notPublic"
									: "unavailableBody",
					)}
				</p>
				<span>
					{t("checked")}{" "}
					{new Intl.DateTimeFormat(locale, {
						dateStyle: "short",
						timeStyle: "short",
						timeZone: "UTC",
					}).format(new Date(feed.checkedAt))}{" "}
					UTC
				</span>
			</div>
			<div className="flex flex-wrap gap-2">
				<Button
					variant="outline"
					disabled={pending}
					onClick={() => start(() => router.refresh())}
				>
					<RefreshCw size={14} />
					{t(pending ? "refreshing" : "refresh")}
				</Button>
				<a
					href={githubReleases}
					target="_blank"
					rel="noopener noreferrer"
					className="btn-outline"
				>
					GitHub
					<ArrowUpRight size={14} />
				</a>
			</div>
		</div>
	);
}

"use client";
import { ArrowUpRight } from "lucide-react";
import { useTranslations } from "next-intl";
import { ClientHighlightedCode } from "@/components/docs/client-highlighted-code";
import { CodeBlock } from "@/components/docs/code-block";
import { locdevSupport, type StackService } from "@/lib/stack-services";
export function ServiceDetails({ service }: { service: StackService }) {
	const t = useTranslations("Services");
	return (
		<div className="service-detail">
			<p>{t(`supportNote.${locdevSupport(service.id)}`)}</p>
			<p>
				<strong>{t("requires")}</strong> {t(`requirements.${service.id}`)}
			</p>
			{service.command ? (
				<>
					<p>{t("example", { module: "<module>" })}</p>
					<CodeBlock code={service.command} language="bash">
						<ClientHighlightedCode code={service.command} language="bash" />
					</CodeBlock>
				</>
			) : (
				<p>{t(service.mode === "embedded" ? "embeddedNote" : "customNote")}</p>
			)}
			<a href={service.docs} target="_blank" rel="noopener noreferrer">
				{t("docs")}
				<ArrowUpRight size={13} />
			</a>
		</div>
	);
}

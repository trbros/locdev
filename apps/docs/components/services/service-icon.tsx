"use client";
import { Database, Radio, Timer, Wrench } from "lucide-react";
import Image from "next/image";
import { useTranslations } from "next-intl";
import logos from "@/lib/service-logos.json";
import type { StackService } from "@/lib/stack-services";

const brands: Record<
	string,
	{ name: keyof typeof logos; ecosystem?: boolean }
> = {
	postgres: { name: "PostgreSQL" },
	mysql: { name: "MySQL" },
	sqlite: { name: "SQLite" },
	redis: { name: "Redis" },
	mongodb: { name: "MongoDB" },
	sqlserver: { name: "Microsoft SQL Server" },
	reverb: { name: "Laravel", ecosystem: true },
	queue: { name: "Laravel", ecosystem: true },
	horizon: { name: "Laravel", ecosystem: true },
	scheduler: { name: "Laravel", ecosystem: true },
	vite: { name: "Vite" },
	solidqueue: { name: "Ruby", ecosystem: true },
	sidekiq: { name: "Ruby", ecosystem: true },
	cable: { name: "Ruby", ecosystem: true },
	celery: { name: "Python", ecosystem: true },
	beat: { name: "Python", ecosystem: true },
	channels: { name: "Django", ecosystem: true },
	nestcron: { name: "NestJS", ecosystem: true },
	socketio: { name: "Socket.IO" },
	wpcron: { name: "WordPress", ecosystem: true },
	messenger: { name: "Php", ecosystem: true },
	asynq: { name: "Go", ecosystem: true },
	apalis: { name: "Rust", ecosystem: true },
	springjobs: { name: "Spring", ecosystem: true },
	hangfire: { name: "Microsoft .NET", ecosystem: true },
	signalr: { name: "Microsoft .NET", ecosystem: true },
	storybook: { name: "Storybook" },
};
export function ServiceIcon({ service }: { service: StackService }) {
	const t = useTranslations("Services");
	const brand = brands[service.id];
	const Icon = {
		database: Database,
		background: Timer,
		realtime: Radio,
		tooling: Wrench,
	}[service.category];
	if (!brand)
		return (
			<span className="service-logo" title={t("categoryIcon")}>
				<Icon size={38} aria-hidden="true" />
			</span>
		);
	const entry = logos[brand.name];
	return (
		<span
			className="service-logo"
			title={
				brand.ecosystem ? t("ecosystemLogo", { name: brand.name }) : brand.name
			}
		>
			<Image
				src={entry.paths.light}
				alt=""
				width={44}
				height={44}
				className="dark:hidden"
			/>
			<Image
				src={entry.paths.dark}
				alt=""
				width={44}
				height={44}
				className="hidden dark:block"
			/>
		</span>
	);
}

import { Link } from "@/i18n/navigation";

// The official PNG logo, used as is (no SVG redraw). public/logo.png is written
// by `pnpm icons` in apps/desktop from apps/desktop/branding/logo.png.
export function LogoMark({ className = "size-9" }: { className?: string }) {
	return (
		// biome-ignore lint/performance/noImgElement: a 256 px static PNG, same file as the app
		<img
			src="/logo.png"
			alt=""
			aria-hidden="true"
			draggable={false}
			width={256}
			height={256}
			className={`${className} object-contain select-none`}
		/>
	);
}

export function Brand({ small = false }: { small?: boolean }) {
	return (
		<Link
			href="/"
			className="group inline-flex items-center gap-2.5"
			aria-label="LocDev — home"
		>
			<LogoMark className={small ? "size-7" : "size-9"} />
			<span
				className={`${small ? "text-lg" : "text-[21px]"} font-semibold tracking-[-0.06em] leading-none text-foreground group-hover:text-ember transition-colors`}
			>
				locdev<span className="text-ember group-hover:text-foreground">.</span>
			</span>
		</Link>
	);
}

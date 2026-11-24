import { ImageIcon } from "lucide-react";
import { getLocale } from "next-intl/server";
import { shotUrl } from "@/lib/screenshots";

/** Real captures of the app, in the page's language. Both themes are in the
 *  page and CSS shows the one for the site's theme, so switching never
 *  flashes (the hidden one is lazy and never downloads). */
export async function Screenshot({ name, alt }: { name: string; alt: string }) {
	const locale = await getLocale();
	const dark = shotUrl(locale, "dark", name);
	const light = shotUrl(locale, "light", name);
	const first = dark ?? light;
	if (!first) return null;
	return (
		<figure className="doc-screenshot">
			<div className="doc-screenshot-head">
				<span className="inline-flex items-center gap-2">
					<ImageIcon size={12} /> LOCDEV / {name.toUpperCase()}
				</span>
				<span>APP CAPTURE</span>
			</div>
			{/* eslint-disable-next-line @next/next/no-img-element */}
			<img
				className={dark && light ? "shot-dark" : undefined}
				src={first}
				alt={alt}
				loading="lazy"
				width={1280}
				height={800}
			/>
			{dark && light && (
				// eslint-disable-next-line @next/next/no-img-element
				<img
					className="shot-light"
					src={light}
					alt={alt}
					loading="lazy"
					width={1280}
					height={800}
				/>
			)}
			<figcaption>{alt}</figcaption>
		</figure>
	);
}

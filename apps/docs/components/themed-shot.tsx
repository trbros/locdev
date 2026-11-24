import Image, { type ImageProps } from "next/image";
import { getLocale } from "next-intl/server";
import { shotUrl } from "@/lib/screenshots";

/** An app capture in the page's language and both themes; CSS shows the one
 *  matching the site's theme (html.light / html.dark, set before paint), so
 *  it never flashes. */
export async function ThemedShot({
	name,
	...props
}: Omit<ImageProps, "src"> & { name: string }) {
	const locale = await getLocale();
	const dark = shotUrl(locale, "dark", name);
	const light = shotUrl(locale, "light", name);
	return (
		<>
			{dark && <Image {...props} className="shot-dark" src={dark} />}
			{light && <Image {...props} className="shot-light" src={light} />}
		</>
	);
}

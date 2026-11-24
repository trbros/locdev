import { type NextRequest, NextResponse } from "next/server";
import createMiddleware from "next-intl/middleware";
import { canonicalRedirect } from "./i18n/redirects";
import { routing } from "./i18n/routing";

const handleI18n = createMiddleware(routing);
export default function proxy(request: NextRequest) {
	const canonical = canonicalRedirect(request.nextUrl.pathname);
	if (canonical !== null) {
		const target = request.nextUrl.clone();
		target.pathname = canonical;
		return NextResponse.redirect(target, 308);
	}
	return handleI18n(request);
}
export const config = { matcher: "/((?!api|_next|_vercel|.*\\..*).*)" };

/** Public app version, embedded by Next.js when building the documentation. */
const configuredVersion = process.env.NEXT_PUBLIC_APP_VERSION?.trim();
if (!configuredVersion) {
	throw new Error(
		"Set NEXT_PUBLIC_APP_VERSION in .env before starting or building the site.",
	);
}
export const appVersion = configuredVersion.startsWith("v")
	? configuredVersion
	: `v${configuredVersion}`;

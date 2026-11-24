export const consentKey = "locdev-cookie-preferences-v1";
export const dismissalKey = "locdev-cookie-notice-dismissed-v1";
/** Fired on window when the choice changes in this tab (storage events only reach other tabs). */
export const consentEvent = "locdev-cookie-preferences";

export type CookiePreferences = {
	version: 1;
	necessary: true;
	remember: boolean;
	/** Google Tag Manager / Analytics; off unless the visitor turns it on. */
	analytics: boolean;
	updatedAt: number;
};

export function parseCookiePreferences(
	raw: string | null,
): CookiePreferences | null {
	if (!raw) return null;
	try {
		const value: unknown = JSON.parse(raw);
		if (typeof value !== "object" || value === null) return null;
		const preference = value as Partial<CookiePreferences>;
		return preference.version === 1 &&
			preference.necessary === true &&
			typeof preference.remember === "boolean" &&
			typeof preference.updatedAt === "number" &&
			Number.isFinite(preference.updatedAt) &&
			preference.updatedAt > 0
			? {
					version: 1,
					necessary: true,
					remember: preference.remember,
					// Choices saved before analytics existed never granted it.
					analytics: preference.analytics === true,
					updatedAt: preference.updatedAt,
				}
			: null;
	} catch {
		return null;
	}
}

import "server-only";
import { publicContactUrl } from "./contact-links.ts";

const privacyContact = publicContactUrl(process.env.SITE_PRIVACY_CONTACT_URL);
const securityContact = publicContactUrl(process.env.SITE_SECURITY_REPORT_URL);
export const siteInformation = {
	privacyContact,
	hosting: process.env.SITE_HOSTING_PROVIDER?.trim() || null,
	retention: process.env.SITE_HOSTING_LOG_RETENTION?.trim() || null,
	securityContact:
		process.env.SITE_SECURITY_REPORTING_CONFIRMED === "true"
			? securityContact
			: null,
};
export const privacyConfirmed = Boolean(
	siteInformation.privacyContact && siteInformation.hosting,
);

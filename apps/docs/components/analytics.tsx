"use client";

import { GoogleTagManager } from "@next/third-parties/google";
import { useEffect, useState } from "react";
import {
	consentEvent,
	consentKey,
	parseCookiePreferences,
} from "@/components/cookie-consent/preferences";

const gtmId = process.env.NEXT_PUBLIC_GTM_ID?.trim();

function consented() {
	for (const storage of ["localStorage", "sessionStorage"] as const) {
		try {
			const saved = parseCookiePreferences(window[storage].getItem(consentKey));
			if (saved) return saved.analytics;
		} catch {
			/* Storage blocked: no consent. */
		}
	}
	return false;
}

/** Google Tag Manager, only after the visitor turns analytics on. */
export function Analytics() {
	const [on, setOn] = useState(false);
	useEffect(() => {
		if (!gtmId) return;
		const sync = () => {
			const next = consented();
			// A loaded tag can't be unloaded: withdrawing consent reloads the page without it.
			setOn((was) => {
				if (was && !next) window.location.reload();
				return next;
			});
		};
		sync();
		window.addEventListener(consentEvent, sync);
		window.addEventListener("storage", sync);
		return () => {
			window.removeEventListener(consentEvent, sync);
			window.removeEventListener("storage", sync);
		};
	}, []);
	return on && gtmId ? <GoogleTagManager gtmId={gtmId} /> : null;
}

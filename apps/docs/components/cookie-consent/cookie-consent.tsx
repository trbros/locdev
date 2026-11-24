"use client";

import { Dialog as DialogPrimitive } from "@base-ui/react/dialog";
import {
	Check,
	Cookie,
	LockKeyhole,
	Settings2,
	Terminal,
	X,
} from "lucide-react";
import { useTranslations } from "next-intl";
import { useEffect, useRef, useState } from "react";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import {
	Dialog,
	DialogClose,
	DialogDescription,
	DialogPortal,
	DialogTitle,
} from "@/components/ui/dialog";
import { Link } from "@/i18n/navigation";
import {
	type CookiePreferences,
	consentEvent,
	consentKey,
	dismissalKey,
	parseCookiePreferences,
} from "./preferences";
import "./cookie-consent.css";

function readStorage(persistent: boolean, key: string) {
	try {
		return (persistent ? localStorage : sessionStorage).getItem(key);
	} catch {
		return null;
	}
}

export function CookieConsent() {
	const t = useTranslations("Cookies");
	const [ready, setReady] = useState(false);
	const [banner, setBanner] = useState(false);
	const [open, setOpen] = useState(false);
	const [remember, setRemember] = useState(true);
	const [analytics, setAnalytics] = useState(false);
	const [storageFailed, setStorageFailed] = useState(false);
	const opener = useRef<HTMLButtonElement | null>(null);
	const launcher = useRef<HTMLButtonElement | null>(null);
	useEffect(() => {
		const sync = () => {
			const saved =
				parseCookiePreferences(readStorage(false, consentKey)) ??
				parseCookiePreferences(readStorage(true, consentKey));
			setRemember(saved?.remember ?? true);
			setAnalytics(saved?.analytics ?? false);
			setBanner(!saved && readStorage(false, dismissalKey) !== "1");
			setReady(true);
		};
		sync();
		const onStorage = (event: StorageEvent) => {
			if (event.key === consentKey || event.key === null) sync();
		};
		window.addEventListener("storage", onStorage);
		return () => window.removeEventListener("storage", onStorage);
	}, []);
	useEffect(() => {
		if (!storageFailed) return;
		const timer = window.setTimeout(() => setStorageFailed(false), 6000);
		return () => window.clearTimeout(timer);
	}, [storageFailed]);
	function dismiss() {
		// Closing the notice never grants consent.
		try {
			sessionStorage.setItem(dismissalKey, "1");
		} catch {
			/* Dismissal still applies to this page. */
		}
		setBanner(false);
	}
	function save(persist: boolean, withAnalytics: boolean) {
		const preference: CookiePreferences = {
			version: 1,
			necessary: true,
			remember: persist,
			analytics: withAnalytics,
			updatedAt: Date.now(),
		};
		let written = false;
		try {
			const target = persist ? localStorage : sessionStorage;
			target.setItem(consentKey, JSON.stringify(preference));
			written = true;
		} catch {
			/* Keep the current choice in component state if storage is unavailable. */
		}
		try {
			(persist ? sessionStorage : localStorage).removeItem(consentKey);
		} catch {
			/* Storage access can be restricted independently. */
		}
		try {
			sessionStorage.removeItem(dismissalKey);
		} catch {
			/* No effect on the saved choice. */
		}
		setRemember(persist);
		setAnalytics(withAnalytics);
		window.dispatchEvent(new CustomEvent(consentEvent, { detail: preference }));
		setStorageFailed(!written);
		setBanner(false);
		setOpen(false);
	}
	function customize(button: HTMLButtonElement) {
		opener.current = button;
		setOpen(true);
	}
	if (!ready) return null;
	return (
		<>
			{banner ? (
				<aside
					className="cookie-terminal"
					aria-labelledby="cookie-notice-title"
				>
					<div className="cookie-titlebar">
						<span>
							<Terminal size={14} /> privacy.config
						</span>
						<Button
							variant="ghost"
							size="icon-sm"
							aria-label={t("close")}
							onClick={dismiss}
						>
							<X size={15} />
						</Button>
					</div>
					<div className="cookie-notice-body">
						<span className="cookie-command">
							<span>❯</span> privacy --preferences
						</span>
						<h2 id="cookie-notice-title">{t("title")}</h2>
						<p>{t("body")}</p>
						<div className="cookie-signal">
							<span /> {t("noTracking")}
						</div>
						<Link href="/privacy">{t("privacy")}</Link>
					</div>
					<div className="cookie-notice-actions">
						<Button
							variant="outline"
							onClick={(event) => customize(event.currentTarget)}
						>
							<Settings2 size={14} />
							{t("customize")}
						</Button>
						<Button variant="outline" onClick={() => save(true, false)}>
							{t("necessaryOnly")}
						</Button>
						<Button className="cookie-accept" onClick={() => save(true, true)}>
							<Check size={14} />
							{t("consent")}
						</Button>
					</div>
				</aside>
			) : (
				<Button
					ref={launcher}
					variant="outline"
					size="icon"
					className="cookie-launcher"
					aria-label={t("reopen")}
					title={t("reopen")}
					onClick={(event) => customize(event.currentTarget)}
				>
					<Cookie size={18} />
				</Button>
			)}
			{storageFailed && (
				<p className="cookie-storage-notice" role="status">
					{t("storageUnavailable")}
				</p>
			)}
			<Dialog open={open} onOpenChange={setOpen}>
				<DialogPortal>
					<DialogPrimitive.Backdrop className="cookie-backdrop" />
					<DialogPrimitive.Popup
						className="cookie-preferences"
						finalFocus={() =>
							opener.current?.isConnected ? opener.current : launcher.current
						}
					>
						<div className="cookie-titlebar">
							<span>
								<Terminal size={14} /> privacy.config <i>/ {t("customize")}</i>
							</span>
							<DialogClose
								render={
									<Button
										variant="ghost"
										size="icon-sm"
										aria-label={t("close")}
									/>
								}
							>
								<X size={15} />
							</DialogClose>
						</div>
						<div className="cookie-preferences-body">
							<span className="cookie-command">
								<span>❯</span> privacy --configure
							</span>
							<DialogTitle>{t("preferencesTitle")}</DialogTitle>
							<DialogDescription>{t("preferencesBody")}</DialogDescription>
							<section className="cookie-category">
								<div>
									<LockKeyhole size={15} />
									<h3>{t("functional")}</h3>
									<span>{t("alwaysOn")}</span>
								</div>
								<p>{t("functionalBody")}</p>
								<code>NEXT_LOCALE · theme</code>
							</section>
							<section className="cookie-category">
								<div>
									<Cookie size={15} />
									<h3>{t("optional")}</h3>
								</div>
								<p>{t("optionalBody")}</p>
								<label
									className="cookie-remember"
									htmlFor="cookie-analytics-choice"
								>
									<Checkbox
										id="cookie-analytics-choice"
										checked={analytics}
										onCheckedChange={setAnalytics}
									/>
									<span>
										<strong>{t("analytics")}</strong>
										<small>{t("analyticsBody")}</small>
									</span>
								</label>
							</section>
							<label
								className="cookie-remember"
								htmlFor="cookie-remember-choice"
							>
								<Checkbox
									id="cookie-remember-choice"
									checked={remember}
									onCheckedChange={setRemember}
								/>
								<span>
									<strong>{t("remember")}</strong>
									<small>{t("rememberBody")}</small>
								</span>
							</label>
							<Link href="/privacy" onClick={() => setOpen(false)}>
								{t("privacy")}
							</Link>
						</div>
						<div className="cookie-notice-actions">
							<Button variant="outline" onClick={() => save(false, analytics)}>
								{t("sessionOnly")}
							</Button>
							<Button
								className="cookie-accept"
								onClick={() => save(remember, analytics)}
							>
								<Check size={14} />
								{t("save")}
							</Button>
						</div>
					</DialogPrimitive.Popup>
				</DialogPortal>
			</Dialog>
		</>
	);
}

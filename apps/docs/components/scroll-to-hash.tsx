"use client";

import { useEffect } from "react";

/** Scrolls to the element the URL's #hash names. A client-side navigation
 *  (the docs' version selector opening /changelog#v0.19.1) lands at the top
 *  of the new page; this brings that entry into view. */
export function ScrollToHash() {
	useEffect(() => {
		const scroll = () => {
			const id = decodeURIComponent(window.location.hash.slice(1));
			if (id) document.getElementById(id)?.scrollIntoView();
		};
		const frame = requestAnimationFrame(scroll);
		window.addEventListener("hashchange", scroll);
		return () => {
			cancelAnimationFrame(frame);
			window.removeEventListener("hashchange", scroll);
		};
	}, []);
	return null;
}

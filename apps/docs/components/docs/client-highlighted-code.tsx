"use client";

import parse from "html-react-parser";
import { useEffect, useState } from "react";

type Highlight = { code: string; language: string; html: string };

export function ClientHighlightedCode({
	code,
	language,
}: {
	code: string;
	language: string;
}) {
	const [result, setResult] = useState<Highlight | null>(null);
	useEffect(() => {
		let cancelled = false;
		import("@/lib/client-highlight")
			.then(({ highlightClientCode }) => highlightClientCode(code, language))
			.then((html) => {
				if (!cancelled) setResult({ code, language, html });
			})
			.catch(() => {
				/* Keep the original code readable if highlighting cannot load. */
			});
		return () => {
			cancelled = true;
		};
	}, [code, language]);
	const current =
		result?.code === code && result.language === language ? result.html : null;
	return (
		<div className="syntax-code">
			{current ? (
				parse(current)
			) : (
				<pre>
					<code>{code}</code>
				</pre>
			)}
		</div>
	);
}

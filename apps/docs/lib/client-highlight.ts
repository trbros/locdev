import { createHighlighterCore } from "shiki/core";
import { createJavaScriptRegexEngine } from "shiki/engine/javascript";
import bash from "shiki/langs/bash.mjs";
import powershell from "shiki/langs/powershell.mjs";
import toml from "shiki/langs/toml.mjs";
import dark from "shiki/themes/github-dark.mjs";
import light from "shiki/themes/github-light.mjs";

let highlighter: ReturnType<typeof createHighlighterCore> | undefined;

/** A small, browser-only bundle; project commands never leave the browser. */
export async function highlightClientCode(code: string, language: string) {
	highlighter ??= createHighlighterCore({
		themes: [light, dark],
		langs: [bash, powershell, toml],
		engine: createJavaScriptRegexEngine(),
	});
	const instance = await highlighter;
	const lang =
		language === "toml"
			? "toml"
			: language === "powershell"
				? "powershell"
				: language === "text"
					? "text"
					: "bash";
	return instance.codeToHtml(code, {
		lang,
		themes: { light: "github-light", dark: "github-dark" },
		defaultColor: false,
	});
}

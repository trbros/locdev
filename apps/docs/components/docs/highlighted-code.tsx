import "server-only";
import parse from "html-react-parser";
import { cache } from "react";
import { codeToHtml } from "shiki";

const languages: Record<string, string> = {
	bash: "bash",
	sh: "bash",
	shell: "bash",
	console: "shellsession",
	powershell: "powershell",
	toml: "toml",
	json: "json",
	text: "text",
	daemon: "shellsession",
};
const highlight = cache((code: string, language: string) =>
	codeToHtml(code, {
		lang: languages[language] ?? "text",
		themes: { light: "github-light", dark: "github-dark" },
		defaultColor: false,
	}),
);

export async function HighlightedCode({
	code,
	language,
}: {
	code: string;
	language: string;
}) {
	const html = await highlight(code, language);
	// Only Shiki's escaped, generated markup enters this server-owned boundary.
	return <div className="syntax-code">{parse(html)}</div>;
}

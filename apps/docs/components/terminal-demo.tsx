import { HighlightedCode } from "@/components/docs/highlighted-code";
import { TerminalDemoClient } from "./terminal-demo-client";

export function TerminalDemo() {
	const commands = [
		"locdev start",
		"locdev add ~/Code/acme --start",
		"locdev doctor",
	];
	return (
		<TerminalDemoClient
			highlightedCommands={commands.map((code) => (
				<HighlightedCode key={code} code={code} language="bash" />
			))}
		/>
	);
}

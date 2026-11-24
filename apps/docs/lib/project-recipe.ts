import type { Stack } from "./stacks";
export type RecipeInput = {
	name: string;
	path: string;
	command: string;
	port: string;
	autoStart: boolean;
	shell: "posix" | "powershell";
	stack: Stack;
};
export type RecipeError =
	| "nameError"
	| "pathError"
	| "commandError"
	| "portError";
export function quoteArgument(value: string, shell: RecipeInput["shell"]) {
	return `'${value.replaceAll("'", shell === "powershell" ? "''" : "'\\''")}'`;
}
function hasControl(value: string) {
	return Array.from(value).some(
		(char) => char.charCodeAt(0) < 32 || char.charCodeAt(0) === 127,
	);
}
export function buildProjectRecipe(input: RecipeInput): {
	errors: RecipeError[];
	cli: string;
	toml: string;
} {
	const errors: RecipeError[] = [];
	if (!/^[a-z0-9](?:[a-z0-9-]{0,61}[a-z0-9])?$/.test(input.name))
		errors.push("nameError");
	if (
		!/^(\/|[a-z]:[\\/]|\\\\[^\\]+\\[^\\]+)/i.test(input.path) ||
		hasControl(input.path)
	)
		errors.push("pathError");
	if (!input.command.trim() || hasControl(input.command))
		errors.push("commandError");
	const port = Number(input.port);
	if (
		!/^\d+$/.test(input.port) ||
		!Number.isInteger(port) ||
		port < 1024 ||
		port > 65535
	)
		errors.push("portError");
	if (errors.length) return { errors, cli: "", toml: "" };
	const quote = (value: string) => quoteArgument(value, input.shell);
	const kind =
		input.stack.id === "laravel"
			? "laravel"
			: input.stack.id === "wordpress"
				? "wordpress"
				: input.stack.id === "nextjs"
					? "next"
					: input.stack.id === "php"
						? "php"
						: input.stack.id === "html"
							? "static"
							: input.stack.id === "nodejs" || input.stack.support === "script"
								? "node"
								: "unknown";
	const cli = [
		"locdev start",
		`locdev add ${quote(input.path)} --name ${input.name} --command ${quote(input.command)} --port ${port}${input.autoStart ? " --auto-start" : ""}`,
		`locdev up ${input.name}`,
		`locdev domain ${input.name}`,
		`locdev logs ${input.name} -f`,
	].join("\n");
	const toml = [
		"version = 1",
		"",
		"[[project]]",
		`name = ${JSON.stringify(input.name)}`,
		`domain = ${JSON.stringify(`${input.name}.test`)}`,
		`path = ${JSON.stringify(input.path)}`,
		`kind = ${JSON.stringify(kind)}`,
		`command = ${JSON.stringify(input.command)}`,
		`port = ${port}`,
		`auto_start = ${input.autoStart}`,
	].join("\n");
	return { errors, cli, toml };
}

import { quoteArgument } from "./project-recipe.ts";
import type { ReleaseAsset, ReleasePlatform } from "./releases";
import { repository } from "./repository.ts";

export function cliInstallCommands(platform: ReleasePlatform, tag?: string) {
	if (tag && !/^[A-Za-z0-9._+-]+$/.test(tag))
		throw new Error("Unsupported release tag for install commands");
	const base = tag
		? `${repository.releases}/download/${encodeURIComponent(tag)}`
		: `${repository.releases}/latest/download`;
	if (platform === "windows") {
		const powershell = [
			`$env:LOCDEV_REPO = '${repository.slug}'`,
			...(tag
				? [`$env:LOCDEV_VERSION = ${quoteArgument(tag, "powershell")}`]
				: []),
			`Invoke-RestMethod '${base}/install.ps1' | Invoke-Expression`,
		].join("\n");
		return {
			shell: "PowerShell",
			language: "powershell",
			code: powershell,
			cmd: `powershell.exe -NoProfile -Command "${powershell.replaceAll("\n", "; ")}"`,
		};
	}
	const variables = [
		`LOCDEV_REPO=${quoteArgument(repository.slug, "posix")}`,
		...(tag ? [`LOCDEV_VERSION=${quoteArgument(tag, "posix")}`] : []),
	].join(" ");
	return {
		shell: platform === "macos" ? "zsh / Bash" : "Bash / sh",
		language: "bash",
		code: `curl -fsSL ${quoteArgument(`${base}/install.sh`, "posix")} | ${variables} sh`,
		cmd: null,
	};
}

/** Uses real release names and quotes them for the selected native shell. */
export function executableInstallCommand(
	asset: ReleaseAsset,
	platform: ReleasePlatform,
) {
	const file = quoteArgument(
		`./${asset.name}`,
		platform === "windows" ? "powershell" : "posix",
	);
	const name = asset.name.toLowerCase();
	if (platform === "linux") {
		if (name.endsWith(".deb")) return `sudo apt install ${file}`;
		if (name.endsWith(".rpm")) return `sudo dnf install ${file}`;
		if (name.endsWith(".appimage")) return `chmod +x ${file}\n${file}`;
	}
	if (platform === "macos" && name.endsWith(".dmg")) return `open ${file}`;
	if (platform === "windows") {
		if (name.endsWith(".msi"))
			return `Start-Process msiexec.exe -ArgumentList @('/i', ('"' + (Resolve-Path ${file}).Path + '"')) -Wait`;
		if (name.endsWith(".exe")) return `Start-Process -FilePath ${file} -Wait`;
	}
	return null;
}

export function sourceInstallCommands(
	platform: ReleasePlatform,
	mode: "desktop" | "cli",
) {
	const language = platform === "windows" ? "powershell" : "bash";
	const commands = [
		`git clone ${repository.clone} locdev`,
		"cd locdev",
		...(mode === "desktop"
			? [
					"cd apps/desktop",
					"pnpm install --frozen-lockfile",
					"pnpm tauri build",
				]
			: [
					"cargo build --release --locked -p locdev -p locdev-daemon",
					platform === "windows"
						? ".\\target\\release\\locdev.exe --version"
						: "./target/release/locdev --version",
				]),
	];
	return { language, code: commands.join("\n") };
}

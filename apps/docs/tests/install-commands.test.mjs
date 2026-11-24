import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import test from "node:test";
import {
	cliInstallCommands,
	executableInstallCommand,
	sourceInstallCommands,
} from "../lib/install-commands.ts";
import { repository } from "../lib/repository.ts";

test("CLI commands pin the selected release, configure the repository and use native shells", () => {
	for (const platform of ["linux", "macos"]) {
		const result = cliInstallCommands(platform, "v0.17.0-preview.1");
		assert.match(result.code, /LOCDEV_VERSION='v0\.17\.0-preview\.1'/);
		assert.ok(result.code.includes(repository.slug));
		assert.ok(result.code.includes("/download/v0.17.0-preview.1/install.sh"));
		assert.equal(
			spawnSync("sh", ["-n", "-c", result.code], { timeout: 2000 }).status,
			0,
		);
	}
	const windows = cliInstallCommands("windows", "v0.17.0");
	assert.match(windows.code, /\$env:LOCDEV_VERSION = 'v0\.17\.0'/);
	assert.match(windows.code, /Invoke-RestMethod.*install\.ps1/);
	assert.match(windows.cmd, /^powershell\.exe -NoProfile -Command "/);
	assert.ok(!windows.cmd.includes("\n"));
	assert.ok(
		cliInstallCommands("linux").code.includes("/latest/download/install.sh"),
	);
	assert.throws(
		() => cliInstallCommands("windows", 'v1"; whoami'),
		/Unsupported release tag/,
	);
});

test("native executable commands keep real filenames quoted and handle the published formats", () => {
	const asset = (name) => ({ name });
	assert.equal(
		executableInstallCommand(asset("LocDev amd64.deb"), "linux"),
		"sudo apt install './LocDev amd64.deb'",
	);
	assert.equal(
		executableInstallCommand(asset("LocDev.rpm"), "linux"),
		"sudo dnf install './LocDev.rpm'",
	);
	const appimage = executableInstallCommand(
		asset("LocDev's.AppImage"),
		"linux",
	);
	assert.equal(
		spawnSync("sh", ["-n", "-c", appimage], { timeout: 2000 }).status,
		0,
	);
	assert.equal(
		executableInstallCommand(asset("LocDev.dmg"), "macos"),
		"open './LocDev.dmg'",
	);
	assert.match(
		executableInstallCommand(asset("LocDev.msi"), "windows"),
		/msiexec\.exe/,
	);
	assert.match(
		executableInstallCommand(asset("LocDev.exe"), "windows"),
		/Start-Process -FilePath/,
	);
	assert.equal(executableInstallCommand(asset("LocDev.tar.gz"), "linux"), null);
});

test("source builds include both CLI binaries and delegate desktop sidecars to the Tauri build hook", () => {
	const windows = sourceInstallCommands("windows", "cli");
	assert.equal(windows.language, "powershell");
	assert.match(windows.code, /--locked -p locdev -p locdev-daemon/);
	assert.ok(windows.code.includes(".\\target\\release\\locdev.exe --version"));
	const desktop = sourceInstallCommands("macos", "desktop");
	assert.ok(desktop.code.includes(repository.clone));
	assert.match(
		desktop.code,
		/pnpm install --frozen-lockfile\npnpm tauri build/,
	);
});

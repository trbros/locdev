import assert from "node:assert/strict";
import test from "node:test";
import { repository } from "../lib/repository.ts";
import { frameworkRequestUrl, stacks, stages } from "../lib/stacks.ts";

test("only audited native detectors are promoted as native integrations", () => {
	assert.deepEqual(
		stacks
			.filter((stack) => stack.support === "detected")
			.map((stack) => stack.id)
			.sort(),
		["laravel", "nextjs", "nodejs", "php", "wordpress"],
	);
	assert.equal(new Set(stacks.map((stack) => stack.id)).size, stacks.length);
	assert.equal(stacks.length, 41);
	for (const stack of stacks) {
		assert.deepEqual(Object.keys(stack.commands), [...stages]);
		assert.ok(stack.docs.startsWith("https://"));
		assert.equal(
			stack.commands.deploy,
			null,
			"deployment requires a project/provider-specific workflow",
		);
	}
	assert.equal(stacks.find((stack) => stack.id === "rails").support, "custom");
	assert.equal(stacks.find((stack) => stack.id === "rust").support, "custom");
	assert.equal(stacks.find((stack) => stack.id === "vue").support, "script");
});

test("framework request opens a reviewable GitHub issue form with encoded input", () => {
	const name = "New stack & title=changed #日本語";
	const url = new URL(frameworkRequestUrl(name));
	assert.equal(url.origin, "https://github.com");
	assert.equal(url.pathname, new URL(repository.issues).pathname);
	assert.equal(url.searchParams.get("title"), `Framework integration: ${name}`);
	assert.ok(url.searchParams.get("body").includes(name));
	assert.equal(url.hash, "");
	assert.equal(
		new URL(frameworkRequestUrl()).searchParams.get("title"),
		"Framework integration request",
	);
});

test("workflow recipes preserve server arguments and use only actual LocDev commands", async () => {
	const { frameworkCommands, getLocdevCommands } = await import(
		"../lib/stack-workflow.ts"
	);
	assert.deepEqual(
		Object.keys(frameworkCommands).sort(),
		stacks.map((stack) => stack.id).sort(),
	);
	for (const stack of stacks) {
		const commands = getLocdevCommands(stack);
		assert.equal(commands[0], "locdev start");
		assert.ok(commands[1].startsWith(`locdev add . --name ${stack.id}`));
		assert.ok(commands.includes(`locdev up ${stack.id}`));
		assert.ok(commands.includes(`locdev domain ${stack.id}`));
		assert.equal(commands.at(-1), `locdev logs ${stack.id} -f`);
		if (stack.commands.run) assert.ok(commands[1].includes(stack.commands.run));
		else
			assert.ok(
				commands.includes(
					`locdev edit ${stack.id} --command '<HTTP_SERVER_COMMAND>'`,
				),
			);
	}
	const rails = stacks.find((stack) => stack.id === "rails");
	assert.ok(getLocdevCommands(rails)[1].includes("--port") === false);
	assert.ok(getLocdevCommands(rails)[1].includes("-p {port}"));
	const quoted = getLocdevCommands({
		...rails,
		commands: { ...rails.commands, run: "echo 'quoted'" },
	})[1];
	assert.ok(quoted.includes("'\\''quoted'\\''"));
});

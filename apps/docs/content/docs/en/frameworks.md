# Frameworks and stacks

When you add a folder, LocDev works out what the project is made of **by reading files only**: manifests (`package.json`, `composer.json`, `Gemfile`, `pyproject.toml`, `requirements.txt`, `Cargo.toml`, `go.mod`, `pom.xml`, `build.gradle`, `*.csproj`), lockfiles and config files. No project script is run.

The result is a list of **components** (framework, runtime, language, UI, tools), each with the evidence that proved it and its version when it can be known: the installed one (`node_modules`, `vendor/composer/installed.json`, `Gemfile.lock`, `uv.lock`, `poetry.lock`, `Cargo.lock`, `pom.xml`) or, without it, the declared range. One of them is the **primary**, the adapter that knows how to start the dev server.

```bash
locdev stack detect ~/Code/api     # any folder
locdev stack inspect shop          # a site, with evidence, versions and prerequisites
```

## Integration

| Kind                | Means                                                                                                                                     |
| ------------------- | ----------------------------------------------------------------------------------------------------------------------------------------- |
| native              | LocDev builds the command with host `127.0.0.1` and the site's port (`{port}`)                                                            |
| package.json script | runs a project script (`dev`, `start`…); when the framework doesn't take the port as an argument, the app has to read the `PORT` variable |
| custom command      | the framework was recognized, but the start command has to come from you (`locdev edit <site> --command`)                                 |
| component           | runs through another (React through Vite, Tailwind through the build)                                                                     |

Some adapters also pass the port in the framework's own variables: Spring Boot gets `SERVER_PORT` and `SERVER_ADDRESS`, and .NET gets `ASPNETCORE_URLS=http://127.0.0.1:<port>` (with `--no-launch-profile`, so `launchSettings.json` doesn't change the address), plus `ASPNETCORE_ENVIRONMENT=Development`, which the launch profile would set, unless the site already has that variable. Python projects with `.venv/` (or `venv/`) run with the virtual environment first on `PATH` and `VIRTUAL_ENV` set.

When several frameworks match (a NestJS app also has Express), the highest-priority one **that knows how to start this project** wins. A Vite installed with no script that runs it, for example, doesn't become the primary; generic Node.js takes over. Versions older than the adapter supports show up as a warning, without blocking.

## Monorepos

npm/Yarn workspaces (`workspaces`), `pnpm-workspace.yaml`, Cargo's `[workspace]` and `go.work` are read, and each package shows up with its own framework. Each package becomes a separate site (`locdev add apps/web`). At the root of a Cargo workspace with no `[package]`, no command is suggested.

## Manual choices

When detection isn't enough:

```bash
locdev stack add tailwind shop                 # a component the files don't prove
locdev stack remove python shop                # hide a detected component
locdev stack use flask shop --apply-command    # pick the adapter and use its command
locdev stack use auto shop                     # back to detection
```

The choices are stored in `projects.toml` (`adapter`, `stacks`, `hidden_stacks`). In the app, the **Stack** section of the site overview has the same control (**Edit…**).

## Updatable definitions

What each framework is (detection rules, version source, commands, config files, `.env` rules, links to the official docs) lives in **definitions**: plain data, separate from the code. LocDev ships a built-in set and can download a newer one without another app version:

- the set is published signed with minisign, and LocDev only uses a set whose signature matches the public key it ships;
- the format is versioned. A set that needs capabilities this version doesn't have is refused, and so are unknown fields;
- commands can only use programs from a fixed list (`npm`, `php`, `python3`, `bin/rails`, `cargo`, `dotnet`…), with no shell metacharacters, and paths only inside the project. Not even a signed set can run anything else;
- only a newer revision replaces the current one, and a revision older than the newest already seen is refused;
- `locdev stack definitions --rollback` goes back to the previous set (or the built-in one), and the abandoned revision is skipped in automatic checks;
- the check runs daily (through the job queue), respects offline mode and can be turned off: `locdev settings set automation.definitions off`.

```bash
locdev stack definitions             # revision in use, source, last check
locdev stack definitions --refresh   # check now
```

The state also shows in `doctor` and in **Settings → Frameworks**. Publishing is done by the `tools/definitions` tool (build, signing, verification), which also checks the registries (npm, Packagist, PyPI, RubyGems) for new major versions of the frameworks. While no update is published, LocDev uses the definitions it ships with, without a warning.

## Recognized stacks

<!-- generated:frameworks -->

The services each stack usually needs (databases, caches) show up in `locdev stack inspect`, and LocDev can create instances of them ([Services](/docs/services)). Queues, schedulers and WebSocket servers become the site's [auxiliary processes](/docs/servers).

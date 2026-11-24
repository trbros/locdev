# Uninstalling LocDev

Decide whether to remove a registered project, login startup or the entire installation. Uninstalling the app is different from deleting your project code. Back up data before making local changes.

## Remove one project only

Replace `acme` with the registered name:

```bash
locdev down acme
locdev remove acme
```

`remove` unregisters the project; it does not delete its files. If that is all you need, keep the app and any DNS or certificates used by other projects.

## Prepare complete removal

Keep the CLI available and the daemon running during preparation: DNS and TLS commands query its state. Close the desktop so it cannot reopen the daemon later. Stop active projects with `locdev down` and their respective names.

If startup at login is enabled, remove it before deleting executables:

```bash
locdev startup status
locdev startup uninstall
```

This removes the login mechanism, not the binaries. See [Startup at login](/docs/service) for platform details.

## Undo DNS and TLS trust

These changes affect the user's LocDev environment, including other local projects. Use them only when you intend to remove that system integration. Review proposed plans and authorization requests; do not use automatic confirmation.

While the daemon is still available:

```bash
locdev dns uninstall
locdev tls untrust
```

The first operation undoes `.test` resolver integration; the second removes local CA trust from the stores handled by the app. Neither deletes project directories. If either fails, inspect the state and follow [Troubleshooting](/docs/troubleshooting) before continuing.

## Stop and remove binaries

After preparation:

```bash
locdev stop
```

This stops the daemon and remaining projects. Use the uninstaller or package manager matching your original installation. For a manually extracted CLI, remove only `locdev` and `locdevd` from the chosen directory and review its `PATH` entry.

## Decide what data to keep

Configuration, project registration, logs, managed runtimes and CA material may remain separate from the binaries. Check actual directories in [Configuration](/docs/configuration), including a custom `LOCDEV_HOME`. Back up and inspect each directory before manual deletion. Do not delete project code, external PHP/Node installations or shared directories.

Reopen the browser after removing TLS trust and check system startup preferences. To reinstall, follow [Installation](/docs/installation); do not assume data from another version is compatible.

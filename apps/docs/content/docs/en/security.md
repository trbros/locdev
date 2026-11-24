# Security

LocDev touches DNS, certificates and processes, so it was designed to need little trust.

## Privileges

- The daemon, the CLI and the app run as your user.
- System changes (resolver, trust store) only happen with a **visible**, **confirmed** script, run by the OS's authorization mechanism.
- Everything LocDev writes to the system carries the `Managed by LocDev` mark, and the uninstall commands only remove what has that mark.
- Other tools' configuration is detected and shown, never edited.

## Network

- The proxy, HTTPS and DNS listen only on `127.0.0.1` (and `::1`), never on the network.
- The DNS server only answers for the `.test` zone, refuses the others and ignores packets that are already responses, avoiding reflection loops.
- IPC uses a socket with `0600` permissions in a `0700` directory owned by your user (Unix) or a named pipe that rejects remote clients (Windows).

## Certificates

- The CA key is kept at `0600` in the user's data folder.
- The CA has **Name Constraints** (`test`, `localhost`) and `pathlen:0`: it can't issue valid certificates for other domains or intermediate CAs.
- Trusting the CA is optional; without it HTTPS works, but browsers warn.

## Processes

- Sites run as your user, in the project directory.
- Stopping a site ends the whole process tree, so no orphans are left.
- Removing a site never deletes files.

## Git and GitHub

- Destructive Git operations (reset, discard, _force push_…) show what will be lost and need a typed phrase. Optionally, they also ask for a password (Argon2id hash, progressive lockout). Before running, LocDev keeps a backup. The password only protects what LocDev starts, not the repository. See [Git, GitHub and protection](/docs/git).
- The GitHub token lives only in the system keyring, never in a file or log. The daemon only talks to `api.github.com`, and every remote action asks for confirmation.
- `git` runs without a shell, with separate arguments and validated names, and never asks for credentials interactively.

## Secrets and credentials

- Secret variables are encrypted (ChaCha20-Poly1305) with the key in the system keyring, and only go into the processes' environment, never into files, arguments or logs ([Environment variables](/docs/environment)).
- Database credentials are read from the project by the daemon on every use and don't go through IPC; a remote database only opens with an explicit choice, and destructive SQL needs the database name ([Database Manager](/docs/database)).

## Email

- The Mailbox only captures, on loopback, and never delivers messages. Replacing a project's real SMTP needs your consent ([Mailbox](/docs/mailbox)).
- Email HTML is shown in an `iframe` with an empty `sandbox` and a policy that blocks all network access.
- Project code only runs when you ask (framework preview, `make:mail`), in a separate process with a clean environment and limited time and output ([Email Studio](/docs/email-studio)).

## Reporting vulnerabilities

Please report security problems privately to the maintainers before disclosing them publicly.

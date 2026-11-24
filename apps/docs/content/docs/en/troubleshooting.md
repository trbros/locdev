# Troubleshooting

Choose the next action based on an observed failure. See the [Doctor reference](/docs/doctor) for an explanation of individual checks. Commands use `acme` as an example; replace it with a registered project name. This website does not execute them.

## Start with evidence

```bash
locdev version
locdev status
locdev doctor
locdev list
```

Record the version, operating system, architecture and first failed check. Identify the failing layer before reinstalling the entire environment.

## The daemon is unreachable

```bash
locdev start
locdev status
```

If it remains unavailable, check that `locdev` and `locdevd` belong to the same installation. Inspect daemon logs under the data directory in [Configuration](/docs/configuration). Follow [Updating](/docs/updating) for incompatible versions. Starting the daemon can also start projects with `auto_start` enabled.

## A project fails to start

```bash
locdev logs acme
locdev domain acme
```

Check the registered directory, effective command, runtime and project dependencies. Install dependencies using the project's package manager and lockfile, outside this website. After fixing the cause, restart only that project:

```bash
locdev reload acme
```

## Ports and the proxy

Doctor distinguishes project server ports from proxy ports. If 80/443 are occupied, the default configuration allows fallback to 7080/7443. Use the port actually reported by the app; a URL without an explicit port may reach another server.

Review [Configuration](/docs/configuration) before changing ports. Stop another server only when you know who owns it; do not terminate unknown processes or apply privileges automatically.

## A .test domain does not resolve

```bash
locdev dns status
```

If another tool owns resolver configuration, investigate the conflict first. Only when you intend to route `.test` through LocDev, run and review the proposed changes:

```bash
locdev dns install
```

See [Domains](/docs/domains) for `.localhost` and platform-specific details.

## HTTPS reports a certificate warning

```bash
locdev tls status
```

Check the local CA and the certificate store used by your browser. If the expected CA is not trusted, review [HTTPS](/docs/https) and the changes proposed by:

```bash
locdev tls trust
```

Reopen the browser afterwards. Do not ignore a certificate that differs from the expected one.

## The development server rejects the host

In the processes LocDev starts, Vite already gets the site's names through the `__VITE_ADDITIONAL_SERVER_ALLOWED_HOSTS` variable (see [Dev servers](/docs/servers)). If Vite runs outside LocDev, add a specific domain you control, such as `acme.test`, to `server.allowedHosts`. Do not use `true` to accept every host. Check the [official Vite options](https://vite.dev/config/server-options.html#server-allowedhosts) and adapt the configuration to the installed framework.

## When to ask for help

Use [Support](/support) with reproduction steps and logs stripped of passwords, tokens, `.env` files and personal data. Report potential vulnerabilities through the [private channel](/security), not a public issue.

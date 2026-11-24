import type { AppUpdate, AppUpdateProgress, ReleaseInfo, UpdateRequest, UpdateResponse } from "./protocol";

/** Bumps the minor version: the release the mock pretends is out. */
function next(version: string) {
  const [major, minor] = version.split(".").map(Number);
  return `${major}.${minor + 1}.0`;
}

function previous(version: string) {
  const [major, minor] = version.split(".").map(Number);
  return `${major}.${Math.max(minor - 1, 0)}.0`;
}

const appNext = next(__APP_VERSION__);
const daemonNext = next(__DAEMON_VERSION__);
const releases: ReleaseInfo[] = [
  {
    tag: `v${appNext}`,
    app: appNext,
    daemon: daemonNext,
    published_at: "2026-11-04T12:00:00Z",
    url: `https://github.com/trbros/locdev/releases/tag/v${appNext}`,
    notes:
      "### Added\n\n- Updates: the app updates itself from signed releases.\n- `locdev update` checks for a newer daemon and CLI.",
    prerelease: false,
    updater: true,
  },
  {
    tag: `v${__APP_VERSION__}`,
    app: __APP_VERSION__,
    daemon: __DAEMON_VERSION__,
    published_at: "2026-11-02T12:00:00Z",
    url: `https://github.com/trbros/locdev/releases/tag/v${__APP_VERSION__}`,
    notes: "",
    prerelease: false,
    updater: true,
  },
  {
    tag: `v${previous(__APP_VERSION__)}`,
    app: previous(__APP_VERSION__),
    daemon: previous(__DAEMON_VERSION__),
    published_at: "2026-10-30T12:00:00Z",
    url: `https://github.com/trbros/locdev/releases/tag/v${previous(__APP_VERSION__)}`,
    notes: "",
    prerelease: false,
    updater: true,
  },
];

let checkedAt = Date.now() - 2 * 3_600_000;

export function handleUpdates(req: UpdateRequest): UpdateResponse {
  if (req.refresh) checkedAt = Date.now();
  return {
    type: "updates",
    repo: "trbros/locdev",
    daemon: __DAEMON_VERSION__,
    latest: releases[0],
    releases,
    daemon_update: true,
    checked_at: checkedAt,
    error: null,
  };
}

let installed: string | null = null;

export function mockAppUpdate(tag?: string | null): AppUpdate {
  const target = tag ? releases.find((r) => r.tag === tag) : releases[0];
  const version = installed ? null : (target?.app ?? null);
  return {
    current: __APP_VERSION__,
    bundled_daemon: __DAEMON_VERSION__,
    version: version === __APP_VERSION__ ? null : version,
    date: target?.published_at ?? null,
    notes: target?.notes || null,
    downgrade: !!tag && target !== releases[0],
    can_install: true,
    reason: null,
    page: target?.url ?? "https://github.com/trbros/locdev/releases/latest",
  };
}

/** Pretends to download 18 MB in a couple of seconds. */
export async function mockAppInstall(tag: string | null | undefined, progress: (p: AppUpdateProgress) => void) {
  const total = 18_400_000;
  for (let downloaded = 0; downloaded < total; downloaded += 1_840_000) {
    progress({ downloaded, total, done: false });
    await new Promise((r) => setTimeout(r, 180));
  }
  progress({ downloaded: 0, total: null, done: true });
  const target = tag ? releases.find((r) => r.tag === tag) : releases[0];
  installed = target?.app ?? null;
  return installed ?? "";
}

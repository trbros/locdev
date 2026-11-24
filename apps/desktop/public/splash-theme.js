// Runs before the splash paints: applies the theme the app saved (next-themes
// keeps it in localStorage under "theme"; dark when unset, like the app) and
// the motion preference from Appearance.
try {
  const theme = localStorage.getItem("theme");
  const dark = theme === "system" ? matchMedia("(prefers-color-scheme: dark)").matches : theme !== "light";
  document.documentElement.classList.toggle("dark", dark);
  const motion = JSON.parse(localStorage.getItem("locdev.prefs") ?? "{}").motion;
  if (motion) document.documentElement.dataset.motion = motion;
} catch {
  // Storage unavailable: keep the defaults (dark, system motion).
}

// The splash's only script. The shell calls these (through `eval`) as each
// boot step really settles; nothing here advances on a timer.
//   splash.stage("proxy", "ok" | "warn" | "fail", "Proxy on :80")
//   splash.done()   – everything is in; play the exit before the swap
//   splash.failed("Daemon didn't start")
window.splash = {
  stage(id, state, text) {
    const node = document.querySelector(`[data-stage="${id}"]`);
    if (node) node.dataset.state = state;
    if (text) this.status(text);
  },
  status(text) {
    const el = document.getElementById("status");
    if (el) el.textContent = text;
  },
  done() {
    document.documentElement.dataset.state = "ready";
  },
  failed(text) {
    document.documentElement.dataset.state = "failed";
    this.status(text);
  },
};

// Calls the shell made before this script loaded.
for (const [method, ...args] of window.__splashQueue ?? []) window.splash[method](...args);
window.__splashQueue = [];

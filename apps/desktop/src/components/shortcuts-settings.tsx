import { Lock, Plus, RotateCcw, Search, TriangleAlert, X } from "lucide-react";
import { useEffect, useState } from "react";

import { Section } from "@/components/page";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Kbd } from "@/components/ui/kbd";
import {
  CATEGORIES,
  comboOf,
  keysOf,
  reservedReason,
  SHORTCUTS,
  type ShortcutDef,
  usable,
  useShortcuts,
} from "@/lib/shortcuts";
import { cn } from "@/lib/utils";
import { t } from "@/lib/i18n";

function Keys({ combo }: { combo: string }) {
  return (
    <span className="inline-flex items-center gap-0.5">
      {keysOf(combo).map((key, i) => (
        <Kbd key={i} className="min-w-5 justify-center text-[11px]">
          {key}
        </Kbd>
      ))}
    </span>
  );
}

/**
 * Listens for the next combination while active (nothing else fires
 * meanwhile). Esc cancels; a combination needs Ctrl/⌘/Alt (or is a
 * function key), so plain typing can't become a shortcut.
 */
function KeyCapture({ onCapture, onCancel }: { onCapture: (combo: string) => void; onCancel: () => void }) {
  const { setCapturing } = useShortcuts();
  const [hint, setHint] = useState<string | null>(null);

  useEffect(() => {
    setCapturing(true);
    const onKey = (e: KeyboardEvent) => {
      e.preventDefault();
      e.stopPropagation();
      if (e.key === "Escape" && !e.ctrlKey && !e.metaKey && !e.altKey && !e.shiftKey) return onCancel();
      const combo = comboOf(e);
      if (!combo) return;
      if (!usable(combo)) {
        setHint(t("Add Ctrl, ⌘ or Alt, so typing never triggers it."));
        return;
      }
      onCapture(combo);
    };
    // Capture phase on window: first in line, and stops the app's listener.
    window.addEventListener("keydown", onKey, true);
    return () => {
      window.removeEventListener("keydown", onKey, true);
      setCapturing(false);
    };
  }, [onCapture, onCancel, setCapturing]);

  return (
    <span className="inline-flex items-center gap-2">
      <span
        role="status"
        className="animate-pulse rounded-md border border-ember/60 bg-ember/6 px-2 py-0.5 text-[12px] text-ember"
      >
        {t("Press the keys…")}
      </span>
      <span className="text-[11.5px] text-muted-foreground">{hint ?? t("Esc cancels")}</span>
    </span>
  );
}

function Row({ def }: { def: ShortcutDef }) {
  const { combos, changed, set, reset, usedBy } = useShortcuts();
  const [capturing, setCapturing] = useState(false);
  const [pending, setPending] = useState<string | null>(null);
  const current = combos(def.id);
  const add = (combo: string) => set(def.id, [...current, combo]);
  const others = pending ? usedBy(pending, def.id) : [];

  const onCapture = (combo: string) => {
    setCapturing(false);
    if (usedBy(combo, def.id).length) setPending(combo);
    else add(combo);
  };

  return (
    <li className="py-2">
      <div className="flex items-center gap-3">
        <span className="min-w-0 flex-1">
          <span className="text-sm">{def.label}</span>
          {def.where && (
            <span className="ml-2 text-[11.5px] text-muted-foreground">{t("on {page}", { page: def.where })}</span>
          )}
        </span>
        <span className="flex flex-wrap items-center justify-end gap-1.5">
          {current.length === 0 && !capturing && (
            <span className="text-[12px] text-muted-foreground">{t("Not set")}</span>
          )}
          {current.map((combo) => {
            const conflict = usedBy(combo, def.id).length > 0;
            const reserved = reservedReason(combo);
            return (
              <span
                key={combo}
                className={cn(
                  "group inline-flex items-center gap-1 rounded-md px-1 py-0.5",
                  (conflict || (reserved && !def.fixed)) && "bg-amber/10",
                )}
                title={
                  conflict
                    ? t("Also used by: {names}", {
                        names: usedBy(combo, def.id)
                          .map((d) => d.label)
                          .join(", "),
                      })
                    : !def.fixed
                      ? (reserved ?? undefined)
                      : undefined
                }
              >
                {(conflict || (reserved && !def.fixed)) && <TriangleAlert className="size-3 text-amber" />}
                <Keys combo={combo} />
                {!def.fixed && (
                  <button
                    type="button"
                    aria-label={t("Remove {keys}", { keys: keysOf(combo).join(" ") })}
                    onClick={() =>
                      set(
                        def.id,
                        current.filter((c) => c !== combo),
                      )
                    }
                    className="text-muted-foreground opacity-0 transition-opacity group-hover:opacity-100 hover:text-foreground focus-visible:opacity-100"
                  >
                    <X className="size-3" />
                  </button>
                )}
              </span>
            );
          })}
          {def.fixed ? (
            <span title={t("Built in")} className="text-muted-foreground">
              <Lock className="size-3.5" />
            </span>
          ) : capturing ? (
            <KeyCapture onCapture={onCapture} onCancel={() => setCapturing(false)} />
          ) : (
            <Button
              size="icon-xs"
              variant="ghost"
              aria-label={t("Add a shortcut for {action}", { action: def.label })}
              onClick={() => {
                setPending(null);
                setCapturing(true);
              }}
            >
              <Plus />
            </Button>
          )}
          {changed(def.id) && !def.fixed && (
            <Button size="icon-xs" variant="ghost" aria-label={t("Restore the default")} onClick={() => reset(def.id)}>
              <RotateCcw />
            </Button>
          )}
        </span>
      </div>
      {pending && (
        <div className="mt-2 flex flex-wrap items-center gap-2 rounded-lg border border-amber/40 bg-amber/6 px-3 py-2 text-[13px]">
          <Keys combo={pending} />
          <span>{t("is already used by {names}.", { names: others.map((d) => d.label).join(", ") })}</span>
          <span className="ml-auto flex gap-1.5">
            <Button
              size="xs"
              disabled={others.some((d) => d.fixed)}
              onClick={() => {
                for (const other of others)
                  set(
                    other.id,
                    combos(other.id).filter((c) => c !== pending),
                  );
                add(pending);
                setPending(null);
              }}
            >
              {t("Use it here")}
            </Button>
            <Button size="xs" variant="ghost" onClick={() => setPending(null)}>
              {t("Cancel")}
            </Button>
          </span>
        </div>
      )}
      {current.some((c) => !def.fixed && reservedReason(c)) && (
        <p className="mt-1 text-[12px] text-muted-foreground">
          {current
            .map((c) => reservedReason(c))
            .filter(Boolean)
            .join(" ")}
        </p>
      )}
    </li>
  );
}

export function ShortcutsSettings() {
  const { reset, changed, combos } = useShortcuts();
  const [query, setQuery] = useState("");
  const needle = query.trim().toLowerCase();
  const matches = (d: ShortcutDef) =>
    !needle ||
    d.label.toLowerCase().includes(needle) ||
    combos(d.id).some((c) => keysOf(c).join(" ").toLowerCase().includes(needle));
  const anyChanged = SHORTCUTS.some((d) => changed(d.id));

  return (
    <>
      <div className="flex items-center gap-2">
        <div className="relative flex-1">
          <Search className="absolute top-1/2 left-2.5 size-3.5 -translate-y-1/2 text-muted-foreground" />
          <Input
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            onKeyDown={(e) => e.key === "Escape" && setQuery("")}
            placeholder={t("Search shortcuts")}
            aria-label={t("Search shortcuts")}
            className="pl-8"
          />
        </div>
        <Button variant="outline" size="sm" disabled={!anyChanged} onClick={() => reset()}>
          <RotateCcw /> {t("Restore all defaults")}
        </Button>
      </div>
      <p className="text-[13px] text-muted-foreground">
        {t(
          "These work while a LocDev window is in front. LocDev registers no system-wide shortcuts, so it never takes keys from other apps. Changes are saved on this computer and followed by every LocDev window.",
        )}
      </p>
      {CATEGORIES.map((category) => {
        const list = SHORTCUTS.filter((d) => d.category === category.id && matches(d));
        if (!list.length) return null;
        return (
          <Section key={category.id} title={category.label}>
            <ul className="divide-y">
              {list.map((def) => (
                <Row key={def.id} def={def} />
              ))}
            </ul>
          </Section>
        );
      })}
      {CATEGORIES.every((c) => !SHORTCUTS.some((d) => d.category === c.id && matches(d))) && (
        <p className="text-sm text-muted-foreground">{t("No shortcut matches.")}</p>
      )}
    </>
  );
}

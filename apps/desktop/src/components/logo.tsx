import { cn } from "@/lib/utils";

/**
 * The LocDev logo, used as it is. Source of truth: branding/logo.png;
 * `pnpm icons` writes the 256px copy in public/ (shared with the splash).
 */
export function LogoMark({ className }: { className?: string }) {
  return (
    <img
      src={`${import.meta.env.BASE_URL}logo.png`}
      alt=""
      aria-hidden
      draggable={false}
      className={cn("size-8 object-contain select-none", className)}
    />
  );
}

import { type ReactNode, useId } from "react";
import { Switch } from "@/components/ui/switch";
import { cn } from "@/lib/utils";

/**
 * A switch with its text. No `<label>`: wrapping base-ui's switch (a span plus
 * a hidden input) in a label left each click to the web engine's label
 * activation, which toggled the wrong switch or twice on some engines. Here the
 * switch toggles itself, and a click on the text toggles it once through
 * `onCheckedChange`; the text names the switch for assistive technology.
 */
export function SwitchField({
  checked,
  onCheckedChange,
  disabled,
  className,
  switchClassName,
  children,
}: {
  checked: boolean;
  onCheckedChange: (checked: boolean) => void;
  disabled?: boolean;
  className?: string;
  switchClassName?: string;
  children: ReactNode;
}) {
  const id = useId();
  return (
    <div className={className}>
      <Switch
        aria-labelledby={id}
        checked={checked}
        disabled={disabled}
        onCheckedChange={(value) => onCheckedChange(value)}
        className={switchClassName}
      />
      {/* biome-ignore lint/a11y/noStaticElementInteractions: the switch itself is the keyboard control; this only widens the pointer target */}
      {/* biome-ignore lint/a11y/useKeyWithClickEvents: as above */}
      <span
        id={id}
        className={cn("min-w-0 select-none", disabled ? "cursor-default" : "cursor-pointer")}
        onClick={() => {
          if (!disabled) onCheckedChange(!checked);
        }}
      >
        {children}
      </span>
    </div>
  );
}

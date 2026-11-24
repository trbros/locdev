import { useTheme } from "next-themes";
import type { SVGProps } from "react";
import { stackNames, stacks, stacksDark } from "./generated";

type Props = SVGProps<SVGSVGElement> & {
  id: string;
  variant?: "light" | "dark" | "auto";
};

/** Um único componente para todos os ids retornados pelo daemon. */
export function StackIcon({ id, variant = "auto", ...props }: Props) {
  const { resolvedTheme } = useTheme();
  const dark = variant === "dark" || (variant === "auto" && resolvedTheme === "dark");
  const Icon = dark ? (stacksDark[id] ?? stacks[id]) : stacks[id];
  const Fallback = stacks.unknown;
  const Component = Icon ?? Fallback;
  return <Component role="img" aria-label={props["aria-label"] ?? stackNames[id] ?? id} {...props} />;
}

export { stackNames, stacks, stacksDark } from "./generated";

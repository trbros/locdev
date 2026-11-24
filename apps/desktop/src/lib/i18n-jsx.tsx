import { Fragment, type ReactNode } from "react";
import { t } from "@/lib/i18n";

/** `t` for a sentence with elements inside (a code chip, a link): the whole
 *  sentence is translated, so each language can put them where it needs to.
 *  `tx("Type {phrase} to confirm", { phrase: <code>…</code> })`. */
export function tx(text: string, vars: Record<string, ReactNode>): ReactNode {
  return t(text)
    .split(/(\{\w+\})/)
    .map((part, i) => {
      const name = /^\{(\w+)\}$/.exec(part)?.[1];
      return <Fragment key={i}>{name && name in vars ? vars[name] : part}</Fragment>;
    });
}

import { autocompletion, closeBrackets, closeBracketsKeymap, completionKeymap } from "@codemirror/autocomplete";
import { defaultKeymap, history, historyKeymap, indentWithTab } from "@codemirror/commands";
import { css } from "@codemirror/lang-css";
import { html } from "@codemirror/lang-html";
import { javascript } from "@codemirror/lang-javascript";
import { json } from "@codemirror/lang-json";
import { markdown } from "@codemirror/lang-markdown";
import { php } from "@codemirror/lang-php";
import { PostgreSQL, sql } from "@codemirror/lang-sql";
import { xml } from "@codemirror/lang-xml";
import { yaml } from "@codemirror/lang-yaml";
import {
  bracketMatching,
  HighlightStyle,
  indentOnInput,
  indentUnit,
  StreamLanguage,
  syntaxHighlighting,
} from "@codemirror/language";
import { json as jsonc } from "@codemirror/legacy-modes/mode/javascript";
import { powerShell } from "@codemirror/legacy-modes/mode/powershell";
import { properties } from "@codemirror/legacy-modes/mode/properties";
import { python } from "@codemirror/legacy-modes/mode/python";
import { ruby } from "@codemirror/legacy-modes/mode/ruby";
import { shell } from "@codemirror/legacy-modes/mode/shell";
import { toml } from "@codemirror/legacy-modes/mode/toml";
import { highlightSelectionMatches, searchKeymap } from "@codemirror/search";
import { Compartment, EditorState, type Extension, RangeSetBuilder } from "@codemirror/state";
import {
  Decoration,
  type DecorationSet,
  drawSelection,
  EditorView,
  highlightActiveLine,
  highlightActiveLineGutter,
  highlightSpecialChars,
  keymap,
  lineNumbers,
  MatchDecorator,
  placeholder as placeholderExt,
  ViewPlugin,
  type ViewUpdate,
} from "@codemirror/view";
import { tags as t } from "@lezer/highlight";
import { forwardRef, useEffect, useImperativeHandle, useRef } from "react";

import { cn } from "@/lib/utils";

export type CodeLanguage =
  | "json"
  | "jsonc"
  | "javascript"
  | "typescript"
  | "jsx"
  | "tsx"
  | "yaml"
  | "sql"
  | "html"
  | "blade"
  | "php"
  | "css"
  | "markdown"
  | "xml"
  | "shell"
  | "powershell"
  | "python"
  | "ruby"
  | "dotenv"
  | "toml"
  | "text";

/** The language for a file, from its name (config files, templates, workflows). */
export function languageForPath(path: string): CodeLanguage {
  const name = path.toLowerCase().split(/[\\/]/).pop() ?? "";
  if (name.endsWith(".blade.php")) return "blade";
  if (name.startsWith(".env")) return "dotenv";
  if (name === ".babelrc" || /^\.prettierrc(\.json)?$/.test(name) || name === ".eslintrc") return "jsonc";
  if (/^(tsconfig|jsconfig)(\..+)?\.json$/.test(name) || /\.(jsonc|json5)$/.test(name)) return "jsonc";
  if (name === "biome.json" || name === "deno.json") return "jsonc";
  const ext = name.includes(".") ? name.slice(name.lastIndexOf(".") + 1) : "";
  switch (ext) {
    case "json":
      return "json";
    case "js":
    case "mjs":
    case "cjs":
      return "javascript";
    case "ts":
    case "mts":
    case "cts":
      return "typescript";
    case "jsx":
      return "jsx";
    case "tsx":
      return "tsx";
    case "yml":
    case "yaml":
      return "yaml";
    case "sql":
      return "sql";
    case "html":
    case "htm":
    case "mjml":
      return "html";
    case "php":
      return "php";
    case "css":
    case "scss":
    case "pcss":
      return "css";
    case "md":
    case "markdown":
      return "markdown";
    case "xml":
    case "dist":
    case "svg":
      return "xml";
    case "sh":
    case "bash":
    case "zsh":
      return "shell";
    case "ps1":
      return "powershell";
    case "py":
      return "python";
    case "rb":
    case "erb":
      return "ruby";
    case "toml":
      return "toml";
    case "ini":
    case "properties":
    case "conf":
      return "dotenv";
    default:
      return "text";
  }
}

function language(lang: CodeLanguage): Extension {
  switch (lang) {
    case "json":
      return json();
    case "jsonc":
      // JSON with comments and trailing commas; the strict parser marks those as errors.
      return StreamLanguage.define(jsonc);
    case "javascript":
      return javascript();
    case "typescript":
      return javascript({ typescript: true });
    case "jsx":
      return javascript({ jsx: true });
    case "tsx":
      return javascript({ jsx: true, typescript: true });
    case "yaml":
      return yaml();
    case "sql":
      return sql({ dialect: PostgreSQL, upperCaseKeywords: false });
    case "html":
      return html();
    case "blade":
      return [html(), blade];
    case "php":
      return php();
    case "css":
      return css();
    case "markdown":
      return markdown();
    case "xml":
      return xml();
    case "shell":
      return StreamLanguage.define(shell);
    case "powershell":
      return StreamLanguage.define(powerShell);
    case "python":
      return StreamLanguage.define(python);
    case "ruby":
      return StreamLanguage.define(ruby);
    case "dotenv":
      return StreamLanguage.define(properties);
    case "toml":
      return StreamLanguage.define(toml);
    default:
      return [];
  }
}

/**
 * Blade on top of HTML: `{{ }}`, `{!! !!}`, `{{-- --}}` and `@directives`.
 * Marked by decoration, so the HTML around them keeps its own highlighting.
 */
const bladeMatcher = new MatchDecorator({
  regexp: /\{\{--[\s\S]*?--\}\}|\{!![\s\S]*?!!\}|\{\{[\s\S]*?\}\}|@[a-zA-Z]+\b/g,
  decoration: (m) =>
    Decoration.mark({
      class: m[0].startsWith("{{--")
        ? "cm-blade-comment"
        : m[0].startsWith("@")
          ? "cm-blade-directive"
          : "cm-blade-echo",
    }),
});
const blade = ViewPlugin.fromClass(
  class {
    decorations: DecorationSet;
    constructor(view: EditorView) {
      this.decorations = bladeMatcher.createDeco(view);
    }
    update(update: ViewUpdate) {
      this.decorations = bladeMatcher.updateDeco(update, this.decorations);
    }
  },
  { decorations: (v) => v.decorations },
);

const highlight = HighlightStyle.define([
  {
    tag: [t.keyword, t.operatorKeyword, t.modifier, t.controlKeyword, t.definitionKeyword],
    color: "var(--syntax-keyword)",
  },
  { tag: [t.string, t.special(t.string), t.regexp, t.inserted], color: "var(--syntax-string)" },
  { tag: [t.number, t.bool, t.null, t.atom, t.unit], color: "var(--syntax-number)" },
  { tag: [t.propertyName, t.attributeName, t.labelName], color: "var(--syntax-property)" },
  { tag: [t.function(t.variableName), t.function(t.propertyName), t.macroName], color: "var(--syntax-function)" },
  { tag: [t.typeName, t.className, t.namespace, t.self], color: "var(--syntax-type)" },
  { tag: [t.tagName, t.heading], color: "var(--syntax-keyword)", fontWeight: "500" },
  {
    tag: [t.comment, t.lineComment, t.blockComment, t.docComment, t.meta],
    color: "var(--syntax-comment)",
    fontStyle: "italic",
  },
  { tag: [t.deleted, t.invalid], color: "var(--destructive)" },
  { tag: [t.link, t.url], color: "var(--syntax-property)", textDecoration: "underline" },
  { tag: t.strong, fontWeight: "600" },
  { tag: t.emphasis, fontStyle: "italic" },
  { tag: [t.variableName, t.punctuation, t.bracket, t.separator, t.operator], color: "inherit" },
]);

function readOnlyExt(readOnly: boolean) {
  return [EditorState.readOnly.of(readOnly), EditorView.editable.of(!readOnly)];
}

const theme = EditorView.theme({
  "&": {
    color: "var(--foreground)",
    backgroundColor: "transparent",
    fontSize: "12.5px",
    height: "100%",
  },
  "&.cm-focused": { outline: "none" },
  ".cm-scroller": {
    fontFamily: "var(--font-mono)",
    lineHeight: "1.6",
  },
  ".cm-content": { padding: "8px 0", caretColor: "var(--ember)" },
  ".cm-line": { padding: "0 12px" },
  ".cm-cursor, .cm-dropCursor": { borderLeftColor: "var(--ember)", borderLeftWidth: "2px" },
  "&.cm-focused > .cm-scroller > .cm-selectionLayer .cm-selectionBackground, .cm-selectionBackground, ::selection": {
    backgroundColor: "color-mix(in oklab, var(--ember) 24%, transparent) !important",
  },
  ".cm-gutters": {
    // Opaque: long lines scroll under it.
    backgroundColor: "var(--well)",
    color: "color-mix(in oklab, var(--muted-foreground) 60%, transparent)",
    border: "none",
  },
  ".cm-lineNumbers .cm-gutterElement": { padding: "0 8px 0 12px", minWidth: "36px" },
  ".cm-activeLine": { backgroundColor: "color-mix(in oklab, var(--foreground) 3.5%, transparent)" },
  ".cm-activeLineGutter": { backgroundColor: "transparent", color: "var(--foreground)" },
  ".cm-matchingBracket": {
    backgroundColor: "color-mix(in oklab, var(--ember) 18%, transparent)",
    outline: "1px solid color-mix(in oklab, var(--ember) 45%, transparent)",
  },
  ".cm-selectionMatch": { backgroundColor: "color-mix(in oklab, var(--signal) 18%, transparent)" },
  ".cm-searchMatch": { backgroundColor: "color-mix(in oklab, var(--amber) 30%, transparent)" },
  ".cm-placeholder": { color: "var(--muted-foreground)" },
  ".cm-panels": { backgroundColor: "var(--popover)", color: "var(--foreground)", borderColor: "var(--border)" },
  ".cm-panels input, .cm-panels button": { fontSize: "12px" },
  ".cm-tooltip": {
    backgroundColor: "var(--popover)",
    color: "var(--popover-foreground)",
    border: "1px solid var(--border)",
    borderRadius: "6px",
  },
  ".cm-tooltip-autocomplete > ul > li[aria-selected]": {
    backgroundColor: "color-mix(in oklab, var(--ember) 22%, transparent)",
    color: "var(--foreground)",
  },
  ".cm-blade-echo": { color: "var(--syntax-function)" },
  ".cm-blade-directive": { color: "var(--syntax-keyword)", fontWeight: "500" },
  ".cm-blade-comment, .cm-blade-comment *": { color: "var(--syntax-comment) !important", fontStyle: "italic" },
  ".cm-line-error": { backgroundColor: "color-mix(in oklab, var(--destructive) 14%, transparent)" },
  ".cm-line-warning": { backgroundColor: "color-mix(in oklab, var(--amber) 14%, transparent)" },
});

const errorLine = Decoration.line({ class: "cm-line-error" });
const warningLine = Decoration.line({ class: "cm-line-warning" });

function lineMarks(marks: Map<number, string> | undefined): Extension {
  if (!marks?.size) return [];
  return EditorView.decorations.compute(["doc"], (state) => {
    const builder = new RangeSetBuilder<Decoration>();
    for (const n of [...marks.keys()].sort((a, b) => a - b)) {
      const level = marks.get(n);
      if (n < 1 || n > state.doc.lines || (level !== "error" && level !== "warning")) continue;
      builder.add(state.doc.line(n).from, state.doc.line(n).from, level === "error" ? errorLine : warningLine);
    }
    return builder.finish();
  });
}

export type CodeEditorHandle = {
  /** Inserts at the cursor (or over the selection) and focuses the editor. */
  insert: (text: string) => void;
  focus: () => void;
};

export type CodeEditorProps = {
  value: string;
  onChange?: (value: string) => void;
  language?: CodeLanguage;
  readOnly?: boolean;
  lineNumbers?: boolean;
  wrap?: boolean;
  /** Spaces per indent level (YAML and most configs: 2). */
  indent?: number;
  placeholder?: string;
  ariaLabel?: string;
  autoFocus?: boolean;
  /** Ctrl/⌘+Enter. */
  onSubmit?: () => void;
  /** Ctrl/⌘+S. */
  onSave?: () => void;
  /** Lines to tint (1-based): "error" or "warning"; other levels are ignored. */
  marks?: Map<number, string>;
  className?: string;
};

/**
 * The one code editor of the app: CodeMirror with the app's colors, the
 * language's highlighting, undo history, search (Ctrl/⌘+F) and bracket
 * matching. Read-only mode is the code viewer.
 */
export const CodeEditor = forwardRef<CodeEditorHandle, CodeEditorProps>(function CodeEditor(props, ref) {
  const host = useRef<HTMLDivElement>(null);
  const view = useRef<EditorView | null>(null);
  // Latest callbacks, read by the extensions without rebuilding the editor.
  const latest = useRef(props);
  latest.current = props;
  const compartments = useRef({
    language: new Compartment(),
    readOnly: new Compartment(),
    marks: new Compartment(),
    wrap: new Compartment(),
  });

  useImperativeHandle(ref, () => ({
    insert(text) {
      const v = view.current;
      if (!v) return;
      const { from, to } = v.state.selection.main;
      v.dispatch({ changes: { from, to, insert: text }, selection: { anchor: from + text.length } });
      v.focus();
    },
    focus() {
      view.current?.focus();
    },
  }));

  // biome-ignore lint/correctness/useExhaustiveDependencies: built once; the effects below keep it in sync.
  useEffect(() => {
    const c = compartments.current;
    const p = latest.current;
    const shortcuts = keymap.of([
      {
        key: "Mod-Enter",
        run: () => {
          if (!latest.current.onSubmit) return false;
          latest.current.onSubmit();
          return true;
        },
      },
      {
        key: "Mod-s",
        preventDefault: true,
        run: () => {
          latest.current.onSave?.();
          return true;
        },
      },
    ]);
    const state = EditorState.create({
      doc: p.value,
      extensions: [
        shortcuts,
        p.lineNumbers === false ? [] : [lineNumbers(), highlightActiveLineGutter()],
        highlightSpecialChars(),
        history(),
        drawSelection(),
        indentOnInput(),
        bracketMatching(),
        closeBrackets(),
        autocompletion({ activateOnTyping: false }),
        highlightActiveLine(),
        highlightSelectionMatches(),
        indentUnit.of(" ".repeat(p.indent ?? 2)),
        EditorState.tabSize.of(p.indent ?? 2),
        keymap.of([
          ...closeBracketsKeymap,
          ...defaultKeymap,
          ...searchKeymap,
          ...historyKeymap,
          ...completionKeymap,
          indentWithTab,
        ]),
        syntaxHighlighting(highlight),
        theme,
        p.placeholder ? placeholderExt(p.placeholder) : [],
        EditorView.contentAttributes.of({
          "aria-label": p.ariaLabel ?? "Code",
          // Explicit, so the scrolling area counts as keyboard-reachable
          // (and read-only views can still be focused and scrolled).
          tabindex: "0",
          spellcheck: "false",
          autocorrect: "off",
          autocapitalize: "off",
        }),
        c.language.of(language(p.language ?? "text")),
        c.readOnly.of(readOnlyExt(!!p.readOnly)),
        c.marks.of(lineMarks(p.marks)),
        c.wrap.of(p.wrap ? EditorView.lineWrapping : []),
        EditorView.updateListener.of((u) => {
          if (u.docChanged) latest.current.onChange?.(u.state.doc.toString());
        }),
      ],
    });
    const v = new EditorView({ state, parent: host.current! });
    view.current = v;
    if (p.autoFocus) v.focus();
    return () => {
      v.destroy();
      view.current = null;
    };
  }, []);

  // A value from outside (load, discard, reset) replaces the document; our
  // own edits already match it, so typing never moves the cursor.
  useEffect(() => {
    const v = view.current;
    if (!v || v.state.doc.toString() === props.value) return;
    v.dispatch({ changes: { from: 0, to: v.state.doc.length, insert: props.value } });
  }, [props.value]);

  useEffect(() => {
    view.current?.dispatch({ effects: compartments.current.language.reconfigure(language(props.language ?? "text")) });
  }, [props.language]);

  useEffect(() => {
    view.current?.dispatch({
      effects: compartments.current.readOnly.reconfigure(readOnlyExt(!!props.readOnly)),
    });
  }, [props.readOnly]);

  useEffect(() => {
    view.current?.dispatch({ effects: compartments.current.marks.reconfigure(lineMarks(props.marks)) });
  }, [props.marks]);

  useEffect(() => {
    view.current?.dispatch({
      effects: compartments.current.wrap.reconfigure(props.wrap ? EditorView.lineWrapping : []),
    });
  }, [props.wrap]);

  return (
    <div
      ref={host}
      data-selectable
      className={cn(
        "code-editor min-h-0 overflow-hidden rounded-md border bg-well focus-within:border-ember/70 [&_.cm-editor]:h-full",
        props.readOnly && "focus-within:border-border",
        props.className,
      )}
    />
  );
});

/** Read-only code with highlighting, for previews (generated files, raw messages). */
export function CodeView({
  value,
  language,
  className,
  lineNumbers,
  wrap,
  marks,
  ariaLabel,
}: {
  value: string;
  language: CodeLanguage;
  className?: string;
  lineNumbers?: boolean;
  wrap?: boolean;
  marks?: Map<number, string>;
  ariaLabel?: string;
}) {
  return (
    <CodeEditor
      value={value}
      language={language}
      readOnly
      lineNumbers={lineNumbers}
      wrap={wrap}
      marks={marks}
      ariaLabel={ariaLabel}
      className={className}
    />
  );
}

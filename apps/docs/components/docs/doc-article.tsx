import { cloneElement } from "react";
import ReactMarkdown, { defaultUrlTransform } from "react-markdown";
import remarkGfm from "remark-gfm";
import { Link } from "@/i18n/navigation";
import { getDocHeadings } from "@/lib/doc-headings";
import { CodeBlock } from "./code-block";
import {
	CliReference,
	FrameworksTable,
	StartersTable,
} from "./generated-reference";
import { HighlightedCode } from "./highlighted-code";
import { Screenshot } from "./screenshot";

/**
 * Each screenshot is a block-level figure, intentionally kept outside markdown
 * paragraphs to avoid invalid <p><figure> nesting and hydration errors.
 */
const ALERT = /^\[!(NOTE|TIP|WARNING|CAUTION|IMPORTANT)\]\s*/;

/** `> [!WARNING]` blockquotes (GitHub's alert syntax) become callouts. */
function alertOf(children: React.ReactNode): string | null {
	const first = Array.isArray(children)
		? children.find((c) => typeof c === "object" && c !== null)
		: children;
	const paragraph = first as
		| { props?: { children?: React.ReactNode } }
		| undefined;
	const text = paragraph?.props?.children;
	const head = Array.isArray(text) ? text[0] : text;
	if (typeof head !== "string") return null;
	return ALERT.exec(head)?.[1] ?? null;
}

export function DocArticle({
	content,
	locale,
}: {
	content: string;
	locale: string;
}) {
	const headings = getDocHeadings(content);
	let headingIndex = 0;
	// Screenshots and generated references are block-level, so they're split out
	// of the Markdown (a <figure> inside a <p> breaks hydration).
	const segments = content.split(
		/^(?:!\[([^\]]*)\]\(screenshot:([a-z0-9-]+)\)|<!-- generated:(cli|frameworks|starters) -->)\s*$/gm,
	);
	return (
		<article className="doc-article prose-docs" id="doc-content">
			{segments.map((segment, index) => {
				const slot = index % 4;
				if (
					slot === 1 ||
					(slot === 2 && segment === undefined) ||
					(slot === 3 && segment === undefined)
				)
					return null;
				if (slot === 2)
					return (
						<Screenshot
							key={`image-${index}`}
							name={segment}
							alt={segments[index - 1] || "LocDev desktop screenshot"}
						/>
					);
				if (slot === 3)
					return segment === "cli" ? (
						<CliReference key={`gen-${index}`} locale={locale} />
					) : segment === "starters" ? (
						<StartersTable key={`gen-${index}`} locale={locale} />
					) : (
						<FrameworksTable key={`gen-${index}`} locale={locale} />
					);
				return (
					<ReactMarkdown
						key={`markdown-${index}`}
						remarkPlugins={[remarkGfm]}
						urlTransform={defaultUrlTransform}
						components={{
							pre: ({ node, children }) => {
								const element = node?.children[0];
								if (element?.type !== "element") return <pre>{children}</pre>;
								const classNames = element.properties.className;
								const language = Array.isArray(classNames)
									? String(classNames[0] ?? "").replace(/^language-/, "")
									: "text";
								const code = element.children
									.map((child) => (child.type === "text" ? child.value : ""))
									.join("");
								return (
									<CodeBlock code={code} language={language || "text"}>
										<HighlightedCode
											code={code}
											language={language || "text"}
										/>
									</CodeBlock>
								);
							},
							h2: ({ children }) => (
								<h2 id={headings[headingIndex++]?.id}>{children}</h2>
							),
							h3: ({ children }) => (
								<h3 id={headings[headingIndex++]?.id}>{children}</h3>
							),
							a: ({ href, children }) => {
								if (!href) return <span>{children}</span>;
								if (href.startsWith("/") && !href.startsWith("//"))
									return <Link href={href}>{children}</Link>;
								if (/^https?:\/\//.test(href))
									return (
										<a href={href} target="_blank" rel="noopener noreferrer">
											{children}
										</a>
									);
								return <a href={href}>{children}</a>;
							},
							blockquote: ({ children }) => {
								const kind = alertOf(children);
								if (!kind) return <blockquote>{children}</blockquote>;
								return (
									<blockquote
										className={`doc-alert doc-alert-${kind.toLowerCase()}`}
									>
										{stripAlert(children)}
									</blockquote>
								);
							},
							img: ({ src, alt }) => (
								<img src={src ?? ""} alt={alt ?? ""} loading="lazy" />
							),
						}}
					>
						{segment ?? ""}
					</ReactMarkdown>
				);
			})}
		</article>
	);
}

/** The callout without its `[!NOTE]` marker. */
function stripAlert(children: React.ReactNode): React.ReactNode {
	const list = Array.isArray(children) ? [...children] : [children];
	const index = list.findIndex((c) => typeof c === "object" && c !== null);
	const paragraph = list[index] as
		| React.ReactElement<{ children?: React.ReactNode }>
		| undefined;
	if (!paragraph) return children;
	const inner = paragraph.props.children;
	const parts = Array.isArray(inner) ? [...inner] : [inner];
	if (typeof parts[0] === "string") parts[0] = parts[0].replace(ALERT, "");
	// cloneElement, not a spread: React's development build keeps internal
	// fields on elements that a spread drops, and then refuses to render them.
	list[index] = cloneElement(paragraph, undefined, ...parts);
	return list;
}

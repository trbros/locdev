import { readFileSync } from "node:fs";
import { join } from "node:path";
import { localeFileKey } from "@/i18n/locales";

type Arg = {
	name: string;
	help: string;
	required: boolean;
	positional: boolean;
	default: string | null;
};
type Command = {
	usage: string;
	name: string;
	about: string;
	details: string | null;
	aliases: string[];
	args: Arg[];
	since: string;
	privileged: boolean;
	examples: string[];
	notes: string[];
};
type Category = { id: string; label: string; commands: Command[] };
type Framework = {
	id: string;
	name: string;
	category: string;
	integration: string;
	runtimes: string[];
	dev: string[];
	supported: string | null;
	docs: string;
	install: string | null;
	test: string | null;
};

const ui = {
	br: {
		since: "desde",
		admin: "pede administrador",
		examples: "Exemplos",
		aliases: "apelidos",
		options: "Opções",
		name: "Stack",
		kind: "Tipo",
		integration: "Integração",
		runtimes: "Runtimes",
		dev: "Comando de desenvolvimento",
		supported: "Versões",
		starter: "Framework",
		methods: "Criado com",
	},
	en: {
		since: "since",
		admin: "needs administrator",
		examples: "Examples",
		aliases: "aliases",
		options: "Options",
		name: "Stack",
		kind: "Kind",
		integration: "Integration",
		runtimes: "Runtimes",
		dev: "Dev command",
		supported: "Versions",
		starter: "Framework",
		methods: "Created with",
	},
	es: {
		since: "desde",
		admin: "requiere administrador",
		examples: "Ejemplos",
		aliases: "alias",
		options: "Opciones",
		name: "Stack",
		kind: "Tipo",
		integration: "Integración",
		runtimes: "Runtimes",
		dev: "Comando de desarrollo",
		supported: "Versiones",
		starter: "Framework",
		methods: "Creado con",
	},
	ja: {
		since: "追加バージョン",
		admin: "管理者権限が必要",
		examples: "例",
		aliases: "別名",
		options: "オプション",
		name: "スタック",
		kind: "種類",
		integration: "統合",
		runtimes: "ランタイム",
		dev: "開発コマンド",
		supported: "対応バージョン",
		starter: "フレームワーク",
		methods: "作成方法",
	},
} as const;

function read<T>(file: string): T {
	// Written by the app's own tests (`make cli-catalog`); the docs CI fails when they're stale.
	return JSON.parse(
		readFileSync(join(process.cwd(), "generated", file), "utf8"),
	) as T;
}

/** Every `locdev` command, from the CLI's own definitions. */
export function CliReference({ locale }: { locale: string }) {
	const t = ui[localeFileKey(locale) as keyof typeof ui] ?? ui.br;
	const catalog = read<{ version: string; categories: Category[] }>("cli.json");
	return (
		<div className="generated-reference">
			{catalog.categories.map((category) => (
				<section key={category.id}>
					<h2 id={`cli-${category.id}`}>{category.label}</h2>
					{category.commands.map((c) => (
						<div
							key={c.usage}
							className="cli-command"
							id={`cli-${c.name.replaceAll(" ", "-")}`}
						>
							<h3>
								<code>{c.usage}</code>
							</h3>
							<p>
								{c.about}
								<span className="cli-meta">
									{" "}
									· {t.since} {c.since}
									{c.privileged ? ` · ${t.admin}` : ""}
									{c.aliases.length
										? ` · ${t.aliases}: ${c.aliases.join(", ")}`
										: ""}
								</span>
							</p>
							{c.details && <p>{c.details}</p>}
							{c.args.length > 0 && (
								<table>
									<tbody>
										{c.args.map((a) => (
											<tr key={a.name}>
												<td>
													<code>{a.name}</code>
												</td>
												<td>
													{a.help}
													{a.default ? ` (default: ${a.default})` : ""}
												</td>
											</tr>
										))}
									</tbody>
								</table>
							)}
							{c.examples.length > 0 && (
								<pre>
									<code>{c.examples.join("\n")}</code>
								</pre>
							)}
							{c.notes.map((n) => (
								<p key={n} className="cli-note">
									{n}
								</p>
							))}
						</div>
					))}
				</section>
			))}
		</div>
	);
}

/** The framework definitions LocDev ships (crates/locdev-frameworks). */
export function FrameworksTable({ locale }: { locale: string }) {
	const t = ui[localeFileKey(locale) as keyof typeof ui] ?? ui.br;
	const { definitions } = read<{ definitions: Framework[] }>("frameworks.json");
	return (
		<div className="generated-reference table-scroll">
			<table>
				<thead>
					<tr>
						<th>{t.name}</th>
						<th>{t.kind}</th>
						<th>{t.integration}</th>
						<th>{t.runtimes}</th>
						<th>{t.dev}</th>
						<th>{t.supported}</th>
					</tr>
				</thead>
				<tbody>
					{definitions.map((f) => (
						<tr key={f.id}>
							<td>
								<a href={f.docs} target="_blank" rel="noopener noreferrer">
									{f.name}
								</a>
							</td>
							<td>{f.category}</td>
							<td>{f.integration}</td>
							<td>{f.runtimes.join(", ")}</td>
							<td>{f.dev[0] ? <code>{f.dev[0]}</code> : "—"}</td>
							<td>{f.supported ?? "—"}</td>
						</tr>
					))}
				</tbody>
			</table>
		</div>
	);
}

type Starter = {
	id: string;
	name: string;
	docs: string;
	methods: { id: string; label: string }[];
	options: {
		id: string;
		label: string;
		default: string;
		choices: { value: string; label: string }[];
		methods: string[];
	}[];
};

/** The generators `locdev new project` runs (from `locdev-scaffold`). */
export function StartersTable({ locale }: { locale: string }) {
	const t = ui[localeFileKey(locale) as keyof typeof ui] ?? ui.br;
	const starters = read<Starter[]>("starters.json");
	return (
		<div className="generated-reference table-scroll">
			<table>
				<thead>
					<tr>
						<th>{t.starter}</th>
						<th>{t.methods}</th>
						<th>{t.options}</th>
					</tr>
				</thead>
				<tbody>
					{starters.map((s) => (
						<tr key={s.id}>
							<td>
								<a href={s.docs} target="_blank" rel="noopener noreferrer">
									{s.name}
								</a>{" "}
								<code>{s.id}</code>
							</td>
							<td>{s.methods.map((m) => m.label).join(", ")}</td>
							<td>
								{s.options.length === 0
									? "—"
									: s.options.map((o) => (
											<div key={o.id}>
												<code>
													{o.id}=
													{o.choices.length
														? o.choices.map((c) => c.value).join("|")
														: "true|false"}
												</code>
											</div>
										))}
							</td>
						</tr>
					))}
				</tbody>
			</table>
		</div>
	);
}

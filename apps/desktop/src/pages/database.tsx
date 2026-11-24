import { ChevronLeft, ChevronRight, Database, KeyRound, Plus, Play, RefreshCw, Table2, Trash2, X } from "lucide-react";
import { type ReactNode, useMemo, useRef, useState } from "react";
import { toast } from "sonner";

import { CodeEditor, type CodeEditorHandle } from "@/components/code-editor";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Skeleton } from "@/components/ui/skeleton";
import { errorMessage } from "@/hooks/use-daemon";
import {
  type RowsQuery,
  useDbActions,
  useDbConnections,
  useDbRows,
  useDbSchema,
  useDbTableInfo,
  useRedisKeys,
  useRedisValue,
} from "@/hooks/use-db";
import { DaemonError } from "@/lib/errors";
import type { DbConnection, DbFilter, DbRows, DbTable, DbTableInfo, QueryCheck } from "@/lib/protocol";
import { cn } from "@/lib/utils";
import { t } from "@/lib/i18n";
import { tx } from "@/lib/i18n-jsx";

const PAGE = 50;
const OPS: DbFilter["op"][] = ["=", "!=", "<", ">", "<=", ">=", "like", "is_null", "not_null"];
const HISTORY_KEY = "locdev.db.history";

function readHistory(): string[] {
  try {
    return JSON.parse(localStorage.getItem(HISTORY_KEY) ?? "[]");
  } catch {
    return [];
  }
}

function writeHistory(list: string[]) {
  try {
    localStorage.setItem(HISTORY_KEY, JSON.stringify(list.slice(0, 30)));
  } catch {
    // Private windows: history just isn't kept.
  }
}

const isMissing = (e: unknown) =>
  e instanceof DaemonError && e.code === "NOT_FOUND" && e.message.startsWith("missing database");
const isRemote = (e: unknown) => e instanceof DaemonError && e.code === "UNAUTHORIZED";

export function DatabaseView() {
  const connections = useDbConnections();
  const [selected, setSelected] = useState<string | null>(null);
  const [allowed, setAllowed] = useState<Set<string>>(new Set());
  const list = connections.data ?? [];
  const current = list.find((c) => c.id === selected) ?? null;
  const groups = useMemo(() => {
    const map = new Map<string, DbConnection[]>();
    for (const c of list) {
      const key = c.project ? t("site {name}", { name: c.project }) : t("LocDev instances");
      map.set(key, [...(map.get(key) ?? []), c]);
    }
    return [...map.entries()];
  }, [list]);

  return (
    <div className="flex h-full min-h-0">
      <aside className="flex w-64 shrink-0 flex-col gap-3 overflow-y-auto border-r px-3 py-4">
        <div className="flex items-center justify-between px-1">
          <h1 className="text-[15px] font-semibold">{t("Databases")}</h1>
          <Button
            size="icon-sm"
            variant="ghost"
            aria-label={t("Refresh connections")}
            onClick={() => connections.refetch()}
          >
            <RefreshCw />
          </Button>
        </div>
        {connections.isPending ? (
          <Skeleton className="h-40" />
        ) : list.length === 0 ? (
          <p className="px-1 text-[12.5px] text-muted-foreground">
            {t(
              "No databases yet. Sites declare them in .env (DB_*, DATABASE_URL, REDIS_URL); instances come from Services.",
            )}
          </p>
        ) : (
          groups.map(([group, items]) => (
            <div key={group}>
              <p className="px-1 pb-1 font-mono text-[10.5px] tracking-wide text-muted-foreground uppercase">{group}</p>
              {items.map((c) => (
                <button
                  type="button"
                  key={c.id}
                  onClick={() => setSelected(c.id)}
                  className={cn(
                    "flex w-full items-center gap-2 rounded-md px-2 py-1.5 text-left text-[13px] hover:bg-foreground/[0.04]",
                    selected === c.id && "bg-foreground/[0.07]",
                  )}
                >
                  {c.engine === "redis" ? (
                    <KeyRound className="size-3.5 shrink-0" />
                  ) : (
                    <Database className="size-3.5 shrink-0" />
                  )}
                  <span className="truncate">{c.label}</span>
                  {c.remote && <span className="ml-auto font-mono text-[10px] text-amber">{t("remote")}</span>}
                </button>
              ))}
            </div>
          ))
        )}
      </aside>
      <main className="flex min-w-0 flex-1 flex-col">
        {!current ? (
          <div className="m-auto max-w-sm text-center text-sm text-muted-foreground">
            {t("Choose a database. Credentials stay in the daemon; LocDev reads them from the site's .env each time.")}
          </div>
        ) : (
          <ConnectionView
            key={current.id}
            connection={current}
            allowRemote={allowed.has(current.id)}
            onAllowRemote={() => setAllowed((s) => new Set(s).add(current.id))}
          />
        )}
      </main>
    </div>
  );
}

function ConnectionView({
  connection: c,
  allowRemote,
  onAllowRemote,
}: {
  connection: DbConnection;
  allowRemote: boolean;
  onAllowRemote: () => void;
}) {
  if (c.remote && !allowRemote) {
    return (
      <div className="m-auto flex max-w-md flex-col items-start gap-3 rounded-lg border border-amber/40 bg-amber/[0.06] p-4 text-sm">
        <p className="font-medium">{t("{label} is on {host}, not this machine.", { label: c.label, host: c.host })}</p>
        <p className="text-muted-foreground">
          {tx(
            "It came from {source} of {project}, and may be a production database. Edits and SQL run there for real.",
            { source: c.source, project: c.project ?? t("a site") },
          )}
        </p>
        <Button size="sm" variant="outline" onClick={onAllowRemote}>
          {t("Open it anyway")}
        </Button>
      </div>
    );
  }
  return c.engine === "redis" ? (
    <RedisView connection={c} allowRemote={allowRemote} />
  ) : (
    <SqlView connection={c} allowRemote={allowRemote} />
  );
}

function Header({ c, children }: { c: DbConnection; children?: ReactNode }) {
  return (
    <div className="flex items-center gap-3 border-b px-5 py-2.5">
      <span className="font-medium">{c.label}</span>
      <span className="font-mono text-[11.5px] text-muted-foreground">
        {c.host ? `${c.username ?? ""}${c.username ? "@" : ""}${c.host}:${c.port}` : (c.database ?? "")}
        {c.has_password ? t(" · password set") : ""}
      </span>
      {c.remote && (
        <span className="rounded border border-amber/40 px-1 font-mono text-[10.5px] text-amber">{t("remote")}</span>
      )}
      <div className="ml-auto flex items-center gap-1">{children}</div>
    </div>
  );
}

function MissingDatabase({ c, allowRemote, error }: { c: DbConnection; allowRemote: boolean; error: unknown }) {
  const { create } = useDbActions(c.id, allowRemote);
  return (
    <div className="m-6 flex max-w-lg flex-col items-start gap-3 rounded-lg border p-4 text-sm">
      <p className="font-medium">{errorMessage(error).replace(/^missing database: /, "")}</p>
      <p className="text-muted-foreground">
        {tx("The server answers, but the database {database} isn't there. LocDev only creates it when you ask.", {
          database: c.database ? <span className="font-mono">{c.database}</span> : t("file"),
        })}
      </p>
      <Button
        size="sm"
        onClick={() =>
          create.mutate(undefined, {
            onSuccess: () => toast.success(t("Database created")),
            onError: (e) => toast.error(t("Could not create it"), { description: errorMessage(e) }),
          })
        }
        disabled={create.isPending}
      >
        {t("Create")} {c.database ?? t("the database")}
      </Button>
    </div>
  );
}

function SqlView({ connection: c, allowRemote }: { connection: DbConnection; allowRemote: boolean }) {
  const schema = useDbSchema(c.id, allowRemote);
  const [table, setTable] = useState<DbTable | null>(null);
  const [tab, setTab] = useState<"data" | "structure" | "sql">("data");

  if (schema.isError) {
    return (
      <>
        <Header c={c} />
        {isMissing(schema.error) ? (
          <MissingDatabase c={c} allowRemote={allowRemote} error={schema.error} />
        ) : (
          <p className="m-6 text-sm text-destructive">{errorMessage(schema.error)}</p>
        )}
      </>
    );
  }
  const tables = schema.data?.tables ?? [];
  return (
    <>
      <Header c={c}>
        {(["data", "structure", "sql"] as const).map((t) => (
          <button
            type="button"
            key={t}
            role="tab"
            aria-selected={tab === t}
            onClick={() => setTab(t)}
            className={cn(
              "rounded-md px-2.5 py-1 font-mono text-[11.5px] text-muted-foreground hover:text-foreground",
              tab === t && "bg-foreground/[0.07] text-foreground",
            )}
          >
            {t}
          </button>
        ))}
      </Header>
      <div className="flex min-h-0 flex-1">
        <nav aria-label={t("Tables")} className="w-52 shrink-0 overflow-y-auto border-r py-2">
          {schema.isPending ? (
            <Skeleton className="mx-3 h-32" />
          ) : tables.length === 0 ? (
            <p className="px-3 text-[12.5px] text-muted-foreground">{t("No tables yet.")}</p>
          ) : (
            tables.map((t) => (
              <button
                type="button"
                key={`${t.schema}.${t.name}`}
                onClick={() => setTable(t)}
                className={cn(
                  "flex w-full items-center gap-2 px-3 py-1 text-left text-[12.5px] hover:bg-foreground/[0.04]",
                  table?.name === t.name && table.schema === t.schema && "bg-foreground/[0.07]",
                )}
              >
                <Table2 className={cn("size-3.5 shrink-0", t.kind === "view" && "text-signal")} />
                <span className="truncate">{t.name}</span>
                {t.rows_estimate !== null && (
                  <span className="ml-auto font-mono text-[10.5px] text-muted-foreground">{t.rows_estimate}</span>
                )}
              </button>
            ))
          )}
        </nav>
        <div className="flex min-w-0 flex-1 flex-col">
          {tab === "sql" ? (
            <SqlEditor c={c} allowRemote={allowRemote} tables={tables} />
          ) : !table ? (
            <p className="m-6 text-sm text-muted-foreground">{t("Choose a table.")}</p>
          ) : tab === "structure" ? (
            <Structure c={c} allowRemote={allowRemote} table={table} />
          ) : (
            <DataGrid key={`${table.schema}.${table.name}`} c={c} allowRemote={allowRemote} table={table} />
          )}
        </div>
      </div>
    </>
  );
}

function Structure({ c, allowRemote, table }: { c: DbConnection; allowRemote: boolean; table: DbTable }) {
  const info = useDbTableInfo(c.id, table.schema, table.name, allowRemote);
  if (!info.data) return <Skeleton className="m-6 h-40" />;
  const i = info.data;
  return (
    <div className="overflow-auto p-5 font-mono text-[12.5px]">
      <table className="w-full">
        <thead className="text-left text-[11px] text-muted-foreground uppercase">
          <tr>
            <th className="py-1 pr-4">{t("column")}</th>
            <th className="pr-4">{t("type")}</th>
            <th className="pr-4">{t("null")}</th>
            <th>{t("default")}</th>
          </tr>
        </thead>
        <tbody>
          {i.columns.map((col) => (
            <tr key={col.name} className="border-t">
              <td className="py-1 pr-4">
                {col.primary_key && <span className="mr-1 text-amber">{t("PK")}</span>}
                {col.name}
              </td>
              <td className="pr-4">{col.data_type}</td>
              <td className="pr-4">{col.nullable ? t("yes") : "no"}</td>
              <td className="text-muted-foreground">{col.default ?? ""}</td>
            </tr>
          ))}
        </tbody>
      </table>
      {i.indexes.length > 0 && (
        <>
          <p className="mt-5 mb-1 text-[11px] text-muted-foreground uppercase">{t("indexes")}</p>
          {i.indexes.map((x) => (
            <p key={x.name}>
              {x.name} ({x.columns.join(", ")}){x.unique ? t(" unique") : ""}
            </p>
          ))}
        </>
      )}
      {i.foreign_keys.length > 0 && (
        <>
          <p className="mt-5 mb-1 text-[11px] text-muted-foreground uppercase">{t("foreign keys")}</p>
          {i.foreign_keys.map((f) => (
            <p key={f.name}>
              {f.columns.join(", ")} → {f.references_table} ({f.references_columns.join(", ")})
            </p>
          ))}
        </>
      )}
    </div>
  );
}

function Grid({
  rows,
  info,
  onSort,
  sort,
  onEdit,
  onDelete,
}: {
  rows: DbRows;
  info?: DbTableInfo;
  sort?: { by: string | null; desc: boolean };
  onSort?: (col: string) => void;
  onEdit?: (rowIndex: number, col: string, value: string | null) => void;
  onDelete?: (rowIndex: number) => void;
}) {
  const [editing, setEditing] = useState<{ row: number; col: string; value: string } | null>(null);
  const editable = !!info?.columns.some((col) => col.primary_key) && !!onEdit;
  const commit = (value: string | null) => {
    if (editing && onEdit) onEdit(editing.row, editing.col, value);
    setEditing(null);
  };
  return (
    <div className="min-h-0 flex-1 overflow-auto">
      <table className="w-max min-w-full font-mono text-[12px]">
        <thead className="sticky top-0 bg-background text-left">
          <tr>
            {rows.columns.map((col) => (
              <th key={col} className="border-b px-3 py-1.5 font-medium whitespace-nowrap">
                {onSort ? (
                  <button type="button" onClick={() => onSort(col)} className="hover:text-ember">
                    {col}
                    {sort?.by === col ? (sort.desc ? " ↓" : " ↑") : ""}
                  </button>
                ) : (
                  col
                )}
              </th>
            ))}
            {onDelete && <th className="border-b" />}
          </tr>
        </thead>
        <tbody>
          {rows.rows.map((row, r) => (
            <tr key={r} className="group border-b border-dashed hover:bg-foreground/[0.02]">
              {row.map((cell, i) => {
                const col = rows.columns[i];
                const isEditing = editing?.row === r && editing.col === col;
                return (
                  <td
                    key={col}
                    className="max-w-80 truncate px-3 py-1"
                    onDoubleClick={() => editable && setEditing({ row: r, col, value: cell ?? "" })}
                    title={editable ? "Double-click to edit" : undefined}
                  >
                    {isEditing ? (
                      <span className="flex items-center gap-1">
                        <input
                          autoFocus
                          value={editing.value}
                          onChange={(e) => setEditing({ ...editing, value: e.target.value })}
                          onKeyDown={(e) => {
                            if (e.key === "Enter") commit(editing.value);
                            if (e.key === "Escape") setEditing(null);
                          }}
                          className="w-48 rounded border bg-background px-1 outline-none focus:border-ember"
                          aria-label={t("New value for {col}", { col })}
                        />
                        <button
                          type="button"
                          className="text-[10.5px] text-muted-foreground"
                          onClick={() => commit(null)}
                        >
                          {t("NULL")}
                        </button>
                      </span>
                    ) : cell === null ? (
                      <span className="text-muted-foreground/60">{t("NULL")}</span>
                    ) : (
                      cell
                    )}
                  </td>
                );
              })}
              {onDelete && (
                <td className="px-2">
                  <button
                    type="button"
                    aria-label={t("Delete row")}
                    onClick={() => onDelete(r)}
                    className="text-muted-foreground opacity-0 group-hover:opacity-100 hover:text-destructive"
                  >
                    <Trash2 className="size-3.5" />
                  </button>
                </td>
              )}
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}

function DataGrid({ c, allowRemote, table }: { c: DbConnection; allowRemote: boolean; table: DbTable }) {
  const info = useDbTableInfo(c.id, table.schema, table.name, allowRemote);
  const [q, setQ] = useState<RowsQuery>({ page: 0, pageSize: PAGE, orderBy: null, desc: false, filters: [] });
  const rows = useDbRows(c.id, table.schema, table.name, q, allowRemote);
  const { edit } = useDbActions(c.id, allowRemote);
  const [draft, setDraft] = useState<DbFilter>({ column: "", op: "=", value: "" });
  const [deleting, setDeleting] = useState<number | null>(null);
  const [adding, setAdding] = useState(false);
  const columns = info.data?.columns ?? [];
  const pk = columns.filter((col) => col.primary_key).map((col) => col.name);
  const keyOf = (r: number): [string, string | null][] =>
    pk.map((name) => [name, rows.data!.rows[r][rows.data!.columns.indexOf(name)]]);
  const run = (e: Parameters<typeof edit.mutate>[0]["edit"], done?: () => void) =>
    edit.mutate(
      { schema: table.schema, table: table.name, edit: e },
      {
        onSuccess: done,
        onError: (err) => toast.error(t("The database refused it"), { description: errorMessage(err) }),
      },
    );
  const pages = rows.data?.total != null ? Math.max(1, Math.ceil(rows.data.total / q.pageSize)) : 1;

  return (
    <>
      <div className="flex flex-wrap items-center gap-2 border-b px-4 py-2 text-[12.5px]">
        <select
          aria-label={t("Filter column")}
          value={draft.column}
          onChange={(e) => setDraft({ ...draft, column: e.target.value })}
          className="h-7 rounded-md border bg-background px-1.5"
        >
          <option value="">{t("column…")}</option>
          {columns.map((col) => (
            <option key={col.name}>{col.name}</option>
          ))}
        </select>
        <select
          aria-label={t("Filter operator")}
          value={draft.op}
          onChange={(e) => setDraft({ ...draft, op: e.target.value as DbFilter["op"] })}
          className="h-7 rounded-md border bg-background px-1.5"
        >
          {OPS.map((op) => (
            <option key={op}>{op}</option>
          ))}
        </select>
        {draft.op !== "is_null" && draft.op !== "not_null" && (
          <Input
            aria-label={t("Filter value")}
            value={draft.value ?? ""}
            onChange={(e) => setDraft({ ...draft, value: e.target.value })}
            className="h-7 w-40 font-mono text-[12px]"
            placeholder={draft.op === "like" ? "%text%" : "value"}
          />
        )}
        <Button
          size="xs"
          variant="outline"
          disabled={!draft.column}
          onClick={() => {
            setQ({ ...q, page: 0, filters: [...q.filters, draft] });
            setDraft({ column: "", op: "=", value: "" });
          }}
        >
          {t("Filter")}
        </Button>
        {q.filters.map((f, i) => (
          <span key={i} className="flex items-center gap-1 rounded border px-1.5 font-mono text-[11px]">
            {f.column} {f.op} {f.op === "is_null" || f.op === "not_null" ? "" : f.value}
            <button
              type="button"
              aria-label={t("Remove filter")}
              onClick={() => setQ({ ...q, page: 0, filters: q.filters.filter((_, j) => j !== i) })}
            >
              <X className="size-3" />
            </button>
          </span>
        ))}
        <span className="ml-auto flex items-center gap-1">
          {pk.length > 0 && (
            <Button size="xs" variant="ghost" onClick={() => setAdding(true)}>
              <Plus data-icon="inline-start" /> {t("Row")}
            </Button>
          )}
          <Button
            size="icon-sm"
            variant="ghost"
            aria-label={t("Previous page")}
            disabled={q.page === 0}
            onClick={() => setQ({ ...q, page: q.page - 1 })}
          >
            <ChevronLeft />
          </Button>
          <span className="font-mono text-[11.5px] text-muted-foreground">
            {q.page + 1}/{pages} · {rows.data?.total ?? "?"} {t("rows")}
          </span>
          <Button
            size="icon-sm"
            variant="ghost"
            aria-label={t("Next page")}
            disabled={q.page + 1 >= pages}
            onClick={() => setQ({ ...q, page: q.page + 1 })}
          >
            <ChevronRight />
          </Button>
        </span>
      </div>
      {rows.isError ? (
        <p className="m-6 text-sm text-destructive">{errorMessage(rows.error)}</p>
      ) : !rows.data ? (
        <Skeleton className="m-6 h-40" />
      ) : (
        <Grid
          rows={rows.data}
          info={info.data}
          sort={{ by: q.orderBy, desc: q.desc }}
          onSort={(col) => setQ({ ...q, orderBy: col, desc: q.orderBy === col ? !q.desc : false })}
          onEdit={(r, col, value) =>
            run({ op: "update", key: keyOf(r), changes: [[col, value]] }, () => toast.success(`Saved ${col}`))
          }
          onDelete={pk.length ? (r) => setDeleting(r) : undefined}
        />
      )}
      {pk.length === 0 && info.data && (
        <p className="border-t px-4 py-1.5 text-[12px] text-muted-foreground">
          {t("No primary key: edit this table with SQL.")}
        </p>
      )}
      <Dialog open={deleting !== null} onOpenChange={(open) => !open && setDeleting(null)}>
        <DialogContent className="sm:max-w-sm">
          <DialogHeader>
            <DialogTitle>{t("Delete this row?")}</DialogTitle>
            <DialogDescription>
              {deleting !== null && rows.data
                ? keyOf(deleting)
                    .map(([k, v]) => `${k} = ${v}`)
                    .join(", ")
                : ""}{" "}
              {t("in {table}. It can't be undone from here.", { table: table.name })}
            </DialogDescription>
          </DialogHeader>
          <DialogFooter>
            <Button variant="ghost" onClick={() => setDeleting(null)}>
              {t("Cancel")}
            </Button>
            <Button
              variant="destructive"
              onClick={() => {
                if (deleting !== null) run({ op: "delete", key: keyOf(deleting) });
                setDeleting(null);
              }}
            >
              {t("Delete")}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
      <Dialog open={adding} onOpenChange={setAdding}>
        <DialogContent className="sm:max-w-lg">
          <InsertForm
            info={info.data}
            onCancel={() => setAdding(false)}
            onSave={(values) => run({ op: "insert", values }, () => setAdding(false))}
          />
        </DialogContent>
      </Dialog>
    </>
  );
}

function InsertForm({
  info,
  onCancel,
  onSave,
}: {
  info?: DbTableInfo;
  onCancel: () => void;
  onSave: (values: [string, string | null][]) => void;
}) {
  const [values, setValues] = useState<Record<string, string>>({});
  if (!info) return null;
  return (
    <form
      onSubmit={(e) => {
        e.preventDefault();
        // Empty fields are left out, so the column's default applies.
        onSave(Object.entries(values).filter(([, v]) => v !== "") as [string, string][]);
      }}
      className="flex flex-col gap-3"
    >
      <DialogHeader>
        <DialogTitle>{t("New row in {name}", { name: info.table.name })}</DialogTitle>
        <DialogDescription>{t("Leave a field empty to use its default.")}</DialogDescription>
      </DialogHeader>
      <div className="grid max-h-[50vh] gap-2 overflow-y-auto">
        {info.columns.map((col) => (
          <div key={col.name} className="grid grid-cols-[10rem_1fr] items-center gap-2">
            <Label htmlFor={`col-${col.name}`} className="truncate font-mono text-[12px]">
              {col.name}
            </Label>
            <Input
              id={`col-${col.name}`}
              value={values[col.name] ?? ""}
              onChange={(e) => setValues({ ...values, [col.name]: e.target.value })}
              placeholder={col.default ? t("default {value}", { value: col.default }) : col.data_type}
              className="font-mono text-[12px]"
            />
          </div>
        ))}
      </div>
      <DialogFooter>
        <Button type="button" variant="ghost" onClick={onCancel}>
          {t("Cancel")}
        </Button>
        <Button type="submit">{t("Insert")}</Button>
      </DialogFooter>
    </form>
  );
}

function SqlEditor({ c, allowRemote, tables }: { c: DbConnection; allowRemote: boolean; tables: DbTable[] }) {
  const { check, query } = useDbActions(c.id, allowRemote);
  const [sql, setSql] = useState("");
  const [result, setResult] = useState<DbRows | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [confirming, setConfirming] = useState<QueryCheck | null>(null);
  const [phrase, setPhrase] = useState("");
  const [history, setHistory] = useState(readHistory);
  const editor = useRef<CodeEditorHandle>(null);

  const execute = async (confirm?: string) => {
    setError(null);
    try {
      const rows = await query.mutateAsync({ sql, confirm });
      setResult(rows);
      const next = [sql, ...history.filter((h) => h !== sql)];
      setHistory(next);
      writeHistory(next);
    } catch (e) {
      setError(errorMessage(e));
    }
  };
  const run = async () => {
    if (!sql.trim()) return;
    const verdict = await check(sql).catch(() => null);
    if (verdict?.confirm) {
      setPhrase("");
      setConfirming(verdict);
      return;
    }
    execute();
  };
  const insert = (text: string) => editor.current?.insert(text);

  return (
    <div className="flex min-h-0 flex-1 flex-col">
      <div className="h-44 shrink-0 border-b">
        <CodeEditor
          ref={editor}
          value={sql}
          onChange={setSql}
          language="sql"
          wrap
          ariaLabel="SQL"
          placeholder={t("select * from users limit 10;   (Ctrl+Enter runs)")}
          onSubmit={run}
          className="h-full rounded-none border-0"
        />
      </div>
      <div className="flex flex-wrap items-center gap-2 border-b px-4 py-1.5">
        <Button
          size="xs"
          onClick={run}
          disabled={query.isPending || !sql.trim()}
          className="bg-ember text-ember-foreground hover:bg-ember/85"
        >
          <Play data-icon="inline-start" className="fill-current" /> {t("Run")}
        </Button>
        <span className="text-[11.5px] text-muted-foreground">{t("Insert:")}</span>
        {tables.slice(0, 12).map((t) => (
          <button
            type="button"
            key={t.name}
            onClick={() => insert(t.name)}
            className="font-mono text-[11.5px] text-muted-foreground hover:text-foreground"
          >
            {t.name}
          </button>
        ))}
        {result && (
          <span className="ml-auto font-mono text-[11px] text-muted-foreground">
            {result.columns.length
              ? t("{n} rows", { n: `${result.rows.length}${result.truncated ? "+" : ""}` })
              : t("{n} affected", { n: result.affected ?? 0 })}{" "}
            · {result.elapsed_ms} {t("ms")}
          </span>
        )}
      </div>
      {error ? (
        <p className="m-4 font-mono text-[12.5px] text-destructive">{error}</p>
      ) : result && result.columns.length > 0 ? (
        <Grid rows={result} />
      ) : (
        <div className="overflow-y-auto p-4">
          <p className="mb-1 text-[11px] text-muted-foreground uppercase">{t("history")}</p>
          {history.length === 0 && (
            <p className="text-[12.5px] text-muted-foreground">{t("Queries you run show up here.")}</p>
          )}
          {history.map((h) => (
            <button
              type="button"
              key={h}
              onClick={() => setSql(h)}
              className="block w-full truncate py-0.5 text-left font-mono text-[12px] text-muted-foreground hover:text-foreground"
            >
              {h}
            </button>
          ))}
        </div>
      )}
      <Dialog open={confirming !== null} onOpenChange={(open) => !open && setConfirming(null)}>
        <DialogContent className="sm:max-w-md">
          <DialogHeader>
            <DialogTitle>{t("This destroys data")}</DialogTitle>
            <DialogDescription>
              {t("On")} {c.label}
              {c.remote ? t(" (remote: {host})", { host: c.host }) : ""}:
            </DialogDescription>
          </DialogHeader>
          <ul className="font-mono text-[12px] text-destructive">
            {confirming?.destructive.map((d) => (
              <li key={d}>{d}</li>
            ))}
          </ul>
          <div className="flex flex-col gap-1.5">
            <Label htmlFor="confirm-db">{tx("Type {phrase} to run it", { phrase: confirming?.confirm })}</Label>
            <Input
              id="confirm-db"
              value={phrase}
              onChange={(e) => setPhrase(e.target.value)}
              className="font-mono"
              autoFocus
            />
          </div>
          <DialogFooter>
            <Button variant="ghost" onClick={() => setConfirming(null)}>
              {t("Cancel")}
            </Button>
            <Button
              variant="destructive"
              disabled={phrase !== confirming?.confirm}
              onClick={() => {
                setConfirming(null);
                execute(phrase);
              }}
            >
              {t("Run")}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}

function RedisView({ connection: c, allowRemote }: { connection: DbConnection; allowRemote: boolean }) {
  const [pattern, setPattern] = useState("*");
  const keys = useRedisKeys(c.id, pattern, allowRemote);
  const [key, setKey] = useState<string | null>(null);
  const value = useRedisValue(c.id, key, allowRemote);
  const { redisEdit } = useDbActions(c.id, allowRemote);
  const [draft, setDraft] = useState<string | null>(null);
  const [confirmDelete, setConfirmDelete] = useState(false);
  const fail = (e: unknown) => toast.error(t("Redis refused it"), { description: errorMessage(e) });

  return (
    <>
      <Header c={c} />
      {keys.isError ? (
        <p className="m-6 text-sm text-destructive">
          {isRemote(keys.error) ? t("Remote connection not allowed.") : errorMessage(keys.error)}
        </p>
      ) : (
        <div className="flex min-h-0 flex-1">
          <div className="flex w-80 shrink-0 flex-col border-r">
            <Input
              value={pattern}
              onChange={(e) => setPattern(e.target.value)}
              aria-label={t("Key pattern")}
              className="m-2 h-7 w-auto font-mono text-[12px]"
            />
            <div className="min-h-0 flex-1 overflow-y-auto">
              {(keys.data?.keys ?? []).map((k) => (
                <button
                  type="button"
                  key={k.key}
                  onClick={() => {
                    setKey(k.key);
                    setDraft(null);
                  }}
                  className={cn(
                    "flex w-full items-center gap-2 px-3 py-1 text-left font-mono text-[12px] hover:bg-foreground/[0.04]",
                    key === k.key && "bg-foreground/[0.07]",
                  )}
                >
                  <span className="w-10 shrink-0 text-[10.5px] text-muted-foreground">{k.kind}</span>
                  <span className="truncate">{k.key}</span>
                  {k.ttl >= 0 && <span className="ml-auto text-[10.5px] text-muted-foreground">{k.ttl}s</span>}
                </button>
              ))}
              {keys.data?.cursor && keys.data.cursor !== "0" && (
                <p className="px-3 py-1 text-[11.5px] text-muted-foreground">
                  {t("More keys exist; narrow the pattern.")}
                </p>
              )}
            </div>
          </div>
          <div className="min-w-0 flex-1 overflow-y-auto p-4">
            {!key ? (
              <p className="text-sm text-muted-foreground">
                {t("Choose a key. Keys are listed with SCAN, a page at a time.")}
              </p>
            ) : !value.data ? (
              value.isError ? (
                <p className="text-sm text-destructive">{errorMessage(value.error)}</p>
              ) : (
                <Skeleton className="h-24" />
              )
            ) : (
              <div className="flex flex-col gap-3">
                <div className="flex items-center gap-2">
                  <span className="font-mono text-[13px] font-medium">{value.data.key}</span>
                  <span className="font-mono text-[11px] whitespace-nowrap text-muted-foreground">
                    {value.data.kind} ·{" "}
                    {value.data.ttl < 0 ? t("no expiry") : t("expires in {ttl}s", { ttl: value.data.ttl })}
                  </span>
                  <span className="ml-auto flex gap-1">
                    {value.data.ttl >= 0 && (
                      <Button
                        size="xs"
                        variant="ghost"
                        onClick={() =>
                          redisEdit.mutate({ op: "expire", key: value.data!.key, seconds: null }, { onError: fail })
                        }
                      >
                        {t("Remove expiry")}
                      </Button>
                    )}
                    <Button
                      size="xs"
                      variant="ghost"
                      className="text-destructive"
                      onClick={() => setConfirmDelete(true)}
                    >
                      <Trash2 data-icon="inline-start" /> {t("Delete")}
                    </Button>
                  </span>
                </div>
                {value.data.kind === "string" ? (
                  <>
                    <textarea
                      value={draft ?? value.data.items[0]?.[0] ?? ""}
                      onChange={(e) => setDraft(e.target.value)}
                      aria-label={t("Value")}
                      className="min-h-32 rounded-md border bg-background p-2 font-mono text-[12.5px] outline-none focus:border-ember"
                    />
                    <Button
                      size="xs"
                      className="self-start"
                      disabled={draft === null}
                      onClick={() =>
                        redisEdit.mutate(
                          { op: "set", key: value.data!.key, value: draft ?? "" },
                          { onSuccess: () => toast.success(t("Saved")), onError: fail },
                        )
                      }
                    >
                      {t("Save")}
                    </Button>
                  </>
                ) : (
                  <table className="font-mono text-[12px]">
                    <tbody>
                      {value.data.items.map(([a, b], i) => (
                        <tr key={i} className="border-b border-dashed">
                          <td className="py-1 pr-4">{a}</td>
                          {b !== null && <td className="text-muted-foreground">{b}</td>}
                        </tr>
                      ))}
                    </tbody>
                  </table>
                )}
                {value.data.truncated && (
                  <p className="text-[12px] text-muted-foreground">{t("Only the first items are shown.")}</p>
                )}
              </div>
            )}
          </div>
        </div>
      )}
      <Dialog open={confirmDelete} onOpenChange={setConfirmDelete}>
        <DialogContent className="sm:max-w-sm">
          <DialogHeader>
            <DialogTitle>{t("Delete {key}?", { key })}</DialogTitle>
            <DialogDescription>{t("On {label}. It can't be undone.", { label: c.label })}</DialogDescription>
          </DialogHeader>
          <DialogFooter>
            <Button variant="ghost" onClick={() => setConfirmDelete(false)}>
              {t("Cancel")}
            </Button>
            <Button
              variant="destructive"
              onClick={() => {
                if (key) redisEdit.mutate({ op: "delete", key }, { onError: fail });
                setKey(null);
                setConfirmDelete(false);
              }}
            >
              {t("Delete")}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </>
  );
}

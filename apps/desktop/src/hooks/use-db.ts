// The database manager. Every call names a connection id; credentials stay
// in the daemon.

import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";

import { request } from "@/lib/daemon";
import type { DbFilter, RedisEdit, RowEdit } from "@/lib/protocol";

export function useDbConnections() {
  return useQuery({
    queryKey: ["db", "connections"],
    queryFn: async () => (await request({ type: "db_connections" }, "db_connections")).connections,
    retry: false,
  });
}

export function useDbSchema(connection: string | null, allowRemote: boolean) {
  return useQuery({
    queryKey: ["db", "schema", connection, allowRemote],
    queryFn: async () => {
      const { databases, tables } = await request(
        { type: "db_schema", connection: connection!, allow_remote: allowRemote },
        "db_schema",
      );
      return { databases, tables };
    },
    enabled: connection !== null,
    retry: false,
  });
}

export function useDbTableInfo(connection: string | null, schema: string, table: string | null, allowRemote: boolean) {
  return useQuery({
    queryKey: ["db", "info", connection, schema, table],
    queryFn: async () => {
      const { type: _, ...info } = await request(
        { type: "db_table_info", connection: connection!, schema, table: table!, allow_remote: allowRemote },
        "db_table_info",
      );
      return info;
    },
    enabled: connection !== null && table !== null,
    retry: false,
  });
}

export interface RowsQuery {
  page: number;
  pageSize: number;
  orderBy: string | null;
  desc: boolean;
  filters: DbFilter[];
}

export function useDbRows(
  connection: string | null,
  schema: string,
  table: string | null,
  q: RowsQuery,
  allowRemote: boolean,
) {
  return useQuery({
    queryKey: ["db", "rows", connection, schema, table, q],
    queryFn: async () => {
      const { type: _, ...rows } = await request(
        {
          type: "db_rows",
          connection: connection!,
          schema,
          table: table!,
          page: q.page,
          page_size: q.pageSize,
          order_by: q.orderBy,
          desc: q.desc,
          filters: q.filters,
          allow_remote: allowRemote,
        },
        "db_rows",
      );
      return rows;
    },
    enabled: connection !== null && table !== null,
    placeholderData: (previous) => previous,
    retry: false,
  });
}

export function useDbActions(connection: string | null, allowRemote: boolean) {
  const client = useQueryClient();
  const refresh = () => client.invalidateQueries({ queryKey: ["db"] });
  const edit = useMutation({
    mutationFn: (input: { schema: string; table: string; edit: RowEdit }) =>
      request({ type: "db_edit_row", connection: connection!, allow_remote: allowRemote, ...input }, "db_affected"),
    onSettled: refresh,
  });
  const check = (sql: string) => request({ type: "db_check", connection: connection!, sql }, "db_check");
  const query = useMutation({
    mutationFn: async (input: { sql: string; confirm?: string | null }) => {
      const { type: _, ...rows } = await request(
        { type: "db_query", connection: connection!, allow_remote: allowRemote, ...input },
        "db_rows",
      );
      return rows;
    },
    onSettled: refresh,
  });
  const create = useMutation({
    mutationFn: () => request({ type: "db_create_database", connection: connection! }, "ok"),
    onSettled: refresh,
  });
  const redisEdit = useMutation({
    mutationFn: (e: RedisEdit) =>
      request({ type: "redis_edit", connection: connection!, edit: e, allow_remote: allowRemote }, "ok"),
    onSettled: refresh,
  });
  return { edit, check, query, create, redisEdit };
}

export function useRedisKeys(connection: string | null, pattern: string, allowRemote: boolean) {
  return useQuery({
    queryKey: ["db", "redis-keys", connection, pattern, allowRemote],
    queryFn: async () => {
      const { cursor, keys } = await request(
        { type: "redis_keys", connection: connection!, pattern: pattern || "*", allow_remote: allowRemote },
        "redis_keys",
      );
      return { cursor, keys };
    },
    enabled: connection !== null,
    retry: false,
  });
}

export function useRedisValue(connection: string | null, key: string | null, allowRemote: boolean) {
  return useQuery({
    queryKey: ["db", "redis-value", connection, key],
    queryFn: async () => {
      const { type: _, ...value } = await request(
        { type: "redis_get", connection: connection!, key: key!, allow_remote: allowRemote },
        "redis_value",
      );
      return value;
    },
    enabled: connection !== null && key !== null,
    retry: false,
  });
}

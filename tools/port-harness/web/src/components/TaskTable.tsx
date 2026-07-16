import {
  useReactTable,
  getCoreRowModel,
  getSortedRowModel,
  flexRender,
  createColumnHelper,
  type RowSelectionState,
  type SortingState,
  type ColumnDef,
} from "@tanstack/react-table";
import { useState } from "react";
import { Link } from "react-router-dom";
import type { TaskWithDetails } from "../api/client";
import { StatusBadge } from "./StatusBadge";

const columnHelper = createColumnHelper<TaskWithDetails>();

interface Props {
  tasks: TaskWithDetails[];
  onSelectionChange?: (ids: number[]) => void;
}

function SortIndicator({ sorted, desc }: { sorted: boolean; desc: boolean }) {
  if (!sorted) return <span className="th-sort" aria-hidden>⇅</span>;
  return <span className="th-sort th-sort-active" aria-hidden>{desc ? "▼" : "▲"}</span>;
}

export function TaskTable({ tasks, onSelectionChange }: Props) {
  const hasSelection = !!onSelectionChange;
  const [rowSelection, setRowSelection] = useState<RowSelectionState>({});
  const [sorting, setSorting] = useState<SortingState>([]);

  const columns: ColumnDef<TaskWithDetails, any>[] = [
    ...(hasSelection
      ? [
          columnHelper.display({
            id: "select",
            enableSorting: false,
            header: ({ table }) => (
              <input
                type="checkbox"
                checked={table.getIsAllRowsSelected()}
                onChange={table.getToggleAllRowsSelectedHandler()}
              />
            ),
            cell: ({ row }) => (
              <input
                type="checkbox"
                checked={row.getIsSelected()}
                onChange={row.getToggleSelectedHandler()}
              />
            ),
          }),
        ]
      : []),
    columnHelper.accessor("symbol_name", {
      header: "Symbol",
      cell: (info) => (
        <Link to={`/symbols/${info.row.original.source_symbol_id}`}>
          {info.getValue()}
        </Link>
      ),
    }),
    columnHelper.accessor("symbol_file", {
      header: "File",
      cell: (info) => (
        <span>
          {info.getValue()}:{info.row.original.start_line}
        </span>
      ),
    }),
    columnHelper.accessor("flow_name", {
      header: "Flow",
      cell: (info) => {
        const name = info.getValue();
        const flowId = info.row.original.flow_id;
        if (!name || !flowId) return "—";
        return <Link to={`/flows/${flowId}`}>{name}</Link>;
      },
    }),
    columnHelper.accessor("status", {
      header: "Status",
      cell: (info) => <StatusBadge status={info.getValue()} />,
    }),
    columnHelper.accessor("target_rust_file", {
      header: "Rust Target",
      cell: (info) => (
        <span style={{ fontSize: "0.8rem" }}>{info.getValue() ?? "-"}</span>
      ),
    }),
    columnHelper.accessor("claim_count", { header: "Claims" }),
    columnHelper.accessor("fixture_count", { header: "Fixtures" }),
    columnHelper.accessor("notes", {
      header: "Notes",
      cell: (info) => (
        <span style={{ fontSize: "0.8rem", color: "#94a3b8" }}>
          {info.getValue() ?? ""}
        </span>
      ),
    }),
  ];

  const table = useReactTable({
    data: tasks,
    columns,
    state: { rowSelection, sorting },
    enableRowSelection: hasSelection,
    onRowSelectionChange: (updater) => {
      const next = typeof updater === "function" ? updater(rowSelection) : updater;
      setRowSelection(next);
      const ids = Object.keys(next)
        .filter((k) => next[k])
        .map((k) => tasks[parseInt(k, 10)]?.id)
        .filter((id): id is number => id !== undefined);
      onSelectionChange?.(ids);
    },
    onSortingChange: setSorting,
    getCoreRowModel: getCoreRowModel(),
    getSortedRowModel: getSortedRowModel(),
  });

  return (
    <table>
      <thead>
        {table.getHeaderGroups().map((hg) => (
          <tr key={hg.id}>
            {hg.headers.map((h) => {
              const canSort = h.column.getCanSort();
              const sortDir = h.column.getIsSorted();
              return (
                <th
                  key={h.id}
                  className={canSort ? "th-sortable" : undefined}
                  onClick={canSort ? h.column.getToggleSortingHandler() : undefined}
                  style={canSort ? { cursor: "pointer", userSelect: "none" } : undefined}
                >
                  <span style={{ display: "inline-flex", alignItems: "center", gap: "0.3rem" }}>
                    {flexRender(h.column.columnDef.header, h.getContext())}
                    {canSort && <SortIndicator sorted={!!sortDir} desc={sortDir === "desc"} />}
                  </span>
                </th>
              );
            })}
          </tr>
        ))}
      </thead>
      <tbody>
        {table.getRowModel().rows.map((row) => (
          <tr key={row.id}>
            {row.getVisibleCells().map((cell) => (
              <td key={cell.id}>
                {flexRender(cell.column.columnDef.cell, cell.getContext())}
              </td>
            ))}
          </tr>
        ))}
      </tbody>
    </table>
  );
}
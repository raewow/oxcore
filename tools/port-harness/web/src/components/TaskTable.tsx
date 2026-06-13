import {
  useReactTable,
  getCoreRowModel,
  flexRender,
  createColumnHelper,
  type RowSelectionState,
} from "@tanstack/react-table";
import { useState } from "react";
import { Link } from "react-router-dom";
import type { TaskWithDetails } from "../api/client";
import { StatusBadge } from "./StatusBadge";

const columnHelper = createColumnHelper<TaskWithDetails>();

interface Props {
  tasks: TaskWithDetails[];
  onSelectionChange: (ids: number[]) => void;
}

export function TaskTable({ tasks, onSelectionChange }: Props) {
  const [rowSelection, setRowSelection] = useState<RowSelectionState>({});

  const columns = [
    columnHelper.display({
      id: "select",
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
    state: { rowSelection },
    onRowSelectionChange: (updater) => {
      const next = typeof updater === "function" ? updater(rowSelection) : updater;
      setRowSelection(next);
      const ids = Object.keys(next)
        .filter((k) => next[k])
        .map((k) => tasks[parseInt(k, 10)]?.id)
        .filter((id): id is number => id !== undefined);
      onSelectionChange(ids);
    },
    getCoreRowModel: getCoreRowModel(),
  });

  return (
    <table>
      <thead>
        {table.getHeaderGroups().map((hg) => (
          <tr key={hg.id}>
            {hg.headers.map((h) => (
              <th key={h.id}>
                {flexRender(h.column.columnDef.header, h.getContext())}
              </th>
            ))}
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

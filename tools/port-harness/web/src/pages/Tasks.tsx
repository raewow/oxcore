import { useState, useEffect } from "react";
import { useSearchParams } from "react-router-dom";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { api } from "../api/client";
import { TaskTable } from "../components/TaskTable";
import { BulkActionBar } from "../components/BulkActionBar";

export function Tasks() {
  const [searchParams] = useSearchParams();
  const [file, setFile] = useState(searchParams.get("file") ?? "");
  const [status, setStatus] = useState("");
  const [q, setQ] = useState("");
  const [selectedIds, setSelectedIds] = useState<number[]>([]);
  const queryClient = useQueryClient();

  useEffect(() => {
    const f = searchParams.get("file");
    if (f) setFile(f);
  }, [searchParams]);

  const { data, isLoading } = useQuery({
    queryKey: ["tasks", file, status, q],
    queryFn: () =>
      api.getTasks({
        file: file || undefined,
        status: status || undefined,
        q: q || undefined,
        limit: "500",
      }),
  });

  const bulkMutation = useMutation({
    mutationFn: ({ ids, fields }: { ids: number[]; fields: Record<string, unknown> }) =>
      api.bulkUpdateTasks(ids, fields),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["tasks"] });
      setSelectedIds([]);
    },
  });

  const jobMutation = useMutation({
    mutationFn: ({ stage, taskIds }: { stage: string; taskIds: number[] }) =>
      api.createJob(stage, taskIds),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["jobs"] });
      setSelectedIds([]);
    },
  });

  return (
    <div>
      <div className="page-header">
        <h2>Tasks</h2>
        <p style={{ color: "#94a3b8" }}>
          {data?.total ?? 0} migration tasks
        </p>
      </div>

      <div className="filters">
        <input
          placeholder="Search symbols..."
          value={q}
          onChange={(e) => setQ(e.target.value)}
        />
        <input
          placeholder="Filter file..."
          value={file}
          onChange={(e) => setFile(e.target.value)}
        />
        <select value={status} onChange={(e) => setStatus(e.target.value)}>
          <option value="">All statuses</option>
          <option value="discovered">discovered</option>
          <option value="documented">documented</option>
          <option value="fixture_defined">fixture_defined</option>
          <option value="rust_planned">rust_planned</option>
          <option value="rust_ported">rust_ported</option>
          <option value="verified">verified</option>
          <option value="done">done</option>
          <option value="blocked">blocked</option>
        </select>
      </div>

      <BulkActionBar
        selectedCount={selectedIds.length}
        onSetStatus={(s) => bulkMutation.mutate({ ids: selectedIds, fields: { status: s } })}
        onQueueJob={(stage) => {
          if (
            window.confirm(
              `Queue ${stage} for ${selectedIds.length} tasks? This may invoke LLM API calls.`,
            )
          ) {
            jobMutation.mutate({ stage, taskIds: selectedIds });
          }
        }}
        onClear={() => setSelectedIds([])}
      />

      {isLoading ? (
        <div>Loading...</div>
      ) : (
        <TaskTable tasks={data?.tasks ?? []} onSelectionChange={setSelectedIds} />
      )}
    </div>
  );
}

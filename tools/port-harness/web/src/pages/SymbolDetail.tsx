import { useParams } from "react-router-dom";
import { useQuery } from "@tanstack/react-query";
import { api } from "../api/client";
import { SourceViewer } from "../components/SourceViewer";
import { StatusBadge } from "../components/StatusBadge";

export function SymbolDetail() {
  const { id } = useParams<{ id: string }>();
  const symbolId = parseInt(id ?? "0", 10);

  const { data: source, isLoading: sourceLoading } = useQuery({
    queryKey: ["symbol-source", symbolId],
    queryFn: () => api.getSymbolSource(symbolId),
    enabled: symbolId > 0,
  });

  const { data: taskData } = useQuery({
    queryKey: ["task-by-symbol", symbolId],
    queryFn: async () => {
      const { tasks } = await api.getTasks({ limit: "1000" });
      const task = tasks.find((t) => t.source_symbol_id === symbolId);
      if (!task) return null;
      return api.getTask(task.id);
    },
    enabled: symbolId > 0,
  });

  if (sourceLoading) return <div>Loading...</div>;
  if (!source) return <div>Symbol not found</div>;

  const sym = source.symbol as { name: string; file: string };

  return (
    <div>
      <div className="page-header">
        <h2>{sym.name}</h2>
        <p style={{ color: "#94a3b8" }}>
          {sym.file}:{source.start_line}-{source.end_line}
        </p>
        {taskData?.task && (
          <StatusBadge status={(taskData.task as { status: string }).status} />
        )}
      </div>

      <h3 style={{ marginBottom: "0.75rem" }}>Source</h3>
      <SourceViewer
        snippet={source.snippet}
        baseLine={source.start_line}
      />

      {taskData?.claims && (taskData.claims as unknown[]).length > 0 && (
        <>
          <h3 style={{ margin: "1.5rem 0 0.75rem" }}>Behaviour Claims</h3>
          <table>
            <thead>
              <tr>
                <th>Category</th>
                <th>Claim</th>
                <th>Source</th>
              </tr>
            </thead>
            <tbody>
              {(taskData.claims as { category: string; claim_text: string; file: string; start_line: number; end_line: number }[]).map(
                (c, i) => (
                  <tr key={i}>
                    <td>{c.category}</td>
                    <td>{c.claim_text}</td>
                    <td>
                      {c.file}:{c.start_line}-{c.end_line}
                    </td>
                  </tr>
                ),
              )}
            </tbody>
          </table>
        </>
      )}
    </div>
  );
}

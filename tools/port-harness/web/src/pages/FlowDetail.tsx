import { useParams } from "react-router-dom";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { api } from "../api/client";
import { StatusBadge } from "../components/StatusBadge";

export function FlowDetail() {
  const { id } = useParams<{ id: string }>();
  const flowId = parseInt(id ?? "0", 10);
  const queryClient = useQueryClient();

  const { data, isLoading } = useQuery({
    queryKey: ["flow", flowId],
    queryFn: () => api.getFlow(flowId),
    enabled: flowId > 0,
  });

  const auditMutation = useMutation({
    mutationFn: () => api.auditFlow(flowId),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["flow", flowId] });
      queryClient.invalidateQueries({ queryKey: ["jobs"] });
      queryClient.invalidateQueries({ queryKey: ["tasks"] });
    },
  });

  if (isLoading) return <div>Loading...</div>;
  if (!data) return <div>Flow not found</div>;

  const flow = data as {
    flow: { name: string; description: string; expected_behaviour: string };
    branches: { condition: string; behaviour: string; file: string; start_line: number }[];
    mutations: { variable_or_field: string; mutation_description: string }[];
    tasks: {
      id: number;
      symbol_name: string;
      status: string;
      target_rust_file: string | null;
      notes: string | null;
    }[];
  };

  const documented = flow.tasks.filter((t) => t.status !== "discovered").length;

  return (
    <div>
      <div className="page-header">
        <h2>{flow.flow.name}</h2>
        <p style={{ color: "#94a3b8" }}>{flow.flow.description}</p>
        <div className="action-buttons" style={{ marginTop: "0.75rem" }}>
          <button
            type="button"
            className="btn"
            disabled={auditMutation.isPending || flow.tasks.length === 0}
            onClick={() => {
              if (
                window.confirm(
                  `Audit Rust implementation for ${flow.tasks.length} symbol(s)? Searches src/ and compares to documented behaviour.`,
                )
              ) {
                auditMutation.mutate();
              }
            }}
          >
            {auditMutation.isPending ? "Queueing…" : "Audit Rust implementation"}
          </button>
        </div>
        {auditMutation.isSuccess && (
          <p style={{ color: "#86efac", fontSize: "0.85rem", marginTop: "0.5rem" }}>
            Queued {auditMutation.data.batches} job(s) for {auditMutation.data.totalTasks} symbols.
            Watch the Jobs page.
          </p>
        )}
        {auditMutation.isError && (
          <p style={{ color: "#f87171", fontSize: "0.85rem", marginTop: "0.5rem" }}>
            {(auditMutation.error as Error).message}
          </p>
        )}
      </div>

      {flow.flow.expected_behaviour && (
        <>
          <h3 style={{ marginBottom: "0.5rem" }}>Expected Behaviour</h3>
          <p style={{ marginBottom: "1.5rem" }}>{flow.flow.expected_behaviour}</p>
        </>
      )}

      <p style={{ color: "#94a3b8", fontSize: "0.85rem", marginBottom: "1rem" }}>
        {documented}/{flow.tasks.length} symbols documented. Audit reports land in{" "}
        <code>tools/port-harness/docs/audits/</code> and task Notes.
      </p>

      <h3 style={{ marginBottom: "0.75rem" }}>Symbols ({flow.tasks.length})</h3>
      <table>
        <thead>
          <tr>
            <th>Symbol</th>
            <th>Status</th>
            <th>Rust target</th>
            <th>Notes</th>
          </tr>
        </thead>
        <tbody>
          {flow.tasks.map((t) => (
            <tr key={t.id}>
              <td>{t.symbol_name}</td>
              <td>
                <StatusBadge status={t.status} />
              </td>
              <td style={{ fontSize: "0.8rem" }}>{t.target_rust_file ?? "—"}</td>
              <td style={{ fontSize: "0.8rem", color: "#94a3b8", maxWidth: 360 }}>
                {t.notes?.slice(0, 120)}
                {t.notes && t.notes.length > 120 ? "…" : ""}
              </td>
            </tr>
          ))}
        </tbody>
      </table>

      {flow.branches.length > 0 && (
        <>
          <h3 style={{ margin: "1.5rem 0 0.75rem" }}>Branches</h3>
          <table>
            <thead>
              <tr>
                <th>Condition</th>
                <th>Behaviour</th>
                <th>Source</th>
              </tr>
            </thead>
            <tbody>
              {flow.branches.map((b, i) => (
                <tr key={i}>
                  <td>{b.condition}</td>
                  <td>{b.behaviour}</td>
                  <td>
                    {b.file}:{b.start_line}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </>
      )}

      {flow.mutations.length > 0 && (
        <>
          <h3 style={{ margin: "1.5rem 0 0.75rem" }}>State Mutations</h3>
          <table>
            <thead>
              <tr>
                <th>Variable</th>
                <th>Description</th>
              </tr>
            </thead>
            <tbody>
              {flow.mutations.map((m, i) => (
                <tr key={i}>
                  <td>{m.variable_or_field}</td>
                  <td>{m.mutation_description}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </>
      )}
    </div>
  );
}

import { useState } from "react";
import { Link } from "react-router-dom";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { api, type FlowSummary } from "../api/client";

function truncate(text: string | null, max = 80): string {
  if (!text) return "";
  return text.length > max ? `${text.slice(0, max)}…` : text;
}

const STAGE_LABELS: Record<FlowSummary["progress"]["stage"], string> = {
  empty: "No symbols",
  audit: "Needs audit",
  plan: "Needs plan",
  port: "Needs port",
  review: "Needs review",
  done: "Done",
};

function FlowProgressCell({ progress }: { progress: FlowSummary["progress"] }) {
  const { stage, percent, total, audited, planned, ported, done } = progress;

  if (total === 0) {
    return <span style={{ color: "#94a3b8", fontSize: "0.85rem" }}>—</span>;
  }

  return (
    <div className="flow-progress-cell">
      <div className="flow-progress-header">
        <span className={`flow-stage-badge flow-stage-${stage}`}>
          {STAGE_LABELS[stage]}
        </span>
        <span className="flow-progress-percent">{percent}%</span>
      </div>
      <div className="job-progress flow-progress-bar-track">
        <div className="job-progress-bar" style={{ width: `${percent}%` }} />
      </div>
      <div className="flow-progress-stats">
        <span title="Audited">{audited}/{total} audited</span>
        <span title="Planned">{planned} planned</span>
        <span title="Ported">{ported} ported</span>
        {done > 0 && <span title="Done">{done} done</span>}
      </div>
    </div>
  );
}

export function Flows() {
  const [q, setQ] = useState("");
  const queryClient = useQueryClient();

  const { data, isLoading } = useQuery({
    queryKey: ["flows"],
    queryFn: api.getFlows,
    refetchInterval: 10000,
  });

  const markDoneMutation = useMutation({
    mutationFn: (flowId: number) => api.markFlowDone(flowId),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["flows"] });
      queryClient.invalidateQueries({ queryKey: ["tasks"] });
    },
  });

  const flows = (data ?? []).filter((f) =>
    !q || f.name.toLowerCase().includes(q.toLowerCase()),
  );

  if (isLoading) return <div>Loading...</div>;

  return (
    <div>
      <div className="page-header">
        <h2>Business Flows</h2>
        <p className="page-subtitle">
          Flows group related symbols into behavioural units for migration planning.
        </p>
      </div>

      <div className="filters" style={{ marginBottom: "1rem" }}>
        <input
          type="search"
          placeholder="Filter by name…"
          value={q}
          onChange={(e) => setQ(e.target.value)}
          style={{ minWidth: 240 }}
        />
      </div>

      {flows.length === 0 ? (
        <p style={{ color: "#94a3b8" }}>
          No flows yet. Index a file and run &quot;Assemble flows&quot; from the Files tab.
        </p>
      ) : (
        <table>
          <thead>
            <tr>
              <th>Name</th>
              <th>Progress</th>
              <th>Description</th>
              <th>Source file</th>
              <th>Risk</th>
              <th>Symbols</th>
              <th></th>
            </tr>
          </thead>
          <tbody>
            {flows.map((f) => {
              const remaining = f.progress.total - f.progress.done;
              const canMarkDone =
                f.progress.total > 0 && f.progress.stage !== "done" && remaining > 0;

              return (
              <tr key={f.id}>
                <td>
                  <Link to={`/flows/${f.id}`}>{f.name}</Link>
                </td>
                <td style={{ minWidth: 220 }}>
                  <FlowProgressCell progress={f.progress} />
                </td>
                <td style={{ fontSize: "0.85rem", maxWidth: 280 }}>
                  {truncate(f.description)}
                </td>
                <td style={{ fontSize: "0.85rem" }}>{f.source_file ?? "—"}</td>
                <td>
                  <span className={`status-badge status-${f.risk_level}`}>
                    {f.risk_level}
                  </span>
                </td>
                <td>{f.symbol_count}</td>
                <td>
                  {canMarkDone && (
                    <button
                      type="button"
                      className="btn btn-sm btn-secondary"
                      disabled={markDoneMutation.isPending}
                      onClick={() => {
                        if (
                          window.confirm(
                            `Mark all incomplete symbols in "${f.name}" as done?`,
                          )
                        ) {
                          markDoneMutation.mutate(f.id);
                        }
                      }}
                    >
                      Done
                    </button>
                  )}
                </td>
              </tr>
              );
            })}
          </tbody>
        </table>
      )}
    </div>
  );
}

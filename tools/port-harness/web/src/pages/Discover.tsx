import { useState, useEffect } from "react";
import { Link } from "react-router-dom";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { api, type DiscoverResult } from "../api/client";
import { StatusBadge } from "../components/StatusBadge";

function RelevanceBadge({ relevance }: { relevance: string }) {
  const colors: Record<string, string> = {
    high: "#ef4444",
    medium: "#f59e0b",
    low: "#6b7280",
  };
  return (
    <span
      className="badge"
      style={{ background: `${colors[relevance] ?? "#374151"}33`, color: colors[relevance] ?? "#9ca3af" }}
    >
      {relevance}
    </span>
  );
}

export function Discover() {
  const [query, setQuery] = useState("");
  const [activeId, setActiveId] = useState<number | null>(null);
  const queryClient = useQueryClient();

  const { data: history } = useQuery({
    queryKey: ["investigations"],
    queryFn: () => api.listInvestigations(),
    refetchInterval: 5000,
  });

  const { data: detail, isLoading: detailLoading } = useQuery({
    queryKey: ["investigation", activeId],
    queryFn: () => api.getInvestigation(activeId!),
    enabled: activeId != null,
    refetchInterval: (q) => {
      const status = q.state.data?.investigation.status;
      return status === "running" ? 2000 : false;
    },
  });

  const startMutation = useMutation({
    mutationFn: (q: string) => api.startDiscovery(q),
    onSuccess: (data) => {
      setActiveId(data.investigationId);
      queryClient.invalidateQueries({ queryKey: ["investigations"] });
      queryClient.invalidateQueries({ queryKey: ["jobs"] });
    },
  });

  const [indexMessage, setIndexMessage] = useState<string | null>(null);

  const indexDirMutation = useMutation({
    mutationFn: () => api.indexDirectory("src/game"),
    onSuccess: (data) => {
      queryClient.invalidateQueries({ queryKey: ["files"] });
      queryClient.invalidateQueries({ queryKey: ["stats"] });
      queryClient.invalidateQueries({ queryKey: ["jobs"] });
      setIndexMessage(`Index job #${data.jobId} queued for ${data.dir}. See Jobs tab.`);
    },
  });

  const actionMutation = useMutation({
    mutationFn: (args: {
      id: number;
      action: "index" | "extract" | "verify";
      paths?: string[];
      taskIds?: number[];
    }) => api.discoverActions(args.id, { action: args.action, paths: args.paths, taskIds: args.taskIds }),
    onSuccess: (data) => {
      queryClient.invalidateQueries({ queryKey: ["jobs"] });
      queryClient.invalidateQueries({ queryKey: ["tasks"] });
      if (data.jobId) {
        setIndexMessage(`Index job #${data.jobId} queued. See Jobs tab.`);
      }
    },
  });

  useEffect(() => {
    if (!activeId && history?.length) {
      setActiveId(history[0]!.id);
    }
  }, [history, activeId]);

  const result: DiscoverResult | null = detail?.result ?? null;
  const isRunning = detail?.investigation.status === "running";

  const undocumentedTaskIds =
    result?.candidates
      .filter((c) => c.in_index && c.task_id && c.task_status === "discovered")
      .map((c) => c.task_id!)
      .filter(Boolean) ?? [];

  const verifyTaskIds =
    result?.candidates
      .filter((c) => c.in_index && c.task_id && c.suggested_action === "verify")
      .map((c) => c.task_id!)
      .filter(Boolean) ?? [];

  const unindexedPaths = result?.unindexed_files.map((f) => f.path) ?? [];

  return (
    <div className="discover-layout">
      <div className="discover-main">
        <div className="page-header">
          <h2>Discover</h2>
          <p className="page-subtitle">
            Describe a bug or missing behavior. The agent searches indexed symbols, claims, flows,
            and the reference C++ tree to find what needs porting or verification.
          </p>
        </div>

        <div className="info-banner">
          <span>
            Index more files first for better DB seed hits. One-time bulk index of game core helps
            discovery find relevant symbols faster.
          </span>
          <button
            type="button"
            className="btn btn-sm"
            disabled={indexDirMutation.isPending}
            onClick={() => indexDirMutation.mutate()}
          >
            {indexDirMutation.isPending ? "Indexing…" : "Index src/game"}
          </button>
        </div>

        {indexMessage && (
          <div className="info-banner" style={{ marginTop: "-0.5rem" }}>
            {indexMessage}
            <button type="button" className="btn-link" onClick={() => setIndexMessage(null)}>
              dismiss
            </button>
          </div>
        )}

        <div className="discover-form">
          <textarea
            className="discover-textarea"
            placeholder='e.g. "When I talk to an NPC, a quest should be available but it isn&apos;t"'
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            rows={4}
          />
          <button
            type="button"
            className="btn"
            disabled={!query.trim() || startMutation.isPending}
            onClick={() => startMutation.mutate(query.trim())}
          >
            {startMutation.isPending ? "Starting…" : "Investigate"}
          </button>
        </div>

        {startMutation.isError && (
          <div className="warning-banner">
            {(startMutation.error as Error).message}
          </div>
        )}

        {activeId && detailLoading && !detail && <p style={{ color: "#94a3b8" }}>Loading…</p>}

        {isRunning && (
          <div className="info-banner">
            <span>Investigation running… Check Jobs for agent activity.</span>
            {detail?.job && (
              <Link to="/jobs">Job #{detail.job.id}</Link>
            )}
          </div>
        )}

        {detail?.investigation.status === "failed" && (
          <div className="warning-banner">Investigation failed. Check Jobs for details.</div>
        )}

        {result && (
          <div className="discover-results">
            <section className="discover-section">
              <h3>Hypothesis</h3>
              <p>{result.hypothesis}</p>
            </section>

            {result.areas.length > 0 && (
              <section className="discover-section">
                <h3>Areas Involved</h3>
                <ul className="discover-list">
                  {result.areas.map((a) => (
                    <li key={a.name}>
                      <strong>{a.name}</strong> — {a.description}
                    </li>
                  ))}
                </ul>
              </section>
            )}

            <section className="discover-section">
              <div className="discover-section-header">
                <h3>Candidates ({result.candidates.length})</h3>
                <div className="action-buttons">
                  {undocumentedTaskIds.length > 0 && (
                    <button
                      type="button"
                      className="btn btn-sm"
                      disabled={actionMutation.isPending}
                      onClick={() =>
                        actionMutation.mutate({
                          id: activeId!,
                          action: "extract",
                          taskIds: undocumentedTaskIds,
                        })
                      }
                    >
                      Queue extract ({undocumentedTaskIds.length})
                    </button>
                  )}
                  {verifyTaskIds.length > 0 && (
                    <button
                      type="button"
                      className="btn btn-sm btn-secondary"
                      disabled={actionMutation.isPending}
                      onClick={() =>
                        actionMutation.mutate({
                          id: activeId!,
                          action: "verify",
                          taskIds: verifyTaskIds,
                        })
                      }
                    >
                      Queue verify ({verifyTaskIds.length})
                    </button>
                  )}
                  {unindexedPaths.length > 0 && (
                    <button
                      type="button"
                      className="btn btn-sm btn-secondary"
                      disabled={actionMutation.isPending}
                      onClick={() =>
                        actionMutation.mutate({
                          id: activeId!,
                          action: "index",
                          paths: unindexedPaths,
                        })
                      }
                    >
                      Index unindexed ({unindexedPaths.length})
                    </button>
                  )}
                </div>
              </div>

              <table>
                <thead>
                  <tr>
                    <th>Symbol</th>
                    <th>File</th>
                    <th>Relevance</th>
                    <th>Status</th>
                    <th>Action</th>
                    <th>Reason</th>
                  </tr>
                </thead>
                <tbody>
                  {result.candidates.map((c, i) => (
                    <tr key={`${c.symbol}-${c.file}-${i}`}>
                      <td>
                        <code>{c.symbol}</code>
                        {c.flow_name && (
                          <div style={{ fontSize: "0.75rem", color: "#94a3b8" }}>{c.flow_name}</div>
                        )}
                      </td>
                      <td className="file-path">{c.file}</td>
                      <td>
                        <RelevanceBadge relevance={c.relevance} />
                      </td>
                      <td>
                        {c.task_status ? (
                          <StatusBadge status={c.task_status} />
                        ) : c.in_index ? (
                          "indexed"
                        ) : (
                          "—"
                        )}
                      </td>
                      <td>
                        <span className="badge" style={{ background: "#374151", color: "#d1d5db" }}>
                          {c.suggested_action}
                        </span>
                      </td>
                      <td style={{ maxWidth: 320 }}>{c.reason}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </section>

            {result.unindexed_files.length > 0 && (
              <section className="discover-section">
                <h3>Unindexed Files ({result.unindexed_files.length})</h3>
                <table>
                  <thead>
                    <tr>
                      <th>Path</th>
                      <th>Reason</th>
                      <th />
                    </tr>
                  </thead>
                  <tbody>
                    {result.unindexed_files.map((f) => (
                      <tr key={f.path}>
                        <td className="file-path">{f.path}</td>
                        <td>{f.reason}</td>
                        <td>
                          <button
                            type="button"
                            className="btn btn-sm"
                            disabled={actionMutation.isPending}
                            onClick={() =>
                              actionMutation.mutate({
                                id: activeId!,
                                action: "index",
                                paths: [f.path],
                              })
                            }
                          >
                            Index
                          </button>
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </section>
            )}

            {result.suggested_next_steps.length > 0 && (
              <section className="discover-section">
                <h3>Suggested Next Steps</h3>
                <ol className="discover-list">
                  {result.suggested_next_steps.map((step, i) => (
                    <li key={i}>{step}</li>
                  ))}
                </ol>
              </section>
            )}
          </div>
        )}
      </div>

      <aside className="discover-history">
        <h3>History</h3>
        {!history?.length && <p style={{ color: "#94a3b8", fontSize: "0.875rem" }}>No investigations yet.</p>}
        <ul className="discover-history-list">
          {history?.map((inv) => (
            <li key={inv.id}>
              <button
                type="button"
                className={`discover-history-item${activeId === inv.id ? " active" : ""}`}
                onClick={() => setActiveId(inv.id)}
              >
                <span className={`status-badge status-${inv.status}`}>{inv.status}</span>
                <span className="discover-history-query">{inv.query}</span>
                <span className="discover-history-meta">
                  {inv.candidate_count > 0 && `${inv.candidate_count} candidates · `}
                  {new Date(inv.created_at).toLocaleString()}
                </span>
              </button>
            </li>
          ))}
        </ul>
      </aside>
    </div>
  );
}

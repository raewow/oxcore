import { useState } from "react";
import { Link, useParams } from "react-router-dom";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { api, type DiscoverCandidate, type FeatureInvestigationSummary } from "../api/client";
import { StatusBadge } from "../components/StatusBadge";

function formatBytes(n: number): string {
  if (n < 1024) return `${n} B`;
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KB`;
  return `${(n / (1024 * 1024)).toFixed(1)} MB`;
}

function progressPct(file: { symbol_count: number; documented: number }): number {
  if (!file.symbol_count) return 0;
  return Math.round((file.documented / file.symbol_count) * 100);
}

const RISK_COLOR: Record<string, string> = {
  critical: "#ef4444",
  high: "#f97316",
  medium: "#eab308",
  low: "#22c55e",
};

function InvestigationPanel({
  featureId,
  inv,
  onAddToFeature,
}: {
  featureId: number;
  inv: FeatureInvestigationSummary;
  onAddToFeature: (file: string) => void;
}) {
  const [expanded, setExpanded] = useState(false);
  const indexMutation = useMutation({
    mutationFn: (paths: string[]) => api.discoverActions(inv.id, { action: "index", paths }),
  });
  const extractMutation = useMutation({
    mutationFn: (taskIds: number[]) => api.discoverActions(inv.id, { action: "extract", taskIds }),
  });

  const isRunning = inv.status === "running";

  return (
    <div style={{ border: "1px solid #334155", borderRadius: "6px", marginBottom: "0.5rem" }}>
      <div
        style={{ padding: "0.6rem 0.75rem", cursor: "pointer", display: "flex", alignItems: "center", gap: "0.75rem" }}
        onClick={() => setExpanded((v) => !v)}
      >
        <span style={{ fontSize: "0.75rem", color: isRunning ? "#38bdf8" : inv.status === "done" ? "#4ade80" : "#f87171" }}>
          {isRunning ? "⟳ running" : inv.status === "done" ? "✓" : "✗"}
        </span>
        <span style={{ flex: 1, fontSize: "0.85rem" }}>{inv.query}</span>
        {inv.candidate_count > 0 && (
          <span className="muted" style={{ fontSize: "0.75rem" }}>{inv.candidate_count} candidates</span>
        )}
        <span className="muted" style={{ fontSize: "0.75rem" }}>{expanded ? "▲" : "▼"}</span>
      </div>

      {expanded && inv.candidates.length > 0 && (
        <div style={{ borderTop: "1px solid #1e293b", padding: "0.5rem 0.75rem" }}>
          {inv.hypothesis && (
            <p style={{ fontSize: "0.8rem", color: "#94a3b8", marginBottom: "0.5rem", fontStyle: "italic" }}>
              {inv.hypothesis}
            </p>
          )}
          <table style={{ width: "100%", fontSize: "0.8rem" }}>
            <thead>
              <tr>
                <th style={{ textAlign: "left", padding: "0.25rem 0.5rem 0.25rem 0" }}>Symbol</th>
                <th style={{ textAlign: "left", padding: "0.25rem 0.5rem" }}>File</th>
                <th style={{ textAlign: "left", padding: "0.25rem 0.5rem" }}>Relevance</th>
                <th style={{ textAlign: "left", padding: "0.25rem 0" }}>Actions</th>
              </tr>
            </thead>
            <tbody>
              {inv.candidates.map((c: DiscoverCandidate, i) => (
                <tr key={i}>
                  <td style={{ padding: "0.2rem 0.5rem 0.2rem 0", fontFamily: "monospace" }}>{c.symbol}</td>
                  <td style={{ padding: "0.2rem 0.5rem", color: "#94a3b8", maxWidth: 200, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
                    {c.file}
                  </td>
                  <td style={{ padding: "0.2rem 0.5rem" }}>
                    <span style={{ color: c.relevance === "high" ? "#4ade80" : c.relevance === "medium" ? "#eab308" : "#94a3b8" }}>
                      {c.relevance}
                    </span>
                  </td>
                  <td style={{ padding: "0.2rem 0", display: "flex", gap: "0.3rem", flexWrap: "wrap" }}>
                    <button
                      className="btn btn-secondary btn-sm"
                      style={{ fontSize: "0.7rem", padding: "0.15rem 0.4rem" }}
                      onClick={(e) => { e.stopPropagation(); onAddToFeature(c.file); }}
                    >
                      + feature
                    </button>
                    {!c.in_index && (
                      <button
                        className="btn btn-secondary btn-sm"
                        style={{ fontSize: "0.7rem", padding: "0.15rem 0.4rem" }}
                        disabled={indexMutation.isPending}
                        onClick={(e) => { e.stopPropagation(); indexMutation.mutate([c.file]); }}
                      >
                        Index
                      </button>
                    )}
                    {c.task_id && (
                      <button
                        className="btn btn-secondary btn-sm"
                        style={{ fontSize: "0.7rem", padding: "0.15rem 0.4rem" }}
                        disabled={extractMutation.isPending}
                        onClick={(e) => { e.stopPropagation(); extractMutation.mutate([c.task_id!]); }}
                      >
                        Extract
                      </button>
                    )}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </div>
  );
}

export function FeatureDetail() {
  const { id } = useParams<{ id: string }>();
  const featureId = parseInt(id ?? "0", 10);
  const queryClient = useQueryClient();
  const [discoveryQuery, setDiscoveryQuery] = useState("");
  const [showTasks, setShowTasks] = useState(false);

  const { data, isLoading, error } = useQuery({
    queryKey: ["feature", featureId],
    queryFn: () => api.getFeature(featureId),
    enabled: featureId > 0,
  });

  const { data: investigations, refetch: refetchInvestigations } = useQuery({
    queryKey: ["feature-investigations", featureId],
    queryFn: () => api.getFeatureInvestigations(featureId),
    enabled: featureId > 0,
    refetchInterval: (query) => {
      const d = query.state.data;
      return Array.isArray(d) && d.some((i) => i.status === "running") ? 5000 : false;
    },
  });

  const invalidate = () => {
    queryClient.invalidateQueries({ queryKey: ["feature", featureId] });
    queryClient.invalidateQueries({ queryKey: ["features"] });
  };

  const refreshMutation = useMutation({ mutationFn: () => api.refreshFeatureSuggestions(featureId), onSuccess: invalidate });
  const acceptMutation = useMutation({ mutationFn: api.acceptFeatureSuggestion, onSuccess: invalidate });
  const acceptAllMutation = useMutation({ mutationFn: () => api.acceptAllFeatureSuggestions(featureId), onSuccess: invalidate });
  const rejectMutation = useMutation({ mutationFn: api.rejectFeatureSuggestion, onSuccess: invalidate });

  const discoveryMutation = useMutation({
    mutationFn: (q: string) => api.startDiscovery(q, { featureId }),
    onSuccess: () => {
      setDiscoveryQuery("");
      refetchInvestigations();
      setTimeout(() => refetchInvestigations(), 3000);
    },
  });

  const bulkIndexMutation = useMutation({
    mutationFn: () => api.indexAllForFeature(featureId),
    onSuccess: invalidate,
  });
  const bulkDocumentMutation = useMutation({
    mutationFn: () => api.documentAllForFeature(featureId),
    onSuccess: invalidate,
  });
  const bulkFlowsMutation = useMutation({
    mutationFn: () => api.assembleFlowsForFeature(featureId),
    onSuccess: invalidate,
  });

  const auditFlowMutation = useMutation({
    mutationFn: (flowId: number) => api.auditFlow(flowId),
  });
  const planFlowMutation = useMutation({
    mutationFn: (flowId: number) => api.planFlow(flowId),
  });
  const portFlowMutation = useMutation({
    mutationFn: (flowId: number) => api.portFlow(flowId),
  });

  const addToFeatureMutation = useMutation({
    mutationFn: (file: string) => api.assignFeature(featureId, "file", file),
    onSuccess: invalidate,
  });

  const indexFileMutation = useMutation({
    mutationFn: (path: string) => api.indexFile(path),
    onSuccess: invalidate,
  });
  const documentFileMutation = useMutation({
    mutationFn: (path: string) => api.documentFile(path),
    onSuccess: invalidate,
  });
  const assembleFlowsFileMutation = useMutation({
    mutationFn: (path: string) => api.assembleFlowsForFile(path),
    onSuccess: invalidate,
  });
  const pipelineFileMutation = useMutation({
    mutationFn: (path: string) => api.runFilePipeline(path),
    onSuccess: invalidate,
  });

  if (isLoading) return <div>Loading feature...</div>;
  if (error) return <div className="warning-banner">{(error as Error).message}</div>;
  if (!data) return <div>Feature not found.</div>;

  const unindexedCount = data.file_status.filter((f) => !f.indexed && f.kind === "cpp").length;
  const discoveredCount = data.file_status.reduce((sum, f) => sum + f.discovered, 0);
  const indexedCppCount = data.file_status.filter((f) => f.indexed && f.kind === "cpp").length;

  return (
    <div>
      {/* Header */}
      <div className="page-header">
        <h2>{data.feature.name}</h2>
        <p className="page-subtitle">{data.feature.description}</p>
        <button className="btn btn-secondary btn-sm" onClick={() => refreshMutation.mutate()}>
          Refresh suggestions
        </button>
      </div>

      {/* Discovery panel */}
      <section className="content-section">
        <div className="section-title">Discovery</div>
        <p className="muted" style={{ fontSize: "0.82rem", marginBottom: "0.75rem" }}>
          Ask about this feature's game logic. Results are seeded from this feature's files and the reference C++ codebase.
        </p>
        <div style={{ display: "flex", gap: "0.5rem", alignItems: "flex-start" }}>
          <textarea
            style={{ flex: 1, minHeight: "60px", resize: "vertical", fontFamily: "inherit", fontSize: "0.85rem", padding: "0.5rem", background: "#0f172a", color: "#e2e8f0", border: "1px solid #334155", borderRadius: "4px" }}
            placeholder="e.g. how does loot rolling decide rarity for boss kills?"
            value={discoveryQuery}
            onChange={(e) => setDiscoveryQuery(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter" && (e.ctrlKey || e.metaKey) && discoveryQuery.trim()) {
                discoveryMutation.mutate(discoveryQuery.trim());
              }
            }}
          />
          <button
            className="btn btn-sm"
            disabled={!discoveryQuery.trim() || discoveryMutation.isPending}
            onClick={() => discoveryMutation.mutate(discoveryQuery.trim())}
          >
            {discoveryMutation.isPending ? "Starting…" : "Investigate"}
          </button>
        </div>
        {discoveryMutation.isError && (
          <p style={{ color: "#f87171", fontSize: "0.8rem", marginTop: "0.4rem" }}>
            {(discoveryMutation.error as Error).message}
          </p>
        )}

        {investigations && investigations.length > 0 && (
          <div style={{ marginTop: "1rem" }}>
            {investigations.map((inv) => (
              <InvestigationPanel
                key={inv.id}
                featureId={featureId}
                inv={inv}
                onAddToFeature={(file) => addToFeatureMutation.mutate(file)}
              />
            ))}
          </div>
        )}
      </section>

      {/* Bulk action toolbar */}
      <section className="content-section">
        <div className="section-title">Bulk actions</div>
        <div style={{ display: "flex", gap: "0.5rem", flexWrap: "wrap" }}>
          <button
            className="btn btn-secondary btn-sm"
            disabled={bulkIndexMutation.isPending || unindexedCount === 0}
            onClick={() => bulkIndexMutation.mutate()}
          >
            {bulkIndexMutation.isPending ? "Indexing…" : `Index all files${unindexedCount > 0 ? ` (${unindexedCount} unindexed)` : " ✓"}`}
          </button>
          <button
            className="btn btn-secondary btn-sm"
            disabled={bulkDocumentMutation.isPending || discoveredCount === 0}
            onClick={() => bulkDocumentMutation.mutate()}
          >
            {bulkDocumentMutation.isPending ? "Queuing…" : `Document all${discoveredCount > 0 ? ` (${discoveredCount} todo)` : " ✓"}`}
          </button>
          <button
            className="btn btn-secondary btn-sm"
            disabled={bulkFlowsMutation.isPending || indexedCppCount === 0}
            onClick={() => bulkFlowsMutation.mutate()}
          >
            {bulkFlowsMutation.isPending ? "Queuing…" : `Assemble flows (${indexedCppCount} files)`}
          </button>
        </div>
        {bulkIndexMutation.isSuccess && bulkIndexMutation.data?.message && (
          <p className="muted" style={{ fontSize: "0.8rem", marginTop: "0.4rem" }}>{bulkIndexMutation.data.message}</p>
        )}
      </section>

      {/* Flows — primary work surface */}
      <section className="content-section">
        <div className="section-title">Flows</div>
        {data.flows.length === 0 ? (
          <p className="muted">No linked flows. Assemble flows from the files below, or accept suggestions.</p>
        ) : (
          <table>
            <thead>
              <tr>
                <th>Name</th>
                <th>Risk</th>
                <th>Actions</th>
              </tr>
            </thead>
            <tbody>
              {data.flows.map((flow) => (
                <tr key={flow.id}>
                  <td>
                    <Link to={`/flows/${flow.id}`}>{flow.name}</Link>
                  </td>
                  <td>
                    {flow.risk_level ? (
                      <span style={{ color: RISK_COLOR[flow.risk_level] ?? "#94a3b8", textTransform: "capitalize", fontSize: "0.8rem" }}>
                        {flow.risk_level}
                      </span>
                    ) : "—"}
                  </td>
                  <td>
                    <div className="action-buttons">
                      <button
                        className="btn btn-secondary btn-sm"
                        disabled={auditFlowMutation.isPending}
                        onClick={() => auditFlowMutation.mutate(flow.id)}
                      >
                        Audit
                      </button>
                      <button
                        className="btn btn-secondary btn-sm"
                        disabled={planFlowMutation.isPending}
                        onClick={() => planFlowMutation.mutate(flow.id)}
                      >
                        Plan
                      </button>
                      <button
                        className="btn btn-secondary btn-sm"
                        disabled={portFlowMutation.isPending}
                        onClick={() => portFlowMutation.mutate(flow.id)}
                      >
                        Port
                      </button>
                    </div>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </section>

      {/* Stats */}
      <div className="stats-grid">
        <div className="stat-card">
          <div className="label">flows</div>
          <div className="value">{data.flows.length}</div>
        </div>
        <div className="stat-card">
          <div className="label">files</div>
          <div className="value">{data.files.length}</div>
        </div>
        <div className="stat-card">
          <div className="label">done</div>
          <div className="value">{data.stats.done}</div>
        </div>
        <div className="stat-card">
          <div className="label">blocked</div>
          <div className="value">{data.stats.blocked}</div>
        </div>
      </div>

      {/* Suggestions */}
      <section className="content-section">
        <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", gap: "1rem" }}>
          <div className="section-title">Suggestions</div>
          {data.suggestions.filter((s) => s.status === "pending").length > 0 && (
            <button
              type="button"
              className="btn btn-sm"
              disabled={acceptAllMutation.isPending}
              onClick={() => acceptAllMutation.mutate()}
            >
              Accept all
            </button>
          )}
        </div>
        {data.suggestions.filter((s) => s.status === "pending").length ? (
          <table>
            <tbody>
              {data.suggestions
                .filter((s) => s.status === "pending")
                .map((suggestion) => (
                  <tr key={suggestion.id}>
                    <td>{suggestion.target_type}</td>
                    <td>{suggestion.target_id}</td>
                    <td>{suggestion.reason}</td>
                    <td>{suggestion.confidence}</td>
                    <td>
                      <div className="action-buttons">
                        <button className="btn btn-sm" onClick={() => acceptMutation.mutate(suggestion.id)}>
                          Accept
                        </button>
                        <button
                          className="btn btn-secondary btn-sm"
                          onClick={() => rejectMutation.mutate(suggestion.id)}
                        >
                          Reject
                        </button>
                      </div>
                    </td>
                  </tr>
                ))}
            </tbody>
          </table>
        ) : (
          <p className="muted">No pending suggestions.</p>
        )}
      </section>

      {/* Files */}
      <section className="content-section">
        <div className="section-title">Files</div>
        <table>
          <thead>
            <tr>
              <th>Path</th>
              <th>Size</th>
              <th>Symbols</th>
              <th>Progress</th>
              <th>Flows</th>
              <th>Actions</th>
            </tr>
          </thead>
          <tbody>
            {data.file_status.map((file) => (
              <tr key={file.path}>
                <td>
                  <Link to={`/files/detail?path=${encodeURIComponent(file.path)}`}>
                    <code className="file-path">{file.path}</code>
                  </Link>
                  {!file.indexed && file.kind === "cpp" && (
                    <span className="badge badge-discovered" style={{ marginLeft: 8 }}>
                      not indexed
                    </span>
                  )}
                </td>
                <td>{formatBytes(file.size_bytes)}</td>
                <td>
                  {file.indexed ? (
                    <>
                      {file.documented}/{file.symbol_count}
                      {file.discovered > 0 && (
                        <span style={{ color: "#94a3b8" }}> ({file.discovered} todo)</span>
                      )}
                      {file.blocked > 0 && (
                        <span style={{ color: "#f87171" }}> ({file.blocked} blocked)</span>
                      )}
                    </>
                  ) : "—"}
                </td>
                <td style={{ minWidth: 100 }}>
                  {file.indexed ? (
                    <>
                      {progressPct(file)}%
                      <div className="job-progress">
                        <div className="job-progress-bar" style={{ width: `${progressPct(file)}%` }} />
                      </div>
                    </>
                  ) : "—"}
                </td>
                <td>{file.indexed ? file.flow_count : "—"}</td>
                <td>
                  <div className="action-buttons">
                    {!file.indexed && file.kind === "cpp" && (
                      <button
                        className="btn btn-secondary btn-sm"
                        disabled={indexFileMutation.isPending}
                        onClick={() => indexFileMutation.mutate(file.path)}
                      >
                        Index
                      </button>
                    )}
                    {file.indexed && file.discovered > 0 && (
                      <button
                        className="btn btn-secondary btn-sm"
                        disabled={documentFileMutation.isPending}
                        onClick={() => documentFileMutation.mutate(file.path)}
                      >
                        Document
                      </button>
                    )}
                    {file.indexed && file.kind === "cpp" && (
                      <button
                        className="btn btn-secondary btn-sm"
                        disabled={assembleFlowsFileMutation.isPending}
                        onClick={() => assembleFlowsFileMutation.mutate(file.path)}
                      >
                        Flows
                      </button>
                    )}
                    {file.kind === "cpp" && (
                      <button
                        className="btn btn-secondary btn-sm"
                        disabled={pipelineFileMutation.isPending}
                        onClick={() => pipelineFileMutation.mutate(file.path)}
                      >
                        Pipeline
                      </button>
                    )}
                  </div>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </section>

      {/* Dependency graph */}
      <section className="content-section">
        <div className="section-title">Dependency graph</div>
        {data.dependency_graph.edges.length ? (
          <table>
            <thead>
              <tr>
                <th>From</th>
                <th>To</th>
                <th>Edges</th>
              </tr>
            </thead>
            <tbody>
              {data.dependency_graph.edges.map((edge) => (
                <tr key={`${edge.from}-${edge.to}`}>
                  <td>
                    <Link to={`/files/detail?path=${encodeURIComponent(edge.from)}`}>{edge.from}</Link>
                  </td>
                  <td>
                    <Link to={`/files/detail?path=${encodeURIComponent(edge.to)}`}>{edge.to}</Link>
                  </td>
                  <td>{edge.count}</td>
                </tr>
              ))}
            </tbody>
          </table>
        ) : (
          <p className="muted">No file dependency edges found.</p>
        )}
      </section>

      {/* Tasks — collapsed by default */}
      <section className="content-section">
        <div style={{ display: "flex", alignItems: "center", gap: "1rem" }}>
          <div className="section-title" style={{ marginBottom: 0 }}>Tasks</div>
          <span className="muted" style={{ fontSize: "0.8rem" }}>
            {data.stats.total} total &middot; {data.stats.done} done
            {data.stats.blocked > 0 && <span style={{ color: "#f87171" }}> &middot; {data.stats.blocked} blocked</span>}
          </span>
          <button
            className="btn btn-secondary btn-sm"
            style={{ marginLeft: "auto" }}
            onClick={() => setShowTasks((v) => !v)}
          >
            {showTasks ? "Hide tasks" : "Show all tasks"}
          </button>
        </div>

        {showTasks && (
          <table style={{ marginTop: "0.75rem" }}>
            <thead>
              <tr>
                <th>Symbol</th>
                <th>Status</th>
                <th>File</th>
                <th>Rust target</th>
              </tr>
            </thead>
            <tbody>
              {data.tasks.map((task) => (
                <tr key={task.id}>
                  <td>
                    <Link to={`/symbols/${task.source_symbol_id}`}>{task.symbol_name}</Link>
                  </td>
                  <td>
                    <StatusBadge status={task.status} />
                  </td>
                  <td>
                    <Link to={`/files/detail?path=${encodeURIComponent(task.symbol_file)}`}>{task.symbol_file}</Link>
                  </td>
                  <td>{task.target_rust_file ?? "—"}</td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </section>
    </div>
  );
}

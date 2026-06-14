import { Link, useParams } from "react-router-dom";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { api } from "../api/client";
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

export function FeatureDetail() {
  const { id } = useParams<{ id: string }>();
  const featureId = parseInt(id ?? "0", 10);
  const queryClient = useQueryClient();
  const { data, isLoading, error } = useQuery({
    queryKey: ["feature", featureId],
    queryFn: () => api.getFeature(featureId),
    enabled: featureId > 0,
  });

  const invalidate = () => {
    queryClient.invalidateQueries({ queryKey: ["feature", featureId] });
    queryClient.invalidateQueries({ queryKey: ["features"] });
  };
  const refreshMutation = useMutation({ mutationFn: () => api.refreshFeatureSuggestions(featureId), onSuccess: invalidate });
  const acceptMutation = useMutation({ mutationFn: api.acceptFeatureSuggestion, onSuccess: invalidate });
  const acceptAllMutation = useMutation({ mutationFn: () => api.acceptAllFeatureSuggestions(featureId), onSuccess: invalidate });
  const rejectMutation = useMutation({ mutationFn: api.rejectFeatureSuggestion, onSuccess: invalidate });

  if (isLoading) return <div>Loading feature...</div>;
  if (error) return <div className="warning-banner">{(error as Error).message}</div>;
  if (!data) return <div>Feature not found.</div>;

  return (
    <div>
      <div className="page-header">
        <h2>{data.feature.name}</h2>
        <p className="page-subtitle">{data.feature.description}</p>
        <button className="btn btn-secondary btn-sm" onClick={() => refreshMutation.mutate()}>
          Refresh suggestions
        </button>
      </div>

      <div className="stats-grid">
        <div className="stat-card">
          <div className="label">tasks</div>
          <div className="value">{data.stats.total}</div>
        </div>
        <div className="stat-card">
          <div className="label">done</div>
          <div className="value">{data.stats.done}</div>
        </div>
        <div className="stat-card">
          <div className="label">blocked</div>
          <div className="value">{data.stats.blocked}</div>
        </div>
        <div className="stat-card">
          <div className="label">files</div>
          <div className="value">{data.files.length}</div>
        </div>
      </div>

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
                  ) : (
                    "—"
                  )}
                </td>
                <td style={{ minWidth: 120 }}>
                  {file.indexed ? (
                    <>
                      {progressPct(file)}%
                      <div className="job-progress">
                        <div
                          className="job-progress-bar"
                          style={{ width: `${progressPct(file)}%` }}
                        />
                      </div>
                    </>
                  ) : (
                    "—"
                  )}
                </td>
                <td>{file.indexed ? file.flow_count : "—"}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </section>

      <section className="content-section">
        <div className="section-title">Flows</div>
        <div className="link-cloud">
          {data.flows.map((flow) => (
            <Link key={flow.id} className="pill" to={`/flows/${flow.id}`}>
              {flow.name}
            </Link>
          ))}
          {!data.flows.length && <span className="muted">No linked flows.</span>}
        </div>
      </section>

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

      <section className="content-section">
        <div className="section-title">Tasks</div>
        <table>
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
      </section>
    </div>
  );
}

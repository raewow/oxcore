import { Link, useSearchParams } from "react-router-dom";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { api } from "../api/client";
import { DependencyGraph } from "../components/DependencyGraph";
import { SourceViewer } from "../components/SourceViewer";
import { StatusBadge } from "../components/StatusBadge";

function pct(done: number, total: number): number {
  return total ? Math.round((done / total) * 100) : 0;
}

export function FileDetail() {
  const [params] = useSearchParams();
  const path = params.get("path") ?? "";
  const queryClient = useQueryClient();

  const { data, isLoading, error } = useQuery({
    queryKey: ["file-detail", path],
    queryFn: () => api.getFileDetail(path),
    enabled: Boolean(path),
    refetchInterval: 5000,
  });

  const invalidate = () => {
    queryClient.invalidateQueries({ queryKey: ["file-detail", path] });
    queryClient.invalidateQueries({ queryKey: ["files"] });
    queryClient.invalidateQueries({ queryKey: ["jobs"] });
    queryClient.invalidateQueries({ queryKey: ["stats"] });
  };

  const indexMutation = useMutation({ mutationFn: api.indexFile, onSuccess: invalidate });
  const documentMutation = useMutation({ mutationFn: (p: string) => api.documentFile(p), onSuccess: invalidate });
  const flowsMutation = useMutation({ mutationFn: api.assembleFlowsForFile, onSuccess: invalidate });
  const pipelineMutation = useMutation({ mutationFn: api.runFilePipeline, onSuccess: invalidate });

  if (!path) return <div>Select a file.</div>;
  if (isLoading) return <div>Loading file...</div>;
  if (error) return <div className="warning-banner">{(error as Error).message}</div>;
  if (!data) return <div>File not found.</div>;

  const progress = pct(data.file.documented, data.file.symbol_count);

  return (
    <div>
      <div className="page-header">
        <h2>{data.file.name}</h2>
        <p className="page-subtitle">{data.file.path}</p>
        <div className="action-buttons">
          {data.file.kind === "cpp" && (
            <>
              <button className="btn btn-secondary btn-sm" onClick={() => indexMutation.mutate(data.file.path)}>
                Index
              </button>
              <button
                className="btn btn-secondary btn-sm"
                disabled={!data.file.indexed}
                onClick={() => documentMutation.mutate(data.file.path)}
              >
                Document
              </button>
              <button
                className="btn btn-secondary btn-sm"
                disabled={!data.file.documented}
                onClick={() => flowsMutation.mutate(data.file.path)}
              >
                Flows
              </button>
              <button className="btn btn-sm" onClick={() => pipelineMutation.mutate(data.file.path)}>
                All
              </button>
            </>
          )}
          <Link className="btn btn-secondary btn-sm" to={`/tasks?file=${encodeURIComponent(data.file.name)}`}>
            Tasks
          </Link>
        </div>
      </div>

      <div className="stats-grid">
        <div className="stat-card">
          <div className="label">symbols</div>
          <div className="value">{data.file.symbol_count}</div>
        </div>
        <div className="stat-card">
          <div className="label">documented</div>
          <div className="value">{data.file.documented}</div>
        </div>
        <div className="stat-card">
          <div className="label">flows</div>
          <div className="value">{data.flows.length}</div>
        </div>
        <div className="stat-card">
          <div className="label">progress</div>
          <div className="value">{progress}%</div>
        </div>
      </div>

      <section className="content-section">
        <div className="section-title">Pipeline jobs</div>
        {data.jobs.length ? (
          <table>
            <tbody>
              {data.jobs.map((job) => (
                <tr key={job.id}>
                  <td>
                    <Link to="/jobs">#{job.id}</Link>
                  </td>
                  <td>{job.summary}</td>
                  <td>{job.status}</td>
                  <td>
                    {job.progress}/{job.total}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        ) : (
          <p className="muted">No recent jobs for this file.</p>
        )}
      </section>

      <section className="content-section">
        <div className="section-title">Features and flows</div>
        <div className="link-cloud">
          {data.features.map((feature) => (
            <Link key={feature.id} to={`/features/${feature.id}`} className="pill">
              {feature.name}
            </Link>
          ))}
          {data.flows.map((flow) => (
            <Link key={flow.id} to={`/flows/${flow.id}`} className="pill">
              {flow.name}
            </Link>
          ))}
          {!data.features.length && !data.flows.length && <span className="muted">No feature or flow links yet.</span>}
        </div>
      </section>

      <section className="content-section">
        <div className="section-title">File dependencies</div>
        <DependencyGraph dependencies={data.dependencies} />
      </section>

      <section className="content-section">
        <div className="section-title">Tasks</div>
        <table>
          <thead>
            <tr>
              <th>Symbol</th>
              <th>Status</th>
              <th>Flow</th>
              <th>Claims</th>
            </tr>
          </thead>
          <tbody>
            {data.tasks.map((task) => (
              <tr key={task.id}>
                <td>
                  <Link to={`/symbols/${task.source_symbol_id}`}>{task.symbol_name}</Link>
                  <div className="muted">
                    {task.start_line}-{task.end_line}
                  </div>
                </td>
                <td>
                  <StatusBadge status={task.status} />
                </td>
                <td>{task.flow_id ? <Link to={`/flows/${task.flow_id}`}>{task.flow_name}</Link> : "—"}</td>
                <td>{task.claim_count}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </section>

      {data.source_preview && (
        <section className="content-section">
          <div className="section-title">Source preview</div>
          <SourceViewer snippet={data.source_preview.text} baseLine={data.source_preview.start_line} />
        </section>
      )}
    </div>
  );
}

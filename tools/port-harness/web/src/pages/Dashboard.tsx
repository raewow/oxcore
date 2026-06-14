import { useQuery } from "@tanstack/react-query";
import { api } from "../api/client";
import { WorkingFiles } from "../components/WorkingFiles";

export function Dashboard() {
  const { data, isLoading } = useQuery({
    queryKey: ["stats"],
    queryFn: api.getStats,
    refetchInterval: 10000,
  });

  if (isLoading) return <div>Loading...</div>;
  if (!data) return <div>No data</div>;

  return (
    <div>
      <div className="page-header">
        <h2>Dashboard</h2>
      </div>

      {(data.warnings.blocked > 0 || data.warnings.missing_docs > 0) && (
        <div className="warning-banner">
          {data.warnings.missing_docs > 0 && (
            <span>{data.warnings.missing_docs} symbols need documentation. </span>
          )}
          {data.warnings.blocked > 0 && (
            <span>{data.warnings.blocked} tasks blocked.</span>
          )}
        </div>
      )}

      <WorkingFiles files={data.working_files ?? []} />

      <div className="stats-grid">
        {data.status_counts.map((s) => (
          <div key={s.status} className="stat-card">
            <div className="label">{s.status.replace(/_/g, " ")}</div>
            <div className="value">{s.count}</div>
          </div>
        ))}
      </div>

      <h3 style={{ marginBottom: "1rem" }}>File Progress</h3>
      <table>
        <thead>
          <tr>
            <th>File</th>
            <th>Documented</th>
            <th>Total</th>
            <th>Progress</th>
          </tr>
        </thead>
        <tbody>
          {data.file_progress.map((f) => (
            <tr key={f.file}>
              <td>{f.file}</td>
              <td>{f.documented}</td>
              <td>{f.total}</td>
              <td>
                <div className="job-progress" style={{ width: 120 }}>
                  <div
                    className="job-progress-bar"
                    style={{
                      width: `${f.total ? (f.documented / f.total) * 100 : 0}%`,
                    }}
                  />
                </div>
              </td>
            </tr>
          ))}
        </tbody>
      </table>

      <h3 style={{ margin: "2rem 0 1rem" }}>Recent Jobs</h3>
      <table>
        <thead>
          <tr>
            <th>ID</th>
            <th>Stage</th>
            <th>Status</th>
            <th>Progress</th>
          </tr>
        </thead>
        <tbody>
          {data.recent_jobs.map((j) => (
            <tr key={j.id}>
              <td>{j.id}</td>
              <td>{j.stage}</td>
              <td>{j.status}</td>
              <td>
                {j.progress}/{j.total}
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}

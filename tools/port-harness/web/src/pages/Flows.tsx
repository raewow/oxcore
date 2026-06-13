import { useState } from "react";
import { Link } from "react-router-dom";
import { useQuery } from "@tanstack/react-query";
import { api, type FlowSummary } from "../api/client";

function truncate(text: string | null, max = 80): string {
  if (!text) return "";
  return text.length > max ? `${text.slice(0, max)}…` : text;
}

export function Flows() {
  const [q, setQ] = useState("");

  const { data, isLoading } = useQuery({
    queryKey: ["flows"],
    queryFn: api.getFlows,
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
              <th>Description</th>
              <th>Source file</th>
              <th>Risk</th>
              <th>Symbols</th>
            </tr>
          </thead>
          <tbody>
            {flows.map((f) => (
              <tr key={f.id}>
                <td>
                  <Link to={`/flows/${f.id}`}>{f.name}</Link>
                </td>
                <td style={{ fontSize: "0.85rem", maxWidth: 320 }}>
                  {truncate(f.description)}
                </td>
                <td style={{ fontSize: "0.85rem" }}>{f.source_file ?? "—"}</td>
                <td>
                  <span className={`status-badge status-${f.risk_level}`}>
                    {f.risk_level}
                  </span>
                </td>
                <td>{f.symbol_count}</td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
    </div>
  );
}

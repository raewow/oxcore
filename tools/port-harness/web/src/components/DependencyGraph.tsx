import { Link } from "react-router-dom";
import type { FileDependencies } from "../api/client";

function FileLink({ file }: { file: string }) {
  return <Link to={`/files/detail?path=${encodeURIComponent(file)}`}>{file}</Link>;
}

function EdgeList({
  title,
  edges,
  direction,
}: {
  title: string;
  edges: FileDependencies["outbound"];
  direction: "in" | "out";
}) {
  return (
    <div className="dependency-column">
      <h4>{title}</h4>
      {edges.length === 0 ? (
        <p className="muted">No file dependencies found.</p>
      ) : (
        edges.map((edge) => (
          <div key={`${direction}-${edge.file}`} className="dependency-edge">
            <div className="dependency-edge-main">
              <FileLink file={edge.file} />
              <span className="pill">{edge.count}</span>
            </div>
            {edge.examples.length > 0 && (
              <div className="dependency-examples">{edge.examples.join(" · ")}</div>
            )}
          </div>
        ))
      )}
    </div>
  );
}

export function DependencyGraph({ dependencies }: { dependencies: FileDependencies }) {
  return (
    <div className="dependency-graph">
      <EdgeList title="Inbound" edges={dependencies.inbound} direction="in" />
      <div className="dependency-node">
        <div className="dependency-node-label">Current file</div>
        <strong>{dependencies.file}</strong>
      </div>
      <EdgeList title="Outbound" edges={dependencies.outbound} direction="out" />
    </div>
  );
}

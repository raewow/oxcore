import { Link } from "react-router-dom";
import type { WorkingFile } from "../api/client";

export function WorkingFiles({ files }: { files: WorkingFile[] }) {
  if (!files.length) return null;

  return (
    <div className="working-files">
      <div className="section-title">Working now</div>
      <div className="working-file-grid">
        {files.map((file) => (
          <Link
            key={file.file}
            className="working-file-card"
            to={`/files/detail?path=${encodeURIComponent(file.file)}`}
          >
            <div className="working-file-name">{file.file}</div>
            <div className="working-file-meta">
              {file.running} running · {file.queued} queued · jobs #{file.job_ids.join(", #")}
            </div>
            <div className="job-progress">
              <div
                className="job-progress-bar"
                style={{ width: `${file.total ? (file.progress / file.total) * 100 : 0}%` }}
              />
            </div>
            <div className="working-file-meta">{file.stages.join(", ")}</div>
          </Link>
        ))}
      </div>
    </div>
  );
}

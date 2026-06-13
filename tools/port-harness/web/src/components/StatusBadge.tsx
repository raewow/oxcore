export function StatusBadge({ status }: { status: string }) {
  const cls = `badge badge-${status}`;
  return <span className={cls}>{status.replace(/_/g, " ")}</span>;
}

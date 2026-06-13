export const AGENT_STAGES = new Set([
  "extract",
  "plan-rust",
  "port",
  "verify",
  "audit-rust",
  "discover",
  "assemble-flows",
]);

export const BACKGROUND_STAGES = new Set(["index", "index-dir", "file-pipeline"]);

export type JobQueueKind = "agent" | "background";

export function isAgentStage(stage: string): boolean {
  return AGENT_STAGES.has(stage);
}

export function isBackgroundStage(stage: string): boolean {
  return BACKGROUND_STAGES.has(stage);
}

export function queueKindForStage(stage: string): JobQueueKind {
  if (isAgentStage(stage)) return "agent";
  if (isBackgroundStage(stage)) return "background";
  throw new Error(`Unknown job stage: ${stage}`);
}

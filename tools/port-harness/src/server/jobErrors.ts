export class JobPausedError extends Error {
  constructor(public readonly jobId: number) {
    super(`Job ${jobId} paused`);
    this.name = "JobPausedError";
  }
}

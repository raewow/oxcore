import { spawn } from "node:child_process";
import type { HarnessConfig } from "../config.js";

export interface CargoResult {
  success: boolean;
  stdout: string;
  stderr: string;
  exitCode: number;
}

export function runCargoTest(
  config: HarnessConfig,
  testName?: string,
): Promise<CargoResult> {
  const args = ["test", "-p", "wow-server"];
  if (testName) {
    args.push(testName);
  }
  args.push("--", "--nocapture");

  return runCargo(config, args);
}

export function runCargoCheck(config: HarnessConfig): Promise<CargoResult> {
  return runCargo(config, ["check", "-p", "wow-server"]);
}

function runCargo(config: HarnessConfig, args: string[]): Promise<CargoResult> {
  return new Promise((resolve) => {
    const proc = spawn("cargo", args, {
      cwd: config.rustRoot,
      shell: true,
    });

    let stdout = "";
    let stderr = "";

    proc.stdout.on("data", (data: Buffer) => {
      stdout += data.toString();
    });
    proc.stderr.on("data", (data: Buffer) => {
      stderr += data.toString();
    });

    proc.on("close", (code) => {
      resolve({
        success: code === 0,
        stdout,
        stderr,
        exitCode: code ?? 1,
      });
    });

    proc.on("error", (err) => {
      resolve({
        success: false,
        stdout,
        stderr: stderr + err.message,
        exitCode: 1,
      });
    });
  });
}

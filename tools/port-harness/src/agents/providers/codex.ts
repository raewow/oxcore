import { spawn } from "node:child_process";
import { existsSync, readdirSync } from "node:fs";
import { tmpdir } from "node:os";
import { writeFileSync, unlinkSync } from "node:fs";
import { join } from "node:path";
import type { z } from "zod";
import type { AgentCallOptions, AgentProvider } from "../provider.js";
import { schemaToJsonSchema } from "../provider.js";

export interface CodexProviderOptions {
  model: string;
  cwd: string;
  codexBin?: string;
  portSandbox?: "read-only" | "workspace-write" | "danger-full-access";
  onActivity?: (message: string) => void;
}

export function buildCodexExecArgs(options: {
  model: string;
  sandbox: string;
  schemaFile: string;
}): string[] {
  return [
    "exec",
    "--json",
    "--ephemeral",
    "--model",
    options.model,
    "--sandbox",
    options.sandbox,
    "--output-schema",
    options.schemaFile,
    "-",
  ];
}

function isPathLike(command: string): boolean {
  return command.includes("/") || command.includes("\\") || /^[A-Za-z]:/.test(command);
}

function findWindowsAppsCodex(): string | undefined {
  if (process.platform !== "win32") return undefined;

  const roots = [
    join(process.env.LOCALAPPDATA ?? "", "OpenAI", "Codex", "bin", "codex.exe"),
    join(process.env.LOCALAPPDATA ?? "", "Microsoft", "WindowsApps", "codex.exe"),
    join("C:\\", "Program Files", "WindowsApps"),
  ];

  for (const directPath of roots.slice(0, 2)) {
    if (directPath && existsSync(directPath)) return directPath;
  }

  const windowsAppsRoot = roots[2];
  try {
    const candidates = readdirSync(windowsAppsRoot, { withFileTypes: true })
      .filter((entry) => entry.isDirectory() && entry.name.startsWith("OpenAI.Codex_"))
      .map((entry) => join(windowsAppsRoot, entry.name, "app", "resources", "codex.exe"))
      .filter((path) => existsSync(path))
      .sort()
      .reverse();
    return candidates[0];
  } catch {
    return undefined;
  }
}

export function resolveCodexBin(configured?: string): string {
  const requested = configured ?? process.env.CODEX_BIN ?? "codex";
  if (requested !== "codex" || isPathLike(requested)) return requested;
  return findWindowsAppsCodex() ?? requested;
}

function extractJson(text: string): string {
  const fenced = text.match(/```(?:json)?\s*([\s\S]*?)```/);
  if (fenced?.[1]) return fenced[1].trim();
  const brace = text.match(/\{[\s\S]*\}/);
  if (brace) return brace[0];
  return text.trim();
}

function summarizeEvent(event: Record<string, unknown>): string | undefined {
  if (event.type === "turn.started") return "Codex turn started";
  if (event.type === "turn.completed") return "Codex turn completed";
  if (event.type === "turn.failed") return "Codex turn failed";
  if (event.type === "item.started") {
    const item = event.item as { type?: string; command?: string } | undefined;
    if (item?.type === "command_execution") return `Command: ${item.command ?? "running"}`;
    if (item?.type) return `Codex ${item.type}`;
  }
  if (event.type === "item.completed") {
    const item = event.item as { type?: string; text?: string } | undefined;
    if (item?.type === "agent_message" && item.text) {
      const text = item.text.trim().replace(/\s+/g, " ");
      return text.length > 140 ? `Agent: ${text.slice(0, 140)}...` : `Agent: ${text}`;
    }
    if (item?.type) return `Codex ${item.type} completed`;
  }
  if (event.type === "error") return `Codex error: ${String(event.message ?? "unknown")}`;
  return undefined;
}

export function createCodexProvider(options: CodexProviderOptions): AgentProvider {
  const codexBin = resolveCodexBin(options.codexBin);
  const portSandbox = options.portSandbox ?? "workspace-write";

  return {
    name: "codex",
    async completeStructured<T>(
      system: string,
      user: string,
      schema: z.ZodType<T>,
      callOptions: AgentCallOptions = {},
    ): Promise<T> {
      const jsonSchema = schemaToJsonSchema(schema);
      const stamp = `${Date.now()}-${Math.random().toString(16).slice(2)}`;
      const promptFile = join(tmpdir(), `port-harness-codex-prompt-${stamp}.txt`);
      const schemaFile = join(tmpdir(), `port-harness-codex-schema-${stamp}.json`);
      const sandbox = callOptions.stage === "port" ? portSandbox : "read-only";

      const prompt = [
        system,
        "",
        user,
        "",
        "Return only a final JSON object matching the provided output schema.",
      ].join("\n");

      writeFileSync(promptFile, prompt, "utf-8");
      writeFileSync(schemaFile, JSON.stringify(jsonSchema, null, 2), "utf-8");
      options.onActivity?.(`Calling Codex (${options.model}, ${sandbox}) via ${codexBin}...`);

      return new Promise((resolve, reject) => {
        const args = buildCodexExecArgs({
          model: options.model,
          sandbox,
          schemaFile,
        });

        const child = spawn(codexBin, args, {
          cwd: options.cwd,
          env: { ...process.env },
          stdio: ["pipe", "pipe", "pipe"],
        });
        child.stdin.end(
          "Read the prompt file and complete the task. The prompt file path is: " + promptFile,
        );

        let stdoutBuffer = "";
        let rawStdout = "";
        let finalText = "";
        let lastLogged = "";
        let stderr = "";

        child.stdout.on("data", (data: Buffer) => {
          const chunk = data.toString();
          rawStdout += chunk;
          stdoutBuffer += chunk;
          const lines = stdoutBuffer.split("\n");
          stdoutBuffer = lines.pop() ?? "";

          for (const line of lines) {
            if (!line.trim()) continue;
            try {
              const event = JSON.parse(line) as Record<string, unknown>;
              const item = event.item as { type?: string; text?: string } | undefined;
              if (event.type === "item.completed" && item?.type === "agent_message" && item.text) {
                finalText = item.text;
              }
              const summary = summarizeEvent(event);
              if (summary && summary !== lastLogged) {
                lastLogged = summary;
                options.onActivity?.(summary);
              }
            } catch {
              // Some Codex builds may print only the final JSON. Keep it as fallback.
            }
          }
        });

        child.stderr.on("data", (data: Buffer) => {
          const text = data.toString();
          stderr += text;
          const line = text.trim();
          if (line && line !== lastLogged) {
            lastLogged = line;
            options.onActivity?.(line.length > 180 ? `${line.slice(0, 180)}...` : line);
          }
        });

        child.on("error", (err) => {
          reject(new Error(`Failed to start Codex at ${codexBin}: ${err.message}`));
        });

        child.on("close", (code) => {
          for (const file of [promptFile, schemaFile]) {
            try {
              unlinkSync(file);
            } catch {
              // ignore temp cleanup errors
            }
          }

          if (code !== 0) {
            reject(new Error(`codex exited with code ${code}: ${stderr}`));
            return;
          }

          try {
            const raw = finalText || rawStdout;
            const parsed = JSON.parse(extractJson(raw));
            options.onActivity?.("Codex finished");
            resolve(schema.parse(parsed));
          } catch (err) {
            reject(
              new Error(
                `Failed to parse Codex response: ${err instanceof Error ? err.message : String(err)}`,
              ),
            );
          }
        });
      });
    },
  };
}

import type { z } from "zod";
import { zodToJsonSchema } from "zod-to-json-schema";
import { createCursorProvider } from "./providers/cursor.js";
import { createOpencodeProvider } from "./providers/opencode.js";
import { createCodexProvider } from "./providers/codex.js";

export interface AgentCallOptions {
  stage?: string;
}

export interface AgentProvider {
  name: string;
  completeStructured<T>(
    system: string,
    user: string,
    schema: z.ZodType<T>,
    options?: AgentCallOptions,
  ): Promise<T>;
}

export function schemaToJsonSchema(schema: z.ZodType): Record<string, unknown> {
  return zodToJsonSchema(schema, { target: "openApi3" }) as Record<string, unknown>;
}

export interface ProviderOptions {
  model: string;
  apiKey?: string;
  baseUrl?: string;
  /** Repo root — used as Cursor local agent cwd */
  cwd?: string;
  /** Live activity lines for job UI / server console */
  onActivity?: (message: string) => void;
  codexBin?: string;
  codexPortSandbox?: "read-only" | "workspace-write" | "danger-full-access";
}

export async function createProvider(
  name: string,
  options: ProviderOptions,
): Promise<AgentProvider> {
  switch (name) {
    case "cursor":
      if (!options.cwd) {
        throw new Error("Cursor provider requires cwd (repo root)");
      }
      return createCursorProvider(
        options.model,
        options.apiKey!,
        options.cwd,
        options.onActivity,
      );
    case "openai":
      return await createOpenAIProvider(options.model, options.apiKey!);
    case "anthropic":
      return await createAnthropicProvider(options.model, options.apiKey!);
    case "openai_compat":
      return createOpenAICompatProvider(options.model, options.apiKey!, options.baseUrl);
    case "opencode":
      if (!options.cwd) {
        throw new Error("opencode provider requires cwd (repo root)");
      }
      return createOpencodeProvider(
        options.model || "anthropic/claude-sonnet-4-6",
        options.cwd,
        options.onActivity,
      );
    case "codex":
      if (!options.cwd) {
        throw new Error("codex provider requires cwd (repo root)");
      }
      return createCodexProvider({
        model: options.model || "gpt-5.4-mini",
        cwd: options.cwd,
        codexBin: options.codexBin,
        portSandbox: options.codexPortSandbox,
        onActivity: options.onActivity,
      });
    default:
      throw new Error(`Unknown provider: ${name}`);
  }
}

async function createOpenAIProvider(model: string, apiKey: string): Promise<AgentProvider> {
  const { default: OpenAI } = await import("openai");
  const client = new OpenAI({ apiKey });

  return {
    name: "openai",
    async completeStructured<T>(system: string, user: string, schema: z.ZodType<T>): Promise<T> {
      const jsonSchema = schemaToJsonSchema(schema);
      const response = await client.chat.completions.create({
        model,
        messages: [
          { role: "system", content: system },
          { role: "user", content: user },
        ],
        response_format: {
          type: "json_schema",
          json_schema: {
            name: "response",
            strict: true,
            schema: jsonSchema,
          },
        },
      });
      const content = response.choices[0]?.message?.content ?? "{}";
      return schema.parse(JSON.parse(content));
    },
  };
}

async function createAnthropicProvider(model: string, apiKey: string): Promise<AgentProvider> {
  const { default: Anthropic } = await import("@anthropic-ai/sdk");
  const client = new Anthropic({ apiKey });

  return {
    name: "anthropic",
    async completeStructured<T>(system: string, user: string, schema: z.ZodType<T>): Promise<T> {
      const jsonSchema = schemaToJsonSchema(schema);
      const response = await client.messages.create({
        model,
        max_tokens: 8192,
        system: `${system}\n\nRespond with valid JSON matching this schema:\n${JSON.stringify(jsonSchema)}`,
        messages: [{ role: "user", content: user }],
      });
      const block = response.content[0];
      const text = block?.type === "text" ? block.text : "{}";
      const jsonMatch = text.match(/\{[\s\S]*\}/);
      const jsonStr = jsonMatch ? jsonMatch[0] : text;
      return schema.parse(JSON.parse(jsonStr));
    },
  };
}

function createOpenAICompatProvider(
  model: string,
  apiKey: string,
  baseUrl?: string,
): AgentProvider {
  const url = baseUrl ?? process.env.PORT_HARNESS_API_BASE ?? "http://localhost:8080/v1";

  return {
    name: "openai_compat",
    async completeStructured<T>(system: string, user: string, schema: z.ZodType<T>): Promise<T> {
      const jsonSchema = schemaToJsonSchema(schema);
      const response = await fetch(`${url}/chat/completions`, {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
          Authorization: `Bearer ${apiKey}`,
        },
        body: JSON.stringify({
          model,
          messages: [
            { role: "system", content: `${system}\n\nRespond with JSON matching schema.` },
            { role: "user", content: user },
          ],
          response_format: { type: "json_object" },
        }),
      });
      const data = (await response.json()) as {
        choices: { message: { content: string } }[];
      };
      const content = data.choices[0]?.message?.content ?? "{}";
      const parsed = JSON.parse(content);
      return schema.parse(parsed);
    },
  };
}

export async function getProviderFromConfig(config: {
  name: string;
  model: string;
  apiKeyEnv?: string;
  rustRoot?: string;
  onActivity?: (message: string) => void;
  codexBin?: string;
  codexPortSandbox?: "read-only" | "workspace-write" | "danger-full-access";
}): Promise<AgentProvider> {
  const apiKey = config.apiKeyEnv ? process.env[config.apiKeyEnv] : undefined;
  if (config.apiKeyEnv && !apiKey) {
    throw new Error(`Missing API key env var: ${config.apiKeyEnv}`);
  }
  return createProvider(config.name, {
    model: config.model,
    apiKey,
    cwd: config.rustRoot,
    onActivity: config.onActivity,
    codexBin: config.codexBin,
    codexPortSandbox: config.codexPortSandbox,
  });
}

export function hashPrompt(system: string, user: string): string {
  let hash = 0;
  const str = system + user;
  for (let i = 0; i < str.length; i++) {
    hash = (hash << 5) - hash + str.charCodeAt(i);
    hash |= 0;
  }
  return hash.toString(16);
}

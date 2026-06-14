import { z } from "zod";

const riskLevelSchema = z.enum(["low", "medium", "high", "critical"]);

export const flowBranchSchema = z.object({
  condition: z.string(),
  behaviour: z.string(),
  file: z.string(),
  start_line: z.number(),
  end_line: z.number(),
});

export const flowMutationSchema = z.object({
  variable_or_field: z.string(),
  mutation_description: z.string(),
  file: z.string(),
  start_line: z.number(),
  end_line: z.number(),
});

export const flowDraftSchema = z.object({
  name: z.string(),
  description: z.string(),
  entry_symbols: z.array(z.string()),
  expected_behaviour: z.string(),
  risk_level: riskLevelSchema,
  source_file: z.string().optional(),
  replace_data: z.boolean().optional(),
  branches: z.array(flowBranchSchema).optional(),
  mutations: z.array(flowMutationSchema).optional(),
});

export const flowSaveSchema = z.object({
  flows: z.array(flowDraftSchema),
});

export const flowUpdateSchema = z.object({
  flow: z.string(),
  name: z.string().optional(),
  description: z.string().optional(),
  entry_symbols: z.array(z.string()).optional(),
  expected_behaviour: z.string().optional(),
  risk_level: riskLevelSchema.optional(),
  source_file: z.string().optional(),
  replace_data: z.boolean().optional(),
  branches: z.array(flowBranchSchema).optional(),
  mutations: z.array(flowMutationSchema).optional(),
});

export type FlowDraftSchema = z.infer<typeof flowDraftSchema>;
export type FlowUpdateSchema = z.infer<typeof flowUpdateSchema>;

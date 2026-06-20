import Database from 'better-sqlite3';
const db = new Database('port_harness.db');
const feat = process.argv[2];
const f = db.prepare("SELECT id FROM feature_group WHERE name=?").get(feat) as any;
const files = (db.prepare("SELECT target_id FROM feature_assignment WHERE feature_id=? AND target_type='file'").all(f.id) as any[]).map(r=>r.target_id);
for (const p of files.sort()) {
  const syms = db.prepare("SELECT name,start_line,end_line FROM code_symbol WHERE file=? AND parent_symbol_id IS NULL ORDER BY start_line").all(p) as any[];
  if (!syms.length) continue;
  console.log(`\n# ${p} (${syms.length})`);
  for (const s of syms) console.log(`  ${s.start_line}-${s.end_line} ${s.name}`);
}

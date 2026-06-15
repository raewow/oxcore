import fs from 'fs';
try {
  console.log('starting');
  console.log('db files:', fs.readdirSync('.').filter(f => f.endsWith('.db')).join(', '));
} catch (e) {
  console.error('Error:', e.message);
}

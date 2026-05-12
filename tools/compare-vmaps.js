#!/usr/bin/env node
// Comprehensive vmap comparison between our extraction and MaNGOS reference

const fs = require('fs');
const path = require('path');

const OUR_PATH = 'C:/Users/krist/Projects/wow/rcore/tools/extractor/data_tmp/vmaps';
const REF_PATH = 'C:/Users/krist/Desktop/WOW/Vanilla bropack v24/MaNGOS/data/vmaps';

// Read vmtile file header
function readVmtile(filePath) {
    if (!fs.existsSync(filePath)) {
        return null;
    }

    const buf = fs.readFileSync(filePath);
    if (buf.length < 12) {
        return { error: 'file too small', size: buf.length };
    }

    const magic = buf.slice(0, 8).toString('ascii');
    const numSpawns = buf.readUInt32LE(8);

    // Read spawn entries
    const spawns = [];
    let offset = 12;

    for (let i = 0; i < numSpawns && offset < buf.length; i++) {
        if (offset + 30 > buf.length) break;

        const flags = buf.readUInt32LE(offset); offset += 4;
        const adtId = buf.readUInt16LE(offset); offset += 2;
        const uniqueId = buf.readUInt32LE(offset); offset += 4;
        const posX = buf.readFloatLE(offset); offset += 4;
        const posY = buf.readFloatLE(offset); offset += 4;
        const posZ = buf.readFloatLE(offset); offset += 4;
        const rotX = buf.readFloatLE(offset); offset += 4;
        const rotY = buf.readFloatLE(offset); offset += 4;
        const rotZ = buf.readFloatLE(offset); offset += 4;
        const scale = buf.readFloatLE(offset); offset += 4;

        // Check if has bounds (flag & 0x01)
        let bounds = null;
        if (flags & 0x04) {
            if (offset + 24 > buf.length) break;
            bounds = {
                minX: buf.readFloatLE(offset), minY: buf.readFloatLE(offset+4), minZ: buf.readFloatLE(offset+8),
                maxX: buf.readFloatLE(offset+12), maxY: buf.readFloatLE(offset+16), maxZ: buf.readFloatLE(offset+20)
            };
            offset += 24;
        }

        if (offset + 4 > buf.length) break;
        const nameLen = buf.readUInt32LE(offset); offset += 4;
        if (offset + nameLen > buf.length) break;
        const name = buf.slice(offset, offset + nameLen).toString('utf8');
        offset += nameLen;

        if (offset + 4 > buf.length) break;
        const referencedVal = buf.readUInt32LE(offset); offset += 4;

        spawns.push({
            flags: flags.toString(16).padStart(8, '0'),
            adtId,
            uniqueId,
            position: { x: posX.toFixed(2), y: posY.toFixed(2), z: posZ.toFixed(2) },
            rotation: { x: rotX.toFixed(2), y: rotY.toFixed(2), z: rotZ.toFixed(2) },
            scale: scale.toFixed(2),
            bounds,
            name,
            referencedVal
        });
    }

    return {
        magic,
        numSpawns,
        actualSpawns: spawns.length,
        size: buf.length,
        spawns
    };
}

// Get map IDs from vmtree files
function getMapIds(dir) {
    return fs.readdirSync(dir)
        .filter(f => f.endsWith('.vmtree'))
        .map(f => f.replace('.vmtree', ''))
        .sort();
}

console.log('=== VMap Extraction Comparison ===\n');

const ourMaps = getMapIds(OUR_PATH);
const refMaps = getMapIds(REF_PATH);

console.log(`Our maps (${ourMaps.length}):`);
console.log(ourMaps.join(', '));
console.log(`\nReference maps (${refMaps.length}):`);
console.log(refMaps.join(', '));

const missingMaps = refMaps.filter(m => !ourMaps.includes(m));
const extraMaps = ourMaps.filter(m => !refMaps.includes(m));

if (missingMaps.length > 0) {
    console.log(`\n⚠️  Missing ${missingMaps.length} maps:`);
    console.log(missingMaps.join(', '));
}

if (extraMaps.length > 0) {
    console.log(`\n⚠️  Extra ${extraMaps.length} maps (not in reference):`);
    console.log(extraMaps.join(', '));
}

// Compare common maps
const commonMaps = ourMaps.filter(m => refMaps.includes(m));
console.log(`\n✓ ${commonMaps.length} common maps to compare\n`);

const issues = [];

for (const mapId of commonMaps) {
    const ourTree = path.join(OUR_PATH, `${mapId}.vmtree`);
    const refTree = path.join(REF_PATH, `${mapId}.vmtree`);

    const ourSize = fs.statSync(ourTree).size;
    const refSize = fs.statSync(refTree).size;

    const diff = ourSize - refSize;
    const diffPct = ((diff / refSize) * 100).toFixed(1);

    // Get vmtile counts
    const ourTiles = fs.readdirSync(OUR_PATH).filter(f => f.startsWith(`${mapId}_`) && f.endsWith('.vmtile'));
    const refTiles = fs.readdirSync(REF_PATH).filter(f => f.startsWith(`${mapId}_`) && f.endsWith('.vmtile'));

    const status = Math.abs(diff) < 1000 ? '✓' :
                   diff > 0 ? '⚠️ ' : '❌';

    console.log(`Map ${mapId}: ${status}`);
    console.log(`  vmtree: ${ourSize.toLocaleString()} bytes (ref: ${refSize.toLocaleString()}, diff: ${diffPct}%)`);
    console.log(`  tiles:  ${ourTiles.length} (ref: ${refTiles.length})`);

    if (ourTiles.length !== refTiles.length) {
        issues.push(`Map ${mapId}: tile count mismatch (${ourTiles.length} vs ${refTiles.length})`);
    }

    if (Math.abs(diffPct) > 5) {
        issues.push(`Map ${mapId}: vmtree size differs by ${diffPct}%`);
    }
}

// Detailed comparison for a sample map (033 - Shadowfang Keep)
if (commonMaps.includes('033')) {
    console.log('\n=== Detailed comparison: Map 033 (Shadowfang Keep) ===\n');

    const sampleTile = '033_27_30.vmtile';
    const ourFile = path.join(OUR_PATH, sampleTile);
    const refFile = path.join(REF_PATH, sampleTile);

    if (fs.existsSync(ourFile) && fs.existsSync(refFile)) {
        const our = readVmtile(ourFile);
        const ref = readVmtile(refFile);

        console.log(`${sampleTile}:`);
        console.log(`  Our: ${our.size.toLocaleString()} bytes, ${our.numSpawns} spawns, magic: "${our.magic}"`);
        console.log(`  Ref: ${ref.size.toLocaleString()} bytes, ${ref.numSpawns} spawns, magic: "${ref.magic}"`);

        if (our.spawns.length > 0 && ref.spawns.length > 0) {
            // Count spawns by flags
            const ourFlags = {};
            const refFlags = {};
            our.spawns.forEach(s => ourFlags[s.flags] = (ourFlags[s.flags] || 0) + 1);
            ref.spawns.forEach(s => refFlags[s.flags] = (refFlags[s.flags] || 0) + 1);

            console.log('\n  Flag distribution:');
            console.log(`    Our: ${JSON.stringify(ourFlags)}`);
            console.log(`    Ref: ${JSON.stringify(refFlags)}`);

            const ourWithBounds = our.spawns.filter(s => s.bounds).length;
            const refWithBounds = ref.spawns.filter(s => s.bounds).length;
            console.log(`\n  With bounds: ${ourWithBounds} (ref: ${refWithBounds})`);

            // Show first spawn for format verification
            console.log('\n  First spawn (ours):');
            console.log(`    ${JSON.stringify(our.spawns[0], null, 4)}`);
            console.log('\n  First spawn (reference):');
            console.log(`    ${JSON.stringify(ref.spawns[0], null, 4)}`);
        }
    }
}

// Summary
console.log('\n=== Summary ===\n');
console.log(`Total maps: ${ourMaps.length}/${refMaps.length}`);
console.log(`Missing maps: ${missingMaps.length}`);

if (issues.length > 0) {
    console.log(`\n⚠️  Issues found: ${issues.length}`);
    issues.forEach(issue => console.log(`  - ${issue}`));
} else {
    console.log('\n✓ All common maps look good!');
}

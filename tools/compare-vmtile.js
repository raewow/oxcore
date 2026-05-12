#!/usr/bin/env node
// Compare vmtile files between our extraction and MaNGOS reference

const fs = require('fs');
const path = require('path');

const OUR_PATH = 'C:/Users/krist/Projects/wow/rcore/tools/extractor/data_tmp/vmaps';
const REF_PATH = 'C:/Users/krist/Projects/wow/rcore/data/vmaps';

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

        // Check if has bounds (flag & 0x04 = MOD_HAS_BOUND)
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

// Compare two vmtile files
function compareFiles(ourFile, refFile) {
    const our = readVmtile(ourFile);
    const ref = readVmtile(refFile);

    console.log('\n=== Comparing', path.basename(ourFile), '===');
    console.log('Our file:', our ? `${our.size} bytes, ${our.numSpawns} spawns` : 'NOT FOUND');
    console.log('Ref file:', ref ? `${ref.size} bytes, ${ref.numSpawns} spawns` : 'NOT FOUND');

    if (!our || !ref) return;

    console.log('\nMagic - Our:', our.magic, '| Ref:', ref.magic);

    if (our.spawns.length > 0 && ref.spawns.length > 0) {
        console.log('\n--- First spawn entry comparison ---');
        console.log('Our first spawn:', JSON.stringify(our.spawns[0], null, 2));
        console.log('Ref first spawn:', JSON.stringify(ref.spawns[0], null, 2));

        // Count spawns by flags
        const ourFlags = {};
        const refFlags = {};
        our.spawns.forEach(s => ourFlags[s.flags] = (ourFlags[s.flags] || 0) + 1);
        ref.spawns.forEach(s => refFlags[s.flags] = (refFlags[s.flags] || 0) + 1);

        console.log('\n--- Spawn flags distribution ---');
        console.log('Our flags:', ourFlags);
        console.log('Ref flags:', refFlags);

        // Count spawns with/without bounds
        const ourWithBounds = our.spawns.filter(s => s.bounds).length;
        const refWithBounds = ref.spawns.filter(s => s.bounds).length;
        console.log('\nWith bounds - Our:', ourWithBounds, '| Ref:', refWithBounds);
    }
}

// List all map 33 files
console.log('=== Map 33 VMap Files ===\n');

console.log('Reference files:');
const refFiles = fs.readdirSync(REF_PATH).filter(f => f.startsWith('033'));
refFiles.forEach(f => {
    const stat = fs.statSync(path.join(REF_PATH, f));
    console.log(`  ${f}: ${stat.size} bytes`);
});

console.log('\nOur files:');
const ourFiles = fs.readdirSync(OUR_PATH).filter(f => f.startsWith('033'));
ourFiles.forEach(f => {
    const stat = fs.statSync(path.join(OUR_PATH, f));
    console.log(`  ${f}: ${stat.size} bytes`);
});

// Compare the common file
const commonFile = '033_27_30.vmtile';
if (fs.existsSync(path.join(OUR_PATH, commonFile)) && fs.existsSync(path.join(REF_PATH, commonFile))) {
    compareFiles(path.join(OUR_PATH, commonFile), path.join(REF_PATH, commonFile));
}

// Also compare vmtree
if (fs.existsSync(path.join(OUR_PATH, '033.vmtree')) && fs.existsSync(path.join(REF_PATH, '033.vmtree'))) {
    const ourTree = fs.statSync(path.join(OUR_PATH, '033.vmtree'));
    const refTree = fs.statSync(path.join(REF_PATH, '033.vmtree'));
    console.log('\n=== VMTree Comparison ===');
    console.log('Our 033.vmtree:', ourTree.size, 'bytes');
    console.log('Ref 033.vmtree:', refTree.size, 'bytes');
}

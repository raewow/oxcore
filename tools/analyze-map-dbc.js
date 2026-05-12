#!/usr/bin/env node
// Check which maps exist in Map.dbc from both WoW clients

const fs = require('fs');

function parseMapDBC(buffer) {
    if (buffer.length < 20) {
        return { error: 'file too small', size: buffer.length };
    }

    const magic = buffer.slice(0, 4).toString('ascii');
    if (magic !== 'WDBC') {
        return { error: `Invalid magic: ${magic}`, expected: 'WDBC' };
    }

    const recordCount = buffer.readUInt32LE(4);
    const fieldCount = buffer.readUInt32LE(8);
    const recordSize = buffer.readUInt32LE(12);
    const stringBlockSize = buffer.readUInt32LE(16);

    const dataStart = 20;
    const stringBlockStart = dataStart + (recordCount * recordSize);

    const maps = [];

    for (let i = 0; i < recordCount; i++) {
        const offset = dataStart + (i * recordSize);
        const mapId = buffer.readUInt32LE(offset);
        const nameOffset = buffer.readUInt32LE(offset + 4);

        // Read string from string block
        let name = '';
        if (nameOffset < stringBlockSize) {
            const strStart = stringBlockStart + nameOffset;
            let strEnd = strStart;
            while (strEnd < buffer.length && buffer[strEnd] !== 0) {
                strEnd++;
            }
            name = buffer.slice(strStart, strEnd).toString('utf8');
        }

        if (name) {
            maps.push({ id: mapId, name });
        }
    }

    return { magic, recordCount, fieldCount, recordSize, stringBlockSize, maps };
}

console.log('=== Analyzing Map.dbc files ===\n');

// Try to find Map.dbc from extraction output
const extractedDbcPath = 'C:/Users/krist/Projects/wow/rcore/server/data/dbc/Map.dbc';
if (fs.existsSync(extractedDbcPath)) {
    console.log('Reading from extracted DBC:', extractedDbcPath);
    const buf = fs.readFileSync(extractedDbcPath);
    const result = parseMapDBC(buf);

    if (result.error) {
        console.log('ERROR:', result.error);
    } else {
        console.log(`Records: ${result.recordCount}`);
        console.log(`Fields: ${result.fieldCount}, Record Size: ${result.recordSize} bytes`);
        console.log(`\nMap IDs (${result.maps.length} maps):`);
        const mapIds = result.maps.map(m => m.id.toString().padStart(3, '0')).sort();
        console.log(mapIds.join(', '));

        console.log('\nMaps with names:');
        result.maps
            .sort((a, b) => a.id - b.id)
            .forEach(m => console.log(`  ${m.id.toString().padStart(3, '0')}: ${m.name}`));
    }
} else {
    console.log('Extracted DBC not found at:', extractedDbcPath);
    console.log('Need to extract DBCs first or read directly from MPQ');
}

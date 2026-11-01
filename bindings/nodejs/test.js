const assert = require('assert');
const path = require('path');
const fs = require('fs');

const rasgapdf = require('./index.js');

console.log('Testing Node.js bindings for RasgaPDF...');

// 1. Version test
const version = rasgapdf.version();
console.log('Version:', version);
assert.ok(typeof version === 'string' && version.length > 0, 'Version should be non-empty string');

// Find fixture
const samplePath = path.resolve(__dirname, '../../docs/sample.pdf');
const mixedPath = path.resolve(__dirname, '../../tests/fixtures/mixed.pdf');
const fixturePath = fs.existsSync(mixedPath) ? mixedPath : samplePath;

if (fs.existsSync(fixturePath)) {
  // 2. parseRaw
  const raw = rasgapdf.parseRaw(fixturePath);
  assert.ok(raw.total_pages > 0, 'Should have pages');
  assert.ok(Array.isArray(raw.spans), 'spans should be an array');
  console.log(`[PASS] parseRaw: ${raw.total_spans} spans`);

  // 3. parsePdf
  const doc = rasgapdf.parsePdf(fixturePath, { detect_tables: true, apply_rotation: true });
  assert.ok(doc.total_pages > 0, 'Structured doc should have pages');
  assert.ok(Array.isArray(doc.pages[0].text_blocks), 'text_blocks should be array');
  console.log(`[PASS] parsePdf: ${doc.pages.length} pages`);

  // 4. bytes API
  if (typeof rasgapdf.extractTextBytes === 'function') {
    const bytes = fs.readFileSync(fixturePath);
    const text = rasgapdf.extractTextBytes(bytes);
    assert.ok(typeof text === 'string' && text.length > 0, 'Extracted text should not be empty');
    console.log(`[PASS] extractTextBytes: ${text.length} chars`);
  }
} else {
  console.log('Sample fixture not found, skipping file parse tests');
}

// 5. Error handling
assert.throws(() => {
  rasgapdf.parseRaw('non_existent_file_12345.pdf');
}, /not found|error/i, 'Should throw on non-existent file');

console.log('✅ All Node.js tests passed successfully!');

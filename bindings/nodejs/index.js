/**
 * RasgaPDF - Node.js Binding
 * Ultra-fast 2D Spatial PDF Parser powered by Rust
 */

const fs = require('fs');
const path = require('path');

let koffi = null;
let ffiLib = null;

function findSharedLib() {
  const libName = process.platform === 'win32'
    ? 'rasgapdf.dll'
    : (process.platform === 'darwin' ? 'librasgapdf.dylib' : 'librasgapdf.so');

  const candidates = [
    path.join(__dirname, libName),
    path.join(__dirname, '..', '..', 'target', 'release', libName),
    path.join(__dirname, '..', '..', '..', 'target', 'release', libName),
    path.join(__dirname, '..', '..', 'target', 'debug', libName),
    `/usr/local/lib/${libName}`,
    `/usr/lib/${libName}`
  ];

  for (const c of candidates) {
    if (fs.existsSync(c)) return c;
  }
  return libName;
}

// Inicializa FFI via koffi
const sharedLibPath = findSharedLib();
try {
  koffi = require('koffi');
  const lib = koffi.load(sharedLibPath);
  ffiLib = {
    parseRaw: lib.func('void* rasgapdf_parse_raw(str)'),
    parseFile: lib.func('void* rasgapdf_parse_file_options(str, str)'),
    extractImages: lib.func('void* rasgapdf_extract_images(str, str)'),
    freeString: lib.func('void rasgapdf_free_string(void*)'),
    version: lib.func('str rasgapdf_version()')
  };
  try {
    ffiLib.extractTextBytes = lib.func('void* rasgapdf_extract_text_bytes(uint8_t*, size_t)');
  } catch (_) {}
} catch (e) {
  // FFI lib will report descriptive error on function invocation if not loaded
}

function decodeAndFree(ptr) {
  if (!ptr) throw new Error('Null pointer returned from native rasgapdf library');
  try {
    return koffi.decode(ptr, 'char', -1);
  } finally {
    ffiLib.freeString(ptr);
  }
}

/**
 * ⚡ Modo 1 (Bruto): Retorna exclusivamente a lista plana de coordenadas X/Y
 * sem overhead de agrupamento em blocos ou tabelas.
 * 
 * @param {string} filePath Caminho do arquivo PDF
 * @returns {object} RawSpatialDocument { file_name, total_pages, total_spans, parse_time_ms, spans: [...] }
 */
function parseRaw(filePath) {
  const absPath = path.resolve(filePath);
  if (!fs.existsSync(absPath)) {
    throw new Error(`PDF file not found: ${absPath}`);
  }

  if (!ffiLib || !ffiLib.parseRaw) {
    throw new Error('RasgaPDF native FFI library not loaded');
  }

  const ptr = ffiLib.parseRaw(absPath);
  const rawStr = decodeAndFree(ptr);
  const parsed = JSON.parse(rawStr);
  if (parsed.error) throw new Error(parsed.message || 'Error parsing PDF');
  return parsed;
}

/**
 * 🎯 Modo 2 (Estruturado): Processa o PDF e retorna a árvore geométrica hierárquica
 * com blocos de texto, linhas, spans e tabelas detectadas.
 *
 * Passe `extract_images: true` para preencher `page.images` e
 * `extract_drawings: true` para preencher `page.drawings` (ambos desligados por padrão).
 *
 * @param {string} filePath Caminho do arquivo PDF
 * @param {object} options Opções de detecção espacial
 * @returns {object} SpatialDocument
 */
function parsePdf(filePath, options = {}) {
  const absPath = path.resolve(filePath);
  if (!fs.existsSync(absPath)) {
    throw new Error(`PDF file not found: ${absPath}`);
  }

  if (!ffiLib || !ffiLib.parseFile) {
    throw new Error('RasgaPDF native FFI library not loaded');
  }

  const defaultOptions = {
    mode: 'structured',
    detect_tables: true,
    min_table_columns: 3,
    column_tolerance: 16.0,
    row_tolerance: 6.0,
    extract_images: false,
    extract_drawings: false,
    filter_background_fills: true,
    ...options
  };

  const optsJson = JSON.stringify(defaultOptions);
  const ptr = ffiLib.parseFile(absPath, optsJson);
  const rawStr = decodeAndFree(ptr);
  const parsed = JSON.parse(rawStr);
  if (parsed.error) throw new Error(parsed.message || 'Error parsing PDF');
  return parsed;
}

/**
 * 🖼️ Extrai os streams originais das imagens embutidas para `outputDir`.
 *
 * Nenhum pixel é decodificado: o stream codificado é copiado como está
 * (ex.: `DCTDecode` → `.jpg`). Um `manifest.json` é gravado junto.
 *
 * @param {string} filePath Caminho do arquivo PDF
 * @param {string} outputDir Pasta destino (criada se não existir)
 * @returns {object} { file_name, total_images, written, images: [{ page, image }] }
 */
function extractImages(filePath, outputDir) {
  const absPath = path.resolve(filePath);
  if (!fs.existsSync(absPath)) {
    throw new Error(`PDF file not found: ${absPath}`);
  }
  if (!outputDir) {
    throw new Error('outputDir is required');
  }

  const absOut = path.resolve(outputDir);

  if (!ffiLib || !ffiLib.extractImages) {
    throw new Error('RasgaPDF native FFI library not loaded');
  }

  const ptr = ffiLib.extractImages(absPath, absOut);
  const rawStr = decodeAndFree(ptr);
  const parsed = JSON.parse(rawStr);
  if (parsed.error) throw new Error(parsed.message || 'Error extracting images');
  return parsed;
}

/**
 * 🚀 Extrai o texto completo do PDF preservando a ordem natural de colunas e blocos de leitura.
 * Aceita caminho do arquivo ou Buffer em memória.
 *
 * @param {string|Buffer} input Caminho do arquivo ou Buffer de bytes do PDF
 * @returns {string} Texto extraído do PDF
 */
function extractText(input) {
  let buf = null;
  let filePath = null;

  if (Buffer.isBuffer(input)) {
    buf = input;
  } else {
    filePath = path.resolve(input);
    if (!fs.existsSync(filePath)) {
      throw new Error(`PDF file not found: ${filePath}`);
    }
  }

  // Caminho nativo ultra-rápido se FFI estiver disponível
  if (ffiLib && ffiLib.extractTextBytes) {
    if (!buf && filePath) {
      buf = fs.readFileSync(filePath);
    }
    if (buf) {
      const ptr = ffiLib.extractTextBytes(buf, buf.length);
      if (ptr) {
        return decodeAndFree(ptr);
      }
    }
  }

  // Fallback via parsePdf estruturado
  if (!filePath && buf) {
    throw new Error('Direct buffer extraction requires native extractTextBytes support');
  }

  const doc = parsePdf(filePath, { detect_tables: false });
  const pagesTexts = [];
  for (const page of (doc.pages || [])) {
    const lines = [];
    for (const block of (page.text_blocks || [])) {
      for (const line of (block.lines || [])) {
        if (line.text && line.text.trim()) lines.push(line.text.trimEnd());
      }
    }
    if (lines.length > 0) pagesTexts.push(lines.join('\n'));
  }
  const joined = pagesTexts.join('\n\n');
  return joined.replace(/\n{3,}/g, '\n\n').trim() + '\n';
}

function version() {
  if (ffiLib && ffiLib.version) return ffiLib.version();
  return '0.3.0';
}

module.exports = {
  parseRaw,
  parsePdf,
  extractText,
  extractImages,
  version
};

/**
 * RasgaPDF - Node.js Binding
 * Ultra-fast 2D Spatial PDF Parser powered by Rust
 */

const fs = require('fs');
const path = require('path');
const { execFileSync } = require('child_process');

let ffiLib = null;

function findBinary() {
  const binaryName = process.platform === 'win32' ? 'rasgapdf.exe' : 'rasgapdf';
  const candidates = [
    path.join(__dirname, '..', '..', 'target', 'release', binaryName),
    path.join(__dirname, '..', '..', '..', 'target', 'release', binaryName),
    path.join(process.env.HOME || '', 'rasgapdf', 'target', 'release', binaryName),
    `/usr/local/bin/${binaryName}`,
    `/usr/bin/${binaryName}`
  ];

  for (const c of candidates) {
    if (fs.existsSync(c)) return c;
  }
  return binaryName;
}

function findSharedLib() {
  const libName = process.platform === 'win32' ? 'rasgapdf.dll' : (process.platform === 'darwin' ? 'librasgapdf.dylib' : 'librasgapdf.so');
  const candidates = [
    path.join(__dirname, libName),
    path.join(__dirname, '..', '..', 'target', 'release', libName),
    path.join(__dirname, '..', '..', '..', 'target', 'release', libName),
    path.join(process.env.HOME || '', 'rasgapdf', 'target', 'release', libName),
    `/usr/local/lib/${libName}`,
    `/usr/lib/${libName}`
  ];

  for (const c of candidates) {
    if (fs.existsSync(c)) return c;
  }
  return null;
}

// Inicializa FFI se koffi estiver disponível
const sharedLibPath = findSharedLib();
if (sharedLibPath) {
  try {
    const koffi = require('koffi');
    const lib = koffi.load(sharedLibPath);
    ffiLib = {
      parseRaw: lib.func('char* rasgapdf_parse_raw(str)'),
      parseFile: lib.func('char* rasgapdf_parse_file_options(str, str)'),
      extractImages: lib.func('char* rasgapdf_extract_images(str, str)'),
      freeString: lib.func('void rasgapdf_free_string(char*)'),
      version: lib.func('str rasgapdf_version()')
    };
  } catch (e) {
    // Fallback para CLI execution
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

  // Se FFI estiver ativo
  if (ffiLib && ffiLib.parseRaw) {
    const ptr = ffiLib.parseRaw(absPath);
    if (!ptr) throw new Error('Null pointer returned from native rasgapdf library');
    try {
      const parsed = JSON.parse(ptr);
      if (parsed.error) throw new Error(parsed.message || 'Error parsing PDF');
      return parsed;
    } finally {
      ffiLib.freeString(ptr);
    }
  }

  // Fallback via binary CLI
  const binPath = findBinary();
  const args = [absPath, '--raw'];
  const stdout = execFileSync(binPath, args, { encoding: 'utf-8', maxBuffer: 100 * 1024 * 1024 });
  return JSON.parse(stdout);
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

  // Se FFI estiver ativo
  if (ffiLib) {
    const optsJson = JSON.stringify(defaultOptions);
    const ptr = ffiLib.parseFile(absPath, optsJson);
    if (!ptr) throw new Error('Null pointer returned from native rasgapdf library');
    try {
      const parsed = JSON.parse(ptr);
      if (parsed.error) throw new Error(parsed.message || 'Error parsing PDF');
      return parsed;
    } finally {
      ffiLib.freeString(ptr);
    }
  }

  // Fallback rápido via binary CLI
  const binPath = findBinary();
  const args = [absPath];
  if (!defaultOptions.detect_tables) args.push('--no-tables');
  args.push('--min-table-cols', String(defaultOptions.min_table_columns));
  args.push('--col-tolerance', String(defaultOptions.column_tolerance));
  args.push('--row-tolerance', String(defaultOptions.row_tolerance));
  if (defaultOptions.extract_images) args.push('--images');
  if (defaultOptions.extract_drawings) args.push('--drawings');
  if (defaultOptions.filter_background_fills === false) args.push('--keep-backgrounds');

  const stdout = execFileSync(binPath, args, { encoding: 'utf-8', maxBuffer: 100 * 1024 * 1024 });
  return JSON.parse(stdout);
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

  if (ffiLib && ffiLib.extractImages) {
    const ptr = ffiLib.extractImages(absPath, absOut);
    if (!ptr) throw new Error('Null pointer returned from native rasgapdf library');
    try {
      const parsed = JSON.parse(ptr);
      if (parsed.error) throw new Error(parsed.message || 'Error extracting images');
      return parsed;
    } finally {
      ffiLib.freeString(ptr);
    }
  }

  // Fallback via binary CLI: o manifesto é gravado em disco.
  const binPath = findBinary();
  execFileSync(binPath, [absPath, '--extract-images', absOut], {
    encoding: 'utf-8',
    maxBuffer: 100 * 1024 * 1024
  });

  return JSON.parse(fs.readFileSync(path.join(absOut, 'manifest.json'), 'utf-8'));
}

function version() {
  if (ffiLib) return ffiLib.version();
  return '0.2.0';
}

module.exports = {
  parseRaw,
  parsePdf,
  extractImages,
  version
};

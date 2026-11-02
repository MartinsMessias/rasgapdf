using System;
using System.IO;
using System.Runtime.InteropServices;
using System.Text.Json;
using System.Text.Json.Serialization;

namespace RasgaPdf
{
    /// <summary>
    /// Elemento de coordenada atômica bruta
    /// </summary>
    public class RawCoordSpan
    {
        [JsonPropertyName("page")] public int Page { get; set; }
        [JsonPropertyName("x0")] public float X0 { get; set; }
        [JsonPropertyName("y0")] public float Y0 { get; set; }
        [JsonPropertyName("x1")] public float X1 { get; set; }
        [JsonPropertyName("y1")] public float Y1 { get; set; }
        [JsonPropertyName("font_size")] public float FontSize { get; set; }
        [JsonPropertyName("font_name")] public string FontName { get; set; } = string.Empty;
        [JsonPropertyName("text")] public string Text { get; set; } = string.Empty;
    }

    /// <summary>
    /// Documento plano contendo exclusivamente a lista de coordenadas X/Y
    /// </summary>
    public class RawSpatialDocument
    {
        [JsonPropertyName("file_name")] public string FileName { get; set; } = string.Empty;
        [JsonPropertyName("total_pages")] public int TotalPages { get; set; }
        [JsonPropertyName("total_spans")] public int TotalSpans { get; set; }
        [JsonPropertyName("parse_time_ms")] public double ParseTimeMs { get; set; }
        [JsonPropertyName("spans")] public RawCoordSpan[] Spans { get; set; } = Array.Empty<RawCoordSpan>();
    }

    public class ParserOptions
    {
        [JsonPropertyName("reading_order")]
        public string ReadingOrder { get; set; } = "natural";
        [JsonPropertyName("mode")]
        public string Mode { get; set; } = "structured";

        [JsonPropertyName("detect_tables")]
        public bool DetectTables { get; set; } = true;

        [JsonPropertyName("min_table_columns")]
        public int MinTableColumns { get; set; } = 3;

        [JsonPropertyName("column_tolerance")]
        public float ColumnTolerance { get; set; } = 16.0f;

        [JsonPropertyName("row_tolerance")]
        public float RowTolerance { get; set; } = 6.0f;

        /// <summary>Collect embedded image metadata into <see cref="SpatialPage.Images"/>.</summary>
        [JsonPropertyName("extract_images")]
        public bool ExtractImages { get; set; } = false;

        /// <summary>Collect vector paths into <see cref="SpatialPage.Drawings"/>.</summary>
        [JsonPropertyName("extract_drawings")]
        public bool ExtractDrawings { get; set; } = false;

        /// <summary>Drop fills covering the whole page (backgrounds).</summary>
        [JsonPropertyName("filter_background_fills")]
        public bool FilterBackgroundFills { get; set; } = true;

        /// <summary>Transforms coordinates to align with the visual display rotation viewport.</summary>
        [JsonPropertyName("apply_rotation")]
        public bool ApplyRotation { get; set; } = false;
    }

    public class BoundingBox
    {
        [JsonPropertyName("x0")] public float X0 { get; set; }
        [JsonPropertyName("y0")] public float Y0 { get; set; }
        [JsonPropertyName("x1")] public float X1 { get; set; }
        [JsonPropertyName("y1")] public float Y1 { get; set; }
        [JsonPropertyName("width")] public float Width { get; set; }
        [JsonPropertyName("height")] public float Height { get; set; }
    }

    public class SpatialSpan
    {
        [JsonPropertyName("text")] public string Text { get; set; } = string.Empty;
        [JsonPropertyName("bbox")] public BoundingBox BBox { get; set; } = new();
        [JsonPropertyName("font_name")] public string FontName { get; set; } = string.Empty;
        [JsonPropertyName("font_size")] public float FontSize { get; set; }
        [JsonPropertyName("char_count")] public int CharCount { get; set; }
    }

    public class SpatialLine
    {
        [JsonPropertyName("text")] public string Text { get; set; } = string.Empty;
        [JsonPropertyName("bbox")] public BoundingBox BBox { get; set; } = new();
        [JsonPropertyName("spans")] public SpatialSpan[] Spans { get; set; } = Array.Empty<SpatialSpan>();
    }

    public class SpatialBlock
    {
        [JsonPropertyName("is_complex_layout")]
        public bool IsComplexLayout { get; set; }

        [JsonPropertyName("complexity_score")]
        public float? ComplexityScore { get; set; }
        [JsonPropertyName("block_id")] public int BlockId { get; set; }
        [JsonPropertyName("block_type")] public string BlockType { get; set; } = "text";
        [JsonPropertyName("bbox")] public BoundingBox BBox { get; set; } = new();
        [JsonPropertyName("lines")] public SpatialLine[] Lines { get; set; } = Array.Empty<SpatialLine>();
    }

    public class SpatialCell
    {
        [JsonPropertyName("row")] public int Row { get; set; }
        [JsonPropertyName("col")] public int Col { get; set; }
        [JsonPropertyName("text")] public string Text { get; set; } = string.Empty;
        [JsonPropertyName("bbox")] public BoundingBox BBox { get; set; } = new();
    }

    public class SpatialTable
    {
        [JsonPropertyName("table_id")] public int TableId { get; set; }
        [JsonPropertyName("bbox")] public BoundingBox BBox { get; set; } = new();
        [JsonPropertyName("col_count")] public int ColCount { get; set; }
        [JsonPropertyName("row_count")] public int RowCount { get; set; }
        [JsonPropertyName("headers")] public string[] Headers { get; set; } = Array.Empty<string>();
        [JsonPropertyName("rows")] public string[][] Rows { get; set; } = Array.Empty<string[]>();
        [JsonPropertyName("cells")] public SpatialCell[] Cells { get; set; } = Array.Empty<SpatialCell>();
    }

    /// <summary>
    /// Embedded image (<c>/Subtype /Image</c>) referenced by a page.
    /// Pixels are never inlined: call <see cref="SpatialPdf.ExtractImages"/> to write
    /// the original encoded streams to disk.
    /// </summary>
    public class SpatialImage
    {
        [JsonPropertyName("image_id")] public int ImageId { get; set; }
        [JsonPropertyName("name")] public string Name { get; set; } = string.Empty;
        [JsonPropertyName("xref")] public int Xref { get; set; }
        [JsonPropertyName("bbox")] public BoundingBox? BBox { get; set; }
        [JsonPropertyName("pixel_width")] public uint PixelWidth { get; set; }
        [JsonPropertyName("pixel_height")] public uint PixelHeight { get; set; }
        [JsonPropertyName("bits_per_component")] public int? BitsPerComponent { get; set; }
        [JsonPropertyName("color_space")] public string? ColorSpace { get; set; }
        [JsonPropertyName("filter")] public string? Filter { get; set; }
        [JsonPropertyName("mime_type")] public string? MimeType { get; set; }
        [JsonPropertyName("byte_length")] public int? ByteLength { get; set; }
        [JsonPropertyName("sha256")] public string? Sha256 { get; set; }
        [JsonPropertyName("file")] public string? File { get; set; }
    }

    /// <summary>
    /// One geometric item of a vector path, discriminated by <see cref="Kind"/>
    /// (<c>line</c>, <c>curve</c>, <c>rect</c> or <c>quad</c>). Unused fields are null.
    /// </summary>
    public class SpatialDrawingItem
    {
        [JsonPropertyName("kind")] public string Kind { get; set; } = string.Empty;
        [JsonPropertyName("p1")] public float[]? P1 { get; set; }
        [JsonPropertyName("p2")] public float[]? P2 { get; set; }
        [JsonPropertyName("p3")] public float[]? P3 { get; set; }
        [JsonPropertyName("p4")] public float[]? P4 { get; set; }
        [JsonPropertyName("bbox")] public BoundingBox? BBox { get; set; }
        [JsonPropertyName("orientation")] public int? Orientation { get; set; }
        [JsonPropertyName("points")] public float[][]? Points { get; set; }
    }

    /// <summary>Painted vector path (fill, stroke or both)</summary>
    public class SpatialDrawing
    {
        [JsonPropertyName("drawing_id")] public int DrawingId { get; set; }
        [JsonPropertyName("bbox")] public BoundingBox BBox { get; set; } = new();
        [JsonPropertyName("drawing_type")] public string DrawingType { get; set; } = string.Empty;
        [JsonPropertyName("item_count")] public int ItemCount { get; set; }
        [JsonPropertyName("items")] public SpatialDrawingItem[] Items { get; set; } = Array.Empty<SpatialDrawingItem>();
        [JsonPropertyName("stroke_color")] public float[]? StrokeColor { get; set; }
        [JsonPropertyName("fill_color")] public float[]? FillColor { get; set; }
        [JsonPropertyName("width")] public float? Width { get; set; }
        [JsonPropertyName("fill_opacity")] public float? FillOpacity { get; set; }
        [JsonPropertyName("stroke_opacity")] public float? StrokeOpacity { get; set; }
        [JsonPropertyName("even_odd")] public bool? EvenOdd { get; set; }
        [JsonPropertyName("dash_pattern")] public float[]? DashPattern { get; set; }
        [JsonPropertyName("dash_phase")] public float? DashPhase { get; set; }
        [JsonPropertyName("line_join")] public string? LineJoin { get; set; }
        [JsonPropertyName("line_cap")] public string? LineCap { get; set; }
        [JsonPropertyName("layer")] public string? Layer { get; set; }
        [JsonPropertyName("seqno")] public int Seqno { get; set; }
    }

    /// <summary>An extracted image together with its 1-based page number</summary>
    public class ExtractedImageRecord
    {
        [JsonPropertyName("page")] public int Page { get; set; }
        [JsonPropertyName("image")] public SpatialImage Image { get; set; } = new();
    }

    /// <summary>Manifest returned by <see cref="SpatialPdf.ExtractImages"/></summary>
    public class ImageManifest
    {
        [JsonPropertyName("file_name")] public string FileName { get; set; } = string.Empty;
        [JsonPropertyName("total_images")] public int TotalImages { get; set; }
        [JsonPropertyName("written")] public int Written { get; set; }
        [JsonPropertyName("images")] public ExtractedImageRecord[] Images { get; set; } = Array.Empty<ExtractedImageRecord>();
    }

    public class SpatialPage
    {
        [JsonPropertyName("is_complex_layout")]
        public bool IsComplexLayout { get; set; }
        [JsonPropertyName("page_number")] public int PageNumber { get; set; }
        [JsonPropertyName("width")] public float Width { get; set; }
        [JsonPropertyName("height")] public float Height { get; set; }
        [JsonPropertyName("rotation")] public int Rotation { get; set; }
        [JsonPropertyName("initial_rotation")] public int InitialRotation { get; set; }
        [JsonPropertyName("coordinate_system")] public string CoordinateSystem { get; set; } = "source";
        [JsonPropertyName("media_box")] public float[]? MediaBox { get; set; }
        [JsonPropertyName("crop_box")] public float[]? CropBox { get; set; }
        [JsonPropertyName("text_blocks")] public SpatialBlock[] TextBlocks { get; set; } = Array.Empty<SpatialBlock>();
        [JsonPropertyName("tables")] public SpatialTable[] Tables { get; set; } = Array.Empty<SpatialTable>();
        [JsonPropertyName("raw_span_count")] public int RawSpanCount { get; set; }
        [JsonPropertyName("images")] public SpatialImage[] Images { get; set; } = Array.Empty<SpatialImage>();
        [JsonPropertyName("drawings")] public SpatialDrawing[] Drawings { get; set; } = Array.Empty<SpatialDrawing>();
    }

    public class SpatialChunk
    {
        [JsonPropertyName("is_complex_layout")]
        public bool IsComplexLayout { get; set; }

        [JsonPropertyName("complexity_score")]
        public float? ComplexityScore { get; set; }
        [JsonPropertyName("chunk_id")] public int ChunkId { get; set; }
        [JsonPropertyName("page_number")] public int PageNumber { get; set; }
        [JsonPropertyName("text")] public string Text { get; set; } = string.Empty;
        [JsonPropertyName("bboxes")] public BoundingBox[] BBoxes { get; set; } = Array.Empty<BoundingBox>();
        [JsonPropertyName("overall_bbox")] public BoundingBox OverallBBox { get; set; } = new();
        [JsonPropertyName("block_count")] public int BlockCount { get; set; }
        [JsonPropertyName("token_estimate")] public int TokenEstimate { get; set; }
        [JsonPropertyName("coordinate_system")] public string CoordinateSystem { get; set; } = "source";
    }

    public class SpatialDocument
    {
        [JsonPropertyName("file_name")] public string FileName { get; set; } = string.Empty;
        [JsonPropertyName("total_pages")] public int TotalPages { get; set; }
        [JsonPropertyName("parse_time_ms")] public double ParseTimeMs { get; set; }
        [JsonPropertyName("pages")] public SpatialPage[] Pages { get; set; } = Array.Empty<SpatialPage>();
    }

    public static class SpatialPdf
    {
        private const string LibName = "rasgapdf";

        [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
        private static extern IntPtr rasgapdf_parse_raw([MarshalAs(UnmanagedType.LPUTF8Str)] string path);

        [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
        private static extern IntPtr rasgapdf_parse_raw_bytes(byte[] bytesPtr, nuint len);

        [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
        private static extern IntPtr rasgapdf_parse_file_options(
            [MarshalAs(UnmanagedType.LPUTF8Str)] string path,
            [MarshalAs(UnmanagedType.LPUTF8Str)] string? optionsJson);

        [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
        private static extern void rasgapdf_free_string(IntPtr ptr);

        [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
        private static extern IntPtr rasgapdf_version();

        [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
        private static extern IntPtr rasgapdf_extract_images(
            [MarshalAs(UnmanagedType.LPUTF8Str)] string path,
            [MarshalAs(UnmanagedType.LPUTF8Str)] string outputDir);

        [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
        private static extern IntPtr rasgapdf_extract_images_bytes(
            byte[] bytesPtr,
            nuint len,
            [MarshalAs(UnmanagedType.LPUTF8Str)] string outputDir);

        [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
        private static extern IntPtr rasgapdf_extract_text_bytes(byte[] bytesPtr, nuint len);

        private static void ThrowIfNativeError(string jsonStr)
        {
            if (string.IsNullOrWhiteSpace(jsonStr)) return;
            try
            {
                using var doc = JsonDocument.Parse(jsonStr);
                if (doc.RootElement.TryGetProperty("error", out var errProp))
                {
                    bool isError = (errProp.ValueKind == JsonValueKind.True) ||
                                   (errProp.ValueKind == JsonValueKind.String && !string.IsNullOrEmpty(errProp.GetString()));
                    if (isError)
                    {
                        var msg = doc.RootElement.TryGetProperty("message", out var msgProp) ? msgProp.GetString() : "Native error";
                        throw new InvalidOperationException($"RasgaPDF Error: {msg}");
                    }
                }
            }
            catch (JsonException)
            {
                // Not an error object
            }
        }

        /// <summary>
        /// 🚀 Extrai o texto completo do PDF preservando a ordem natural de colunas e blocos de leitura.
        /// </summary>
        public static string ExtractText(string filePath)
        {
            var fullPath = Path.GetFullPath(filePath);
            if (!File.Exists(fullPath))
            {
                throw new FileNotFoundException("PDF file not found", fullPath);
            }
            var bytes = File.ReadAllBytes(fullPath);
            return ExtractText(bytes);
        }

        /// <summary>
        /// 🚀 Extrai o texto completo do PDF diretamente de um array de bytes em memória.
        /// </summary>
        public static string ExtractText(byte[] pdfBytes)
        {
            if (pdfBytes == null || pdfBytes.Length == 0)
            {
                return string.Empty;
            }

            try
            {
                var resultPtr = rasgapdf_extract_text_bytes(pdfBytes, (nuint)pdfBytes.Length);
                if (resultPtr != IntPtr.Zero)
                {
                    try
                    {
                        return Marshal.PtrToStringUTF8(resultPtr) ?? string.Empty;
                    }
                    finally
                    {
                        rasgapdf_free_string(resultPtr);
                    }
                }
            }
            catch (EntryPointNotFoundException)
            {
                // Fallback
            }

            return string.Empty;
        }

        public static string GetVersion()
        {
            var ptr = rasgapdf_version();
            if (ptr == IntPtr.Zero) return "0.3.4";
            return Marshal.PtrToStringUTF8(ptr) ?? "0.3.4";
        }

        /// <summary>
        /// ⚡ Modo 1 (Bruto): Retorna exclusivamente a lista plana de coordenadas X/Y de um arquivo
        /// </summary>
        public static RawSpatialDocument ParseRaw(string filePath)
        {
            var fullPath = Path.GetFullPath(filePath);
            if (!File.Exists(fullPath))
            {
                throw new FileNotFoundException("PDF file not found", fullPath);
            }

            var resultPtr = rasgapdf_parse_raw(fullPath);
            if (resultPtr == IntPtr.Zero)
            {
                throw new InvalidOperationException("Native rasgapdf parser returned null");
            }

            try
            {
                var jsonStr = Marshal.PtrToStringUTF8(resultPtr);
                if (string.IsNullOrEmpty(jsonStr))
                {
                    throw new InvalidOperationException("Empty response from rasgapdf parser");
                }

                ThrowIfNativeError(jsonStr);

                var doc = JsonSerializer.Deserialize<RawSpatialDocument>(jsonStr);
                if (doc == null)
                {
                    throw new JsonException("Failed to deserialize RawSpatialDocument");
                }

                return doc;
            }
            finally
            {
                rasgapdf_free_string(resultPtr);
            }
        }

        /// <summary>
        /// ⚡ Modo 1 (Bruto Memória): Retorna lista plana de coordenadas X/Y diretamente de bytes em RAM
        /// </summary>
        public static RawSpatialDocument ParseRaw(byte[] pdfBytes)
        {
            if (pdfBytes == null || pdfBytes.Length == 0)
            {
                throw new ArgumentException("PDF bytes array cannot be null or empty", nameof(pdfBytes));
            }

            var resultPtr = rasgapdf_parse_raw_bytes(pdfBytes, (nuint)pdfBytes.Length);
            if (resultPtr == IntPtr.Zero)
            {
                throw new InvalidOperationException("Native rasgapdf parser returned null");
            }

            try
            {
                var jsonStr = Marshal.PtrToStringUTF8(resultPtr);
                if (string.IsNullOrEmpty(jsonStr))
                {
                    throw new InvalidOperationException("Empty response from rasgapdf parser");
                }

                ThrowIfNativeError(jsonStr);

                var doc = JsonSerializer.Deserialize<RawSpatialDocument>(jsonStr);
                if (doc == null)
                {
                    throw new JsonException("Failed to deserialize RawSpatialDocument");
                }

                return doc;
            }
            finally
            {
                rasgapdf_free_string(resultPtr);
            }
        }

        /// <summary>
        /// 🎯 Modo 2 (Estruturado): Processa o layout hierárquico com blocos e tabelas
        /// </summary>
        public static SpatialDocument Parse(string filePath, ParserOptions? options = null)
        {
            var fullPath = Path.GetFullPath(filePath);
            if (!File.Exists(fullPath))
            {
                throw new FileNotFoundException("PDF file not found", fullPath);
            }

            options ??= new ParserOptions();
            var optsJson = JsonSerializer.Serialize(options);

            var resultPtr = rasgapdf_parse_file_options(fullPath, optsJson);
            if (resultPtr == IntPtr.Zero)
            {
                throw new InvalidOperationException("Native rasgapdf parser returned null");
            }

            try
            {
                var jsonStr = Marshal.PtrToStringUTF8(resultPtr);
                if (string.IsNullOrEmpty(jsonStr))
                {
                    throw new InvalidOperationException("Empty response from rasgapdf parser");
                }

                ThrowIfNativeError(jsonStr);

                var doc = JsonSerializer.Deserialize<SpatialDocument>(jsonStr);
                if (doc == null)
                {
                    throw new JsonException("Failed to deserialize SpatialDocument");
                }

                return doc;
            }
            finally
            {
                rasgapdf_free_string(resultPtr);
            }
        }

        /// <summary>
        /// 🎯 Alias ergonômico para <see cref="Parse(string, ParserOptions?)"/>
        /// </summary>
        public static SpatialDocument ParsePdf(string filePath, ParserOptions? options = null)
        {
            return Parse(filePath, options);
        }

        /// <summary>
        /// 🖼️ Writes original image streams of every embedded image to <paramref name="outputDir"/>
        /// </summary>
        public static ImageManifest ExtractImages(string filePath, string outputDir)
        {
            var fullPath = Path.GetFullPath(filePath);
            if (!File.Exists(fullPath))
            {
                throw new FileNotFoundException("PDF file not found", fullPath);
            }

            var fullOutputDir = Path.GetFullPath(outputDir);

            var resultPtr = rasgapdf_extract_images(fullPath, fullOutputDir);
            if (resultPtr == IntPtr.Zero)
            {
                throw new InvalidOperationException("Native rasgapdf parser returned null");
            }

            try
            {
                var jsonStr = Marshal.PtrToStringUTF8(resultPtr);
                if (string.IsNullOrEmpty(jsonStr))
                {
                    throw new InvalidOperationException("Empty response from rasgapdf parser");
                }

                ThrowIfNativeError(jsonStr);

                var manifest = JsonSerializer.Deserialize<ImageManifest>(jsonStr);
                if (manifest == null)
                {
                    throw new JsonException("Failed to deserialize ImageManifest");
                }

                return manifest;
            }
            finally
            {
                rasgapdf_free_string(resultPtr);
            }
        }

        /// <summary>
        /// 🖼️ Writes original image streams directly from RAM bytes to <paramref name="outputDir"/>
        /// </summary>
        public static ImageManifest ExtractImages(byte[] pdfBytes, string outputDir)
        {
            if (pdfBytes == null || pdfBytes.Length == 0)
            {
                throw new ArgumentException("PDF bytes array cannot be null or empty", nameof(pdfBytes));
            }

            var fullOutputDir = Path.GetFullPath(outputDir);

            var resultPtr = rasgapdf_extract_images_bytes(pdfBytes, (nuint)pdfBytes.Length, fullOutputDir);
            if (resultPtr == IntPtr.Zero)
            {
                throw new InvalidOperationException("Native rasgapdf parser returned null");
            }

            try
            {
                var jsonStr = Marshal.PtrToStringUTF8(resultPtr);
                if (string.IsNullOrEmpty(jsonStr))
                {
                    throw new InvalidOperationException("Empty response from rasgapdf parser");
                }

                ThrowIfNativeError(jsonStr);

                var manifest = JsonSerializer.Deserialize<ImageManifest>(jsonStr);
                if (manifest == null)
                {
                    throw new JsonException("Failed to deserialize ImageManifest");
                }

                return manifest;
            }
            finally
            {
                rasgapdf_free_string(resultPtr);
            }
        }
    }

    /// <summary>
    /// Alias de compatibilidade com a documentação para <see cref="SpatialPdf"/>
    /// </summary>
    public static class PdfParser
    {
        public static RawSpatialDocument ParseRaw(string filePath) => SpatialPdf.ParseRaw(filePath);
        public static RawSpatialDocument ParseRaw(byte[] pdfBytes) => SpatialPdf.ParseRaw(pdfBytes);
        public static SpatialDocument Parse(string filePath, ParserOptions? options = null) => SpatialPdf.Parse(filePath, options);
        public static SpatialDocument ParsePdf(string filePath, ParserOptions? options = null) => SpatialPdf.ParsePdf(filePath, options);
        public static string ExtractText(string filePath) => SpatialPdf.ExtractText(filePath);
        public static string ExtractText(byte[] pdfBytes) => SpatialPdf.ExtractText(pdfBytes);
        public static ImageManifest ExtractImages(string filePath, string outputDir) => SpatialPdf.ExtractImages(filePath, outputDir);
        public static ImageManifest ExtractImages(byte[] pdfBytes, string outputDir) => SpatialPdf.ExtractImages(pdfBytes, outputDir);
        public static string GetVersion() => SpatialPdf.GetVersion();
    }
}

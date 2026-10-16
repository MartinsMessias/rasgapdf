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
        [JsonPropertyName("block_id")] public int BlockId { get; set; }
        [JsonPropertyName("block_type")] public string BlockType { get; set; } = "text";
        [JsonPropertyName("bbox")] public BoundingBox BBox { get; set; } = new();
        [JsonPropertyName("lines")] public SpatialLine[] Lines { get; set; } = Array.Empty<SpatialLine>();
    }

    public class SpatialTable
    {
        [JsonPropertyName("table_id")] public int TableId { get; set; }
        [JsonPropertyName("bbox")] public BoundingBox BBox { get; set; } = new();
        [JsonPropertyName("col_count")] public int ColCount { get; set; }
        [JsonPropertyName("row_count")] public int RowCount { get; set; }
        [JsonPropertyName("headers")] public string[] Headers { get; set; } = Array.Empty<string>();
        [JsonPropertyName("rows")] public string[][] Rows { get; set; } = Array.Empty<string[]>();
    }

    public class SpatialPage
    {
        [JsonPropertyName("page_number")] public int PageNumber { get; set; }
        [JsonPropertyName("width")] public float Width { get; set; }
        [JsonPropertyName("height")] public float Height { get; set; }
        [JsonPropertyName("text_blocks")] public SpatialBlock[] TextBlocks { get; set; } = Array.Empty<SpatialBlock>();
        [JsonPropertyName("tables")] public SpatialTable[] Tables { get; set; } = Array.Empty<SpatialTable>();
        [JsonPropertyName("raw_span_count")] public int RawSpanCount { get; set; }
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

        [DllImport(LibName, CallingConvention = CallingConvention.Cdecl, CharSet = CharSet.Ansi)]
        private static extern IntPtr rasgapdf_parse_raw(string path);

        [DllImport(LibName, CallingConvention = CallingConvention.Cdecl, CharSet = CharSet.Ansi)]
        private static extern IntPtr rasgapdf_parse_file_options(string path, string? optionsJson);

        [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
        private static extern void rasgapdf_free_string(IntPtr ptr);

        [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
        private static extern IntPtr rasgapdf_version();

        public static string GetVersion()
        {
            var ptr = rasgapdf_version();
            return Marshal.PtrToStringAnsi(ptr) ?? "0.1.0";
        }

        /// <summary>
        /// ⚡ Modo 1 (Bruto): Retorna exclusivamente a lista plana de coordenadas X/Y
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
    }
}

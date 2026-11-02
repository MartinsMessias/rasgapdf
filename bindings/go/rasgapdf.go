package rasgapdf

/*
#cgo CFLAGS: -I../../include
#cgo LDFLAGS: -L../../target/release -lrasgapdf -Wl,-rpath,../../target/release
#include "rasgapdf.h"
#include <stdlib.h>
*/
import "C"
import (
	"encoding/json"
	"errors"
	"fmt"
	"os"
	"path/filepath"
	"unsafe"
)

// RawCoordSpan representa o elemento atômico mais direto de coordenada espacial
type RawCoordSpan struct {
	Page     int     `json:"page"`
	X0       float32 `json:"x0"`
	Y0       float32 `json:"y0"`
	X1       float32 `json:"x1"`
	Y1       float32 `json:"y1"`
	FontSize float32 `json:"font_size"`
	FontName string  `json:"font_name"`
	Text     string  `json:"text"`
}

// RawSpatialDocument é o documento plano contendo a lista pura de coordenadas X/Y
type RawSpatialDocument struct {
	FileName    string         `json:"file_name"`
	TotalPages  int            `json:"total_pages"`
	TotalSpans  int            `json:"total_spans"`
	ParseTimeMs float64        `json:"parse_time_ms"`
	Spans       []RawCoordSpan `json:"spans"`
}

// Options defines the parser configurations for structured mode
type Options struct {
	Mode            string  `json:"mode,omitempty"`
	DetectTables    bool    `json:"detect_tables"`
	MinTableColumns int     `json:"min_table_columns"`
	ColumnTolerance float32 `json:"column_tolerance"`
	RowTolerance    float32 `json:"row_tolerance"`
	// ExtractImages fills page.images with embedded image metadata.
	ExtractImages bool `json:"extract_images"`
	// ExtractDrawings fills page.drawings with vector paths.
	ExtractDrawings bool `json:"extract_drawings"`
	// FilterBackgroundFills drops fills covering the whole page. nil keeps the
	// native default (true); set to a pointer to false to keep backgrounds.
	FilterBackgroundFills *bool `json:"filter_background_fills,omitempty"`
	// ApplyRotation transforms visual coordinates into the rotated viewport.
	ApplyRotation bool `json:"apply_rotation"`
}

// DefaultOptions returns standard parsing options
func DefaultOptions() Options {
	return Options{
		Mode:            "structured",
		DetectTables:    true,
		MinTableColumns: 3,
		ColumnTolerance: 16.0,
		RowTolerance:    6.0,
		ApplyRotation:   false,
	}
}

// BoundingBox holds physical 2D coordinates in typographical points (1/72 inch)
type BoundingBox struct {
	X0     float32 `json:"x0"`
	Y0     float32 `json:"y0"`
	X1     float32 `json:"x1"`
	Y1     float32 `json:"y1"`
	Width  float32 `json:"width"`
	Height float32 `json:"height"`
}

// SpatialSpan represents text with font metadata
type SpatialSpan struct {
	Text      string      `json:"text"`
	BBox      BoundingBox `json:"bbox"`
	FontName  string      `json:"font_name"`
	FontSize  float32     `json:"font_size"`
	CharCount int         `json:"char_count"`
}

// SpatialLine represents a line of text
type SpatialLine struct {
	Text  string        `json:"text"`
	BBox  BoundingBox   `json:"bbox"`
	Spans []SpatialSpan `json:"spans"`
}

// SpatialBlock represents a block of text
type SpatialBlock struct {
	BlockID   int           `json:"block_id"`
	BlockType string        `json:"block_type"`
	BBox      BoundingBox   `json:"bbox"`
	Lines     []SpatialLine `json:"lines"`
}

// SpatialCell represents a single cell in an extracted table
type SpatialCell struct {
	Row  int         `json:"row"`
	Col  int         `json:"col"`
	Text string      `json:"text"`
	BBox BoundingBox `json:"bbox"`
}

// SpatialTable represents an extracted table
type SpatialTable struct {
	TableID  int           `json:"table_id"`
	BBox     BoundingBox   `json:"bbox"`
	ColCount int           `json:"col_count"`
	RowCount int           `json:"row_count"`
	Headers  []string      `json:"headers"`
	Rows     [][]string    `json:"rows"`
	Cells    []SpatialCell `json:"cells,omitempty"`
}

// SpatialImage represents an embedded image (/Subtype /Image) referenced by a page.
// Image bytes are never inlined: use ExtractImages to write the original streams.
type SpatialImage struct {
	ImageID          int          `json:"image_id"`
	Name             string       `json:"name"`
	Xref             int          `json:"xref"`
	BBox             *BoundingBox `json:"bbox"`
	PixelWidth       uint32       `json:"pixel_width"`
	PixelHeight      uint32       `json:"pixel_height"`
	BitsPerComponent *int         `json:"bits_per_component"`
	ColorSpace       *string      `json:"color_space"`
	Filter           *string      `json:"filter"`
	MimeType         *string      `json:"mime_type"`
	ByteLength       *int         `json:"byte_length"`
	Sha256           *string      `json:"sha256"`
	File             *string      `json:"file"`
}

// SpatialDrawingItem is one geometric item of a vector path, discriminated by Kind
// ("line", "curve", "rect" or "quad"). Unused fields are nil.
type SpatialDrawingItem struct {
	Kind        string       `json:"kind"`
	P1          []float32    `json:"p1,omitempty"`
	P2          []float32    `json:"p2,omitempty"`
	P3          []float32    `json:"p3,omitempty"`
	P4          []float32    `json:"p4,omitempty"`
	BBox        *BoundingBox `json:"bbox,omitempty"`
	Orientation *int         `json:"orientation,omitempty"`
	Points      [][]float32  `json:"points,omitempty"`
}

// SpatialDrawing represents a painted vector path (fill, stroke or both)
type SpatialDrawing struct {
	DrawingID     int                  `json:"drawing_id"`
	BBox          BoundingBox          `json:"bbox"`
	DrawingType   string               `json:"drawing_type"`
	ItemCount     int                  `json:"item_count"`
	Items         []SpatialDrawingItem `json:"items"`
	StrokeColor   []float32            `json:"stroke_color"`
	FillColor     []float32            `json:"fill_color"`
	Width         *float32             `json:"width"`
	FillOpacity   *float32             `json:"fill_opacity"`
	StrokeOpacity *float32             `json:"stroke_opacity"`
	EvenOdd       *bool                `json:"even_odd"`
	DashPattern   []float32            `json:"dash_pattern"`
	DashPhase     *float32             `json:"dash_phase"`
	LineJoin      *string              `json:"line_join"`
	LineCap       *string              `json:"line_cap"`
	Layer         *string              `json:"layer"`
	Seqno         int                  `json:"seqno"`
}

// ExtractedImageRecord pairs an extracted image with its 1-based page number
type ExtractedImageRecord struct {
	Page  int          `json:"page"`
	Image SpatialImage `json:"image"`
}

// ImageManifest is returned by ExtractImages
type ImageManifest struct {
	FileName    string                 `json:"file_name"`
	TotalImages int                    `json:"total_images"`
	Written     int                    `json:"written"`
	Images      []ExtractedImageRecord `json:"images"`
}

// SpatialPage holds elements for a single page
type SpatialPage struct {
	PageNumber       int              `json:"page_number"`
	Width            float32          `json:"width"`
	Height           float32          `json:"height"`
	Rotation         int              `json:"rotation"`
	InitialRotation  int              `json:"initial_rotation"`
	CoordinateSystem string           `json:"coordinate_system"`
	MediaBox         []float32        `json:"media_box,omitempty"`
	CropBox          []float32        `json:"crop_box,omitempty"`
	TextBlocks       []SpatialBlock   `json:"text_blocks"`
	Tables           []SpatialTable   `json:"tables"`
	RawSpanCount     int              `json:"raw_span_count"`
	Images           []SpatialImage   `json:"images"`
	Drawings         []SpatialDrawing `json:"drawings"`
}

// SpatialChunk represents a semantic chunk for RAG with constituent bounding boxes
type SpatialChunk struct {
	ChunkID          int           `json:"chunk_id"`
	PageNumber       int           `json:"page_number"`
	Text             string        `json:"text"`
	BBoxes           []BoundingBox `json:"bboxes"`
	OverallBBox      BoundingBox   `json:"overall_bbox"`
	BlockCount       int           `json:"block_count"`
	TokenEstimate    int           `json:"token_estimate"`
	CoordinateSystem string        `json:"coordinate_system"`
}

// SpatialDocument represents the entire parsed PDF document
type SpatialDocument struct {
	FileName    string        `json:"file_name"`
	TotalPages  int           `json:"total_pages"`
	ParseTimeMs float64       `json:"parse_time_ms"`
	Pages       []SpatialPage `json:"pages"`
}

// checkNativeError checks if the returned JSON represents an error object
func checkNativeError(data []byte) error {
	var errObj struct {
		Error   interface{} `json:"error"`
		Message string      `json:"message"`
	}
	if err := json.Unmarshal(data, &errObj); err == nil {
		if errObj.Error == true || errObj.Message != "" {
			msg := errObj.Message
			if msg == "" {
				msg = fmt.Sprintf("%v", errObj.Error)
			}
			return fmt.Errorf("rasgapdf native error: %s", msg)
		}
	}
	return nil
}

// ParseRaw parses a PDF and returns pure raw X/Y coordinate spans without block grouping
func ParseRaw(path string) (*RawSpatialDocument, error) {
	absPath, err := filepath.Abs(path)
	if err != nil {
		return nil, err
	}

	if _, err := os.Stat(absPath); os.IsNotExist(err) {
		return nil, errors.New("file not found: " + absPath)
	}

	cPath := C.CString(absPath)
	defer C.free(unsafe.Pointer(cPath))

	cRes := C.rasgapdf_parse_raw(cPath)
	if cRes == nil {
		return nil, errors.New("rasgapdf native parser returned null")
	}
	defer C.rasgapdf_free_string(cRes)

	bytes := []byte(C.GoString(cRes))
	if err := checkNativeError(bytes); err != nil {
		return nil, err
	}

	var doc RawSpatialDocument
	if err := json.Unmarshal(bytes, &doc); err != nil {
		return nil, err
	}

	return &doc, nil
}

// ParseRawBytes parses PDF directly from a byte slice in RAM
func ParseRawBytes(data []byte) (*RawSpatialDocument, error) {
	if len(data) == 0 {
		return nil, errors.New("empty PDF byte slice")
	}

	cRes := C.rasgapdf_parse_raw_bytes((*C.uint8_t)(unsafe.Pointer(&data[0])), C.size_t(len(data)))
	if cRes == nil {
		return nil, errors.New("rasgapdf native parser returned null")
	}
	defer C.rasgapdf_free_string(cRes)

	bytes := []byte(C.GoString(cRes))
	if err := checkNativeError(bytes); err != nil {
		return nil, err
	}

	var doc RawSpatialDocument
	if err := json.Unmarshal(bytes, &doc); err != nil {
		return nil, err
	}

	return &doc, nil
}

// ParseFile parses a PDF from a path and returns the SpatialDocument
func ParseFile(path string, opts *Options) (*SpatialDocument, error) {
	absPath, err := filepath.Abs(path)
	if err != nil {
		return nil, err
	}

	if _, err := os.Stat(absPath); os.IsNotExist(err) {
		return nil, errors.New("file not found: " + absPath)
	}

	cPath := C.CString(absPath)
	defer C.free(unsafe.Pointer(cPath))

	var cOpts *C.char
	if opts != nil {
		optsBytes, _ := json.Marshal(opts)
		cOpts = C.CString(string(optsBytes))
		defer C.free(unsafe.Pointer(cOpts))
	}

	cRes := C.rasgapdf_parse_file_options(cPath, cOpts)
	if cRes == nil {
		return nil, errors.New("rasgapdf native parser returned null")
	}
	defer C.rasgapdf_free_string(cRes)

	bytes := []byte(C.GoString(cRes))
	if err := checkNativeError(bytes); err != nil {
		return nil, err
	}

	var doc SpatialDocument
	if err := json.Unmarshal(bytes, &doc); err != nil {
		return nil, err
	}

	return &doc, nil
}

// ParsePdf is an alias to ParseFile for parity across language bindings
func ParsePdf(path string, opts *Options) (*SpatialDocument, error) {
	return ParseFile(path, opts)
}

// ExtractTextBytes extracts natural reading-order text directly from bytes in RAM
func ExtractTextBytes(data []byte) (string, error) {
	if len(data) == 0 {
		return "", errors.New("empty PDF byte slice")
	}

	cRes := C.rasgapdf_extract_text_bytes((*C.uint8_t)(unsafe.Pointer(&data[0])), C.size_t(len(data)))
	if cRes == nil {
		return "", errors.New("rasgapdf native text extractor returned null")
	}
	defer C.rasgapdf_free_string(cRes)

	return C.GoString(cRes), nil
}

// Version returns the native RasgaPDF library version
func Version() string {
	cVer := C.rasgapdf_version()
	if cVer == nil {
		return "0.3.3"
	}
	return C.GoString(cVer)
}

// ExtractImages writes the original encoded streams of every embedded image to
// outputDir and returns the generated manifest. The streams are copied verbatim
// (e.g. a DCTDecode image becomes a .jpg), so no pixels are decoded.
func ExtractImages(path string, outputDir string) (*ImageManifest, error) {
	absPath, err := filepath.Abs(path)
	if err != nil {
		return nil, err
	}

	if _, err := os.Stat(absPath); os.IsNotExist(err) {
		return nil, errors.New("file not found: " + absPath)
	}

	absOut, err := filepath.Abs(outputDir)
	if err != nil {
		return nil, err
	}

	cPath := C.CString(absPath)
	defer C.free(unsafe.Pointer(cPath))

	cOut := C.CString(absOut)
	defer C.free(unsafe.Pointer(cOut))

	cRes := C.rasgapdf_extract_images(cPath, cOut)
	if cRes == nil {
		return nil, errors.New("rasgapdf native parser returned null")
	}
	defer C.rasgapdf_free_string(cRes)

	bytes := []byte(C.GoString(cRes))
	if err := checkNativeError(bytes); err != nil {
		return nil, err
	}

	var manifest ImageManifest
	if err := json.Unmarshal(bytes, &manifest); err != nil {
		return nil, err
	}

	return &manifest, nil
}

// ExtractImagesBytes extracts original encoded image streams directly from RAM bytes to outputDir
func ExtractImagesBytes(data []byte, outputDir string) (*ImageManifest, error) {
	if len(data) == 0 {
		return nil, errors.New("empty PDF byte slice")
	}

	absOut, err := filepath.Abs(outputDir)
	if err != nil {
		return nil, err
	}

	cOut := C.CString(absOut)
	defer C.free(unsafe.Pointer(cOut))

	cRes := C.rasgapdf_extract_images_bytes((*C.uint8_t)(unsafe.Pointer(&data[0])), C.size_t(len(data)), cOut)
	if cRes == nil {
		return nil, errors.New("rasgapdf native parser returned null")
	}
	defer C.rasgapdf_free_string(cRes)

	bytes := []byte(C.GoString(cRes))
	if err := checkNativeError(bytes); err != nil {
		return nil, err
	}

	var manifest ImageManifest
	if err := json.Unmarshal(bytes, &manifest); err != nil {
		return nil, err
	}

	return &manifest, nil
}

// DeduplicateBBoxes removes duplicate bounding boxes within a tolerance (in points)
func DeduplicateBBoxes(bboxes []BoundingBox, tolerance float32) []BoundingBox {
	unique := make([]BoundingBox, 0, len(bboxes))
	for _, b := range bboxes {
		isDup := false
		for _, u := range unique {
			if diff(b.X0, u.X0) <= tolerance &&
				diff(b.Y0, u.Y0) <= tolerance &&
				diff(b.X1, u.X1) <= tolerance &&
				diff(b.Y1, u.Y1) <= tolerance {
				isDup = true
				break
			}
		}
		if !isDup {
			unique = append(unique, b)
		}
	}
	return unique
}

func diff(a, b float32) float32 {
	if a > b {
		return a - b
	}
	return b - a
}

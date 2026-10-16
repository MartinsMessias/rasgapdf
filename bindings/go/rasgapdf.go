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
	"os"
	"path/filepath"
	"unsafe"
)

// Options defines the parser configurations
type Options struct {
	DetectTables     bool    `json:"detect_tables"`
	MinTableColumns  int     `json:"min_table_columns"`
	ColumnTolerance  float32 `json:"column_tolerance"`
	RowTolerance     float32 `json:"row_tolerance"`
}

// DefaultOptions returns standard parsing options
func DefaultOptions() Options {
	return Options{
		DetectTables:    true,
		MinTableColumns: 3,
		ColumnTolerance: 16.0,
		RowTolerance:    6.0,
	}
}

// BoundingBox holds physical 2D coordinates
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

// SpatialTable represents an extracted table
type SpatialTable struct {
	TableID  int        `json:"table_id"`
	BBox     BoundingBox `json:"bbox"`
	ColCount int        `json:"col_count"`
	RowCount int        `json:"row_count"`
	Headers  []string   `json:"headers"`
	Rows     [][]string `json:"rows"`
}

// SpatialPage holds elements for a single page
type SpatialPage struct {
	PageNumber   int            `json:"page_number"`
	Width        float32        `json:"width"`
	Height       float32        `json:"height"`
	Rotation     int            `json:"rotation"`
	TextBlocks   []SpatialBlock `json:"text_blocks"`
	Tables       []SpatialTable `json:"tables"`
	RawSpanCount int            `json:"raw_span_count"`
}

// SpatialDocument represents the entire parsed PDF document
type SpatialDocument struct {
	FileName     string        `json:"file_name"`
	TotalPages   int           `json:"total_pages"`
	ParseTimeMs  float64       `json:"parse_time_ms"`
	Pages        []SpatialPage `json:"pages"`
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

	goStr := C.GoString(cRes)

	var doc SpatialDocument
	if err := json.Unmarshal([]byte(goStr), &doc); err != nil {
		return nil, err
	}

	return &doc, nil
}

// Version returns the native RasgaPDF library version
func Version() string {
	return C.GoString(C.rasgapdf_version())
}

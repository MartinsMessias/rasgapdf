package rasgapdf

import (
	"os"
	"path/filepath"
	"testing"
)

func findFixture(t *testing.T, name string) string {
	candidates := []string{
		filepath.Join("..", "..", "tests", "fixtures", name),
		filepath.Join("..", "..", "docs", name),
	}
	for _, c := range candidates {
		if abs, err := filepath.Abs(c); err == nil {
			if _, err := os.Stat(abs); err == nil {
				return abs
			}
		}
	}
	t.Skipf("Fixture %s not found", name)
	return ""
}

func TestVersion(t *testing.T) {
	v := Version()
	if v == "" {
		t.Fatal("Version() returned empty string")
	}
}

func TestParseRaw(t *testing.T) {
	fix := findFixture(t, "mixed.pdf")
	doc, err := ParseRaw(fix)
	if err != nil {
		t.Fatalf("ParseRaw failed: %v", err)
	}
	if doc.TotalPages == 0 {
		t.Errorf("Expected TotalPages > 0, got %d", doc.TotalPages)
	}
	if doc.TotalSpans == 0 {
		t.Errorf("Expected TotalSpans > 0, got %d", doc.TotalSpans)
	}
}

func TestParseFileAndPdfAlias(t *testing.T) {
	fix := findFixture(t, "mixed.pdf")
	opts := DefaultOptions()
	opts.DetectTables = true
	opts.ApplyRotation = true

	doc1, err := ParseFile(fix, &opts)
	if err != nil {
		t.Fatalf("ParseFile failed: %v", err)
	}
	if len(doc1.Pages) == 0 {
		t.Fatal("Expected at least 1 page")
	}

	doc2, err := ParsePdf(fix, &opts)
	if err != nil {
		t.Fatalf("ParsePdf alias failed: %v", err)
	}
	if len(doc2.Pages) != len(doc1.Pages) {
		t.Errorf("ParsePdf returned %d pages, expected %d", len(doc2.Pages), len(doc1.Pages))
	}
}

func TestBytesAPIs(t *testing.T) {
	fix := findFixture(t, "mixed.pdf")
	bytes, err := os.ReadFile(fix)
	if err != nil {
		t.Fatalf("Failed to read fixture: %v", err)
	}

	rawDoc, err := ParseRawBytes(bytes)
	if err != nil {
		t.Fatalf("ParseRawBytes failed: %v", err)
	}
	if rawDoc.TotalSpans == 0 {
		t.Errorf("Expected TotalSpans > 0 from ParseRawBytes")
	}

	text, err := ExtractTextBytes(bytes)
	if err != nil {
		t.Fatalf("ExtractTextBytes failed: %v", err)
	}
	if len(text) == 0 {
		t.Errorf("Expected non-empty text from ExtractTextBytes")
	}
}

func TestMalformedBytesError(t *testing.T) {
	_, err := ParseRawBytes([]byte{})
	if err == nil {
		t.Error("Expected error on empty byte slice")
	}

	_, err = ParseRawBytes([]byte("NOT_A_PDF"))
	if err == nil {
		t.Error("Expected error on invalid PDF bytes")
	}
}

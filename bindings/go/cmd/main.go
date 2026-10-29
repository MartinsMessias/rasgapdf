package main

import (
	"fmt"
	"log"
	"os"

	"github.com/MartinsMessias/rasgapdf/bindings/go"
)

func main() {
	pdfPath := "../../demo_gradio/sample.pdf"
	if len(os.Args) > 1 {
		pdfPath = os.Args[1]
	}

	fmt.Println("=== RasgaPDF Go Native Test ===")
	fmt.Println("Version:", rasgapdf.Version())

	// 1. Teste Raw
	rawDoc, err := rasgapdf.ParseRaw(pdfPath)
	if err != nil {
		log.Fatalf("Error parsing PDF raw: %v", err)
	}
	fmt.Printf("⚡ [Modo Raw]: %d spans extraídos em %.2f ms\n", rawDoc.TotalSpans, rawDoc.ParseTimeMs)
	if len(rawDoc.Spans) > 0 {
		s := rawDoc.Spans[0]
		fmt.Printf("   Primeiro span: (X0: %.2f, Y0: %.2f) -> %q\n", s.X0, s.Y0, s.Text)
	}

	// 2. Teste Estruturado
	opts := rasgapdf.DefaultOptions()
	doc, err := rasgapdf.ParseFile(pdfPath, &opts)
	if err != nil {
		log.Fatalf("Error parsing PDF structured: %v", err)
	}

	fmt.Printf("🎯 [Modo Estruturado]: %d blocos em %.2f ms\n", len(doc.Pages[0].TextBlocks), doc.ParseTimeMs)
}

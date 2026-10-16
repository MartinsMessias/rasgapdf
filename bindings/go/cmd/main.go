package main

import (
	"fmt"
	"log"

	"github.com/messiasmartins/rasgapdf/bindings/go"
)

func main() {
	fmt.Println("RasgaPDF Go Native Version:", rasgapdf.Version())

	opts := rasgapdf.DefaultOptions()
	doc, err := rasgapdf.ParseFile("/home/messias_martins/poc_pdf_c/artigo_ia.pdf", &opts)
	if err != nil {
		log.Fatalf("Error parsing PDF: %v", err)
	}

	fmt.Printf("Document: %s\n", doc.FileName)
	fmt.Printf("Total Pages: %d\n", doc.TotalPages)
	fmt.Printf("Parse Time: %.2f ms\n", doc.ParseTimeMs)
	if len(doc.Pages) > 0 && len(doc.Pages[0].TextBlocks) > 0 {
		fmt.Printf("First Block: %s\n", doc.Pages[0].TextBlocks[0].Lines[0].Text)
	}
}

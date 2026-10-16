use std::fs::File;
use std::io::Write;
use clap::{Parser, ValueEnum};
use rasgapdf::{parse_file, parse_raw_file, ParserOptions};

#[derive(ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
pub enum OutputFormat {
    Json,
    Text,
    Csv,
    Tsv,
}

#[derive(Parser, Debug)]
#[command(
    name = "rasgapdf",
    author = "Messias Martins",
    version = env!("CARGO_PKG_VERSION"),
    about = "⚡ Blazing-fast 2D Spatial PDF Parser in Pure Rust",
    long_about = "Extracts exact physical 2D coordinates (X/Y) or structured blocks/tables from PDF files at native hardware speeds."
)]
struct Cli {
    /// Caminho para o arquivo PDF
    #[arg(value_name = "PDF_FILE")]
    input: String,

    /// Ativar Modo Bruto: retorna exclusivamente a lista plana de coordenadas X/Y sem overhead de blocos/tabelas
    #[arg(short, long, default_value_t = false)]
    raw: bool,

    /// Formato de saída: json, text, csv, tsv (padrão: json)
    #[arg(short, long, value_enum, default_value_t = OutputFormat::Json)]
    format: OutputFormat,

    /// Arquivo de saída para salvar o resultado (padrão: stdout)
    #[arg(short, long, value_name = "OUTPUT_FILE")]
    output: Option<String>,

    /// Formatar JSON com identação legível
    #[arg(short, long, default_value_t = false)]
    pretty: bool,

    /// Desativar detector automático de tabelas (apenas no modo estruturado)
    #[arg(long, default_value_t = false)]
    no_tables: bool,

    /// Tolerância horizontal para agrupamento de colunas (em pontos)
    #[arg(long, default_value_t = 16.0)]
    col_tolerance: f32,

    /// Tolerância vertical para agrupamento de linhas (em pontos)
    #[arg(long, default_value_t = 6.0)]
    row_tolerance: f32,

    /// Quantidade mínima de colunas para considerar uma tabela
    #[arg(long, default_value_t = 3)]
    min_table_cols: usize,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Cli::parse();

    if args.raw {
        // --- MODO 1: COORDENADAS BRUTAS (RAW X/Y) ---
        let raw_doc = match parse_raw_file(&args.input) {
            Ok(d) => d,
            Err(e) => {
                eprintln!("❌ Erro ao processar PDF (modo raw): {}", e);
                std::process::exit(1);
            }
        };

        let result_string = match args.format {
            OutputFormat::Json => {
                if args.pretty {
                    serde_json::to_string_pretty(&raw_doc)?
                } else {
                    serde_json::to_string(&raw_doc)?
                }
            }
            OutputFormat::Text => {
                let mut out = String::new();
                out.push_str(&format!(
                    "# RasgaPDF Raw Spatial Dump - Arquivo: {} | Páginas: {} | Spans: {} | Tempo: {:.2} ms\n",
                    raw_doc.file_name, raw_doc.total_pages, raw_doc.total_spans, raw_doc.parse_time_ms
                ));
                for s in &raw_doc.spans {
                    out.push_str(&format!(
                        "[Pág. {:2}] (X0: {:7.2}, Y0: {:7.2}, X1: {:7.2}, Y1: {:7.2}) [{:4.1}pt] {}\n",
                        s.page, s.x0, s.y0, s.x1, s.y1, s.font_size, s.text
                    ));
                }
                out
            }
            OutputFormat::Csv => {
                let mut out = String::from("page,x0,y0,x1,y1,font_size,font_name,text\n");
                for s in &raw_doc.spans {
                    let escaped_text = s.text.replace('"', "\"\"");
                    out.push_str(&format!(
                        "{},{:.2},{:.2},{:.2},{:.2},{:.1},\"{}\",\"{}\"\n",
                        s.page, s.x0, s.y0, s.x1, s.y1, s.font_size, s.font_name, escaped_text
                    ));
                }
                out
            }
            OutputFormat::Tsv => {
                let mut out = String::from("page\tx0\ty0\tx1\ty1\tfont_size\tfont_name\ttext\n");
                for s in &raw_doc.spans {
                    let sanitized = s.text.replace('\t', " ").replace('\n', " ");
                    out.push_str(&format!(
                        "{}\t{:.2}\t{:.2}\t{:.2}\t{:.2}\t{:.1}\t{}\t{}\n",
                        s.page, s.x0, s.y0, s.x1, s.y1, s.font_size, s.font_name, sanitized
                    ));
                }
                out
            }
        };

        if let Some(out_path) = args.output {
            let mut file = File::create(&out_path)?;
            file.write_all(result_string.as_bytes())?;
            eprintln!(
                "⚡ Extração bruta (raw X/Y) concluída em {:.2} ms ({} spans em {} páginas) -> Salvo em {}",
                raw_doc.parse_time_ms, raw_doc.total_spans, raw_doc.total_pages, out_path
            );
        } else {
            println!("{}", result_string);
            eprintln!(
                "⚡ Raw X/Y extraído em {:.2} ms | Total spans: {} | Páginas: {}",
                raw_doc.parse_time_ms, raw_doc.total_spans, raw_doc.total_pages
            );
        }
    } else {
        // --- MODO 2: ESTRUTURADO (HIERÁRQUICO COM BLOCOS E TABELAS) ---
        let options = ParserOptions {
            mode: "structured".to_string(),
            detect_tables: !args.no_tables,
            min_table_columns: args.min_table_cols,
            column_tolerance: args.col_tolerance,
            row_tolerance: args.row_tolerance,
        };

        let doc = match parse_file(&args.input, &options) {
            Ok(d) => d,
            Err(e) => {
                eprintln!("❌ Erro ao processar PDF: {}", e);
                std::process::exit(1);
            }
        };

        let result_string = match args.format {
            OutputFormat::Json => {
                if args.pretty {
                    serde_json::to_string_pretty(&doc)?
                } else {
                    serde_json::to_string(&doc)?
                }
            }
            OutputFormat::Text => {
                let mut out = String::new();
                out.push_str(&format!(
                    "# RasgaPDF Structured Dump - Arquivo: {} | Páginas: {} | Tempo: {:.2} ms\n",
                    doc.file_name, doc.total_pages, doc.parse_time_ms
                ));
                for page in &doc.pages {
                    out.push_str(&format!("\n=== Página {} ({}x{}) ===\n", page.page_number, page.width, page.height));
                    if !page.tables.is_empty() {
                        out.push_str(&format!("  [Tabelas Detectadas: {}]\n", page.tables.len()));
                        for (idx, tbl) in page.tables.iter().enumerate() {
                            out.push_str(&format!("    Tabela #{}: {} linhas x {} colunas (Bbox: {:?})\n", idx + 1, tbl.rows.len(), tbl.col_count, tbl.bbox));
                        }
                    }
                    out.push_str(&format!("  [Blocos de Texto: {}]\n", page.text_blocks.len()));
                    for block in &page.text_blocks {
                        out.push_str(&format!("    Bloco #{}: {:?}\n", block.block_id, block.bbox));
                        for line in &block.lines {
                            out.push_str(&format!("      Linha: {:?} -> \"{}\"\n", line.bbox, line.text));
                        }
                    }
                }
                out
            }
            OutputFormat::Csv | OutputFormat::Tsv => {
                eprintln!("⚠️ Formatos CSV e TSV diretos são recomendados no modo `--raw`. Alternando para listagem plana...");
                let raw_doc = parse_raw_file(&args.input).map_err(|e| e.to_string())?;
                if args.format == OutputFormat::Csv {
                    let mut out = String::from("page,x0,y0,x1,y1,font_size,font_name,text\n");
                    for s in &raw_doc.spans {
                        let escaped_text = s.text.replace('"', "\"\"");
                        out.push_str(&format!(
                            "{},{:.2},{:.2},{:.2},{:.2},{:.1},\"{}\",\"{}\"\n",
                            s.page, s.x0, s.y0, s.x1, s.y1, s.font_size, s.font_name, escaped_text
                        ));
                    }
                    out
                } else {
                    let mut out = String::from("page\tx0\ty0\tx1\ty1\tfont_size\tfont_name\ttext\n");
                    for s in &raw_doc.spans {
                        let sanitized = s.text.replace('\t', " ").replace('\n', " ");
                        out.push_str(&format!(
                            "{}\t{:.2}\t{:.2}\t{:.2}\t{:.2}\t{:.1}\t{}\t{}\n",
                            s.page, s.x0, s.y0, s.x1, s.y1, s.font_size, s.font_name, sanitized
                        ));
                    }
                    out
                }
            }
        };

        if let Some(out_path) = args.output {
            let mut file = File::create(&out_path)?;
            file.write_all(result_string.as_bytes())?;
            eprintln!(
                "✅ Extração estruturada concluída em {:.2} ms ({} páginas) -> Salvo em {}",
                doc.parse_time_ms, doc.total_pages, out_path
            );
        } else {
            println!("{}", result_string);
            eprintln!(
                "⚡ Estruturado em {:.2} ms | Páginas: {} | Arquivo: {}",
                doc.parse_time_ms, doc.total_pages, doc.file_name
            );
        }
    }

    Ok(())
}

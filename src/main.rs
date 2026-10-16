use std::fs::File;
use std::io::Write;
use clap::Parser;
use rasgapdf::{parse_file, ParserOptions};

#[derive(Parser, Debug)]
#[command(
    name = "rasgapdf",
    author = "Messias Martins",
    version = env!("CARGO_PKG_VERSION"),
    about = "⚡ Blazing-fast 2D Spatial PDF Parser in Pure Rust",
    long_about = "Extracts exact physical bounding boxes, text lines, spans, and tabular data from PDF files at native hardware speeds."
)]
struct Cli {
    /// Caminho para o arquivo PDF
    #[arg(value_name = "PDF_FILE")]
    input: String,

    /// Arquivo de saída para salvar o JSON resultante (padrão: stdout)
    #[arg(short, long, value_name = "OUTPUT_FILE")]
    output: Option<String>,

    /// Formatar JSON com identação legível
    #[arg(short, long, default_value_t = false)]
    pretty: bool,

    /// Desativar detector automático de tabelas
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

    let options = ParserOptions {
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

    let json_output = if args.pretty {
        serde_json::to_string_pretty(&doc)?
    } else {
        serde_json::to_string(&doc)?
    };

    if let Some(out_path) = args.output {
        let mut file = File::create(&out_path)?;
        file.write_all(json_output.as_bytes())?;
        eprintln!(
            "✅ Extração espacial concluída em {:.2} ms ({} páginas) -> Salvo em {}",
            doc.parse_time_ms, doc.total_pages, out_path
        );
    } else {
        println!("{}", json_output);
        eprintln!(
            "⚡ Processado em {:.2} ms | Páginas: {} | Arquivo: {}",
            doc.parse_time_ms, doc.total_pages, doc.file_name
        );
    }

    Ok(())
}

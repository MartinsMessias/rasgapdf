use crate::types::{BoundingBox, SpatialCell, SpatialTable};

#[derive(Debug, Clone)]
pub struct RawSpan {
    pub x0: f32,
    pub y0: f32,
    pub x1: f32,
    pub y1: f32,
    pub font_size: f32,
    pub text: String,
}

pub struct TableDetector {
    pub min_cols: usize,
    pub col_tolerance: f32,
    pub row_tolerance: f32,
}

impl TableDetector {
    pub fn new(min_cols: usize, col_tolerance: f32, row_tolerance: f32) -> Self {
        Self {
            min_cols,
            col_tolerance,
            row_tolerance,
        }
    }

    pub fn extract_tables(&self, spans: &[RawSpan], table_id_start: usize) -> (Vec<SpatialTable>, Vec<RawSpan>) {
        if spans.is_empty() {
            return (Vec::new(), Vec::new());
        }

        // Agrupa spans por linha horizontal (y bucket)
        let mut lines: Vec<Vec<RawSpan>> = Vec::new();
        let mut current_line: Vec<RawSpan> = Vec::new();
        let mut current_y = spans[0].y0;

        for span in spans {
            if (span.y0 - current_y).abs() <= self.row_tolerance {
                current_line.push(span.clone());
            } else {
                if !current_line.is_empty() {
                    current_line.sort_by(|a, b| a.x0.partial_cmp(&b.x0).unwrap_or(std::cmp::Ordering::Equal));
                    lines.push(current_line);
                }
                current_line = vec![span.clone()];
                current_y = span.y0;
            }
        }
        if !current_line.is_empty() {
            current_line.sort_by(|a, b| a.x0.partial_cmp(&b.x0).unwrap_or(std::cmp::Ordering::Equal));
            lines.push(current_line);
        }

        let mut detected_tables = Vec::new();
        let mut non_table_spans = Vec::new();
        let mut active_table_rows: Vec<Vec<RawSpan>> = Vec::new();
        let mut active_cols: Vec<f32> = Vec::new();
        let mut table_counter = table_id_start;

        for line in lines {
            if line.len() >= self.min_cols {
                // Inicia ou continua uma tabela
                if active_cols.is_empty() {
                    active_cols = line.iter().map(|s| s.x0).collect();
                    active_table_rows.push(line);
                } else {
                    // Verifica se alinha com as colunas existentes
                    let aligns = line.iter().any(|s| {
                        active_cols.iter().any(|&c| (c - s.x0).abs() <= self.col_tolerance)
                    });

                    if aligns {
                        active_table_rows.push(line);
                    } else {
                        // Fecha a tabela anterior
                        if active_table_rows.len() >= 2 {
                            let table = self.build_table(&active_table_rows, &active_cols, table_counter);
                            detected_tables.push(table);
                            table_counter += 1;
                        } else {
                            for r in active_table_rows.drain(..) {
                                non_table_spans.extend(r);
                            }
                        }
                        active_table_rows = vec![line];
                        active_cols = active_table_rows[0].iter().map(|s| s.x0).collect();
                    }
                }
            } else {
                // Linha de texto normal
                if !active_table_rows.is_empty() {
                    if active_table_rows.len() >= 2 {
                        let table = self.build_table(&active_table_rows, &active_cols, table_counter);
                        detected_tables.push(table);
                        table_counter += 1;
                    } else {
                        for r in active_table_rows.drain(..) {
                            non_table_spans.extend(r);
                        }
                    }
                    active_table_rows.clear();
                    active_cols.clear();
                }
                non_table_spans.extend(line);
            }
        }

        // Tabela no final do documento
        if active_table_rows.len() >= 2 {
            let table = self.build_table(&active_table_rows, &active_cols, table_counter);
            detected_tables.push(table);
        } else {
            for r in active_table_rows.drain(..) {
                non_table_spans.extend(r);
            }
        }

        (detected_tables, non_table_spans)
    }

    fn build_table(&self, rows_raw: &[Vec<RawSpan>], cols_x: &[f32], table_id: usize) -> SpatialTable {
        let mut min_x = f32::MAX;
        let mut min_y = f32::MAX;
        let mut max_x = f32::MIN;
        let mut max_y = f32::MIN;

        let num_cols = cols_x.len();
        let mut structured_rows: Vec<Vec<String>> = Vec::new();
        let mut cells: Vec<SpatialCell> = Vec::new();

        for (row_idx, row_spans) in rows_raw.iter().enumerate() {
            let mut row_strings = vec![String::new(); num_cols];

            for span in row_spans {
                min_x = min_x.min(span.x0);
                min_y = min_y.min(span.y0);
                max_x = max_x.max(span.x1);
                max_y = max_y.max(span.y1);

                // Encontra melhor coluna
                let mut best_col = 0;
                let mut min_dist = f32::MAX;
                for (c_idx, &cx) in cols_x.iter().enumerate() {
                    let d = (cx - span.x0).abs();
                    if d < min_dist {
                        min_dist = d;
                        best_col = c_idx;
                    }
                }

                if !row_strings[best_col].is_empty() {
                    row_strings[best_col].push(' ');
                }
                row_strings[best_col].push_str(span.text.trim());

                cells.push(SpatialCell {
                    row: row_idx,
                    col: best_col,
                    text: span.text.trim().to_string(),
                    bbox: BoundingBox::new(span.x0, span.y0, span.x1, span.y1),
                });
            }

            structured_rows.push(row_strings);
        }

        let headers = if !structured_rows.is_empty() {
            structured_rows[0].clone()
        } else {
            Vec::new()
        };

        SpatialTable {
            table_id,
            bbox: BoundingBox::new(min_x, min_y, max_x, max_y),
            col_count: num_cols,
            row_count: rows_raw.len(),
            headers,
            rows: structured_rows,
            cells,
        }
    }
}

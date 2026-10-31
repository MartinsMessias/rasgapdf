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
            min_cols: min_cols.max(1),
            col_tolerance: if col_tolerance.is_finite() && col_tolerance >= 0.0 {
                col_tolerance
            } else {
                3.0
            },
            row_tolerance: if row_tolerance.is_finite() && row_tolerance >= 0.0 {
                row_tolerance
            } else {
                2.0
            },
        }
    }

    pub fn extract_tables(&self, spans: &[RawSpan], table_id_start: usize) -> (Vec<SpatialTable>, Vec<RawSpan>) {
        let valid_spans: Vec<RawSpan> = spans
            .iter()
            .filter(|s| s.x0.is_finite() && s.y0.is_finite() && s.x1.is_finite() && s.y1.is_finite())
            .cloned()
            .collect();

        if valid_spans.is_empty() {
            return (Vec::new(), Vec::new());
        }

        // Agrupa spans por linha horizontal (y bucket)
        let mut lines: Vec<Vec<RawSpan>> = Vec::new();
        let mut current_line: Vec<RawSpan> = Vec::new();
        let mut current_y = valid_spans[0].y0;

        for span in &valid_spans {
            if (span.y0 - current_y).abs() <= self.row_tolerance {
                current_line.push(span.clone());
            } else {
                if !current_line.is_empty() {
                    current_line.sort_by(|a, b| a.x0.partial_cmp(&b.x0).unwrap_or(std::cmp::Ordering::Equal));
                    lines.push(Self::coalesce_line_spans(&current_line, self.col_tolerance));
                }
                current_line = vec![span.clone()];
                current_y = span.y0;
            }
        }
        if !current_line.is_empty() {
            current_line.sort_by(|a, b| a.x0.partial_cmp(&b.x0).unwrap_or(std::cmp::Ordering::Equal));
            lines.push(Self::coalesce_line_spans(&current_line, self.col_tolerance));
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
                            table_counter = table_counter.saturating_add(1);
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
                        table_counter = table_counter.saturating_add(1);
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
        let num_cols = cols_x.len();
        if num_cols == 0 || rows_raw.is_empty() {
            return SpatialTable {
                table_id,
                bbox: BoundingBox::new(0.0, 0.0, 0.0, 0.0),
                col_count: 0,
                row_count: 0,
                headers: Vec::new(),
                rows: Vec::new(),
                cells: Vec::new(),
            };
        }

        let mut min_x = f32::MAX;
        let mut min_y = f32::MAX;
        let mut max_x = f32::MIN;
        let mut max_y = f32::MIN;
        let mut has_points = false;

        let mut structured_rows: Vec<Vec<String>> = Vec::with_capacity(rows_raw.len());
        let mut cells: Vec<SpatialCell> = Vec::new();

        for (row_idx, row_spans) in rows_raw.iter().enumerate() {
            let mut row_strings = vec![String::new(); num_cols];

            for span in row_spans {
                if span.x0.is_finite() && span.y0.is_finite() && span.x1.is_finite() && span.y1.is_finite() {
                    min_x = min_x.min(span.x0);
                    min_y = min_y.min(span.y0);
                    max_x = max_x.max(span.x1);
                    max_y = max_y.max(span.y1);
                    has_points = true;
                }

                // Encontra melhor coluna
                let mut best_col = 0;
                let mut min_dist = f32::MAX;
                for (c_idx, &cx) in cols_x.iter().enumerate() {
                    let d = (cx - span.x0).abs();
                    if d.is_finite() && d < min_dist {
                        min_dist = d;
                        best_col = c_idx;
                    }
                }

                if best_col < num_cols {
                    if !row_strings[best_col].is_empty() {
                        row_strings[best_col].push(' ');
                    }
                    row_strings[best_col].push_str(span.text.trim());
                }

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

        let bbox = if has_points && min_x <= max_x && min_y <= max_y {
            BoundingBox::new(min_x, min_y, max_x, max_y)
        } else {
            BoundingBox::new(0.0, 0.0, 0.0, 0.0)
        };

        SpatialTable {
            table_id,
            bbox,
            col_count: num_cols,
            row_count: rows_raw.len(),
            headers,
            rows: structured_rows,
            cells,
        }
    }

    fn coalesce_line_spans(line: &[RawSpan], col_tolerance: f32) -> Vec<RawSpan> {
        if line.is_empty() {
            return Vec::new();
        }
        let mut items = Vec::new();
        let mut curr = line[0].clone();

        for span in &line[1..] {
            let gap = span.x0 - curr.x1;
            if gap.is_finite() && gap < col_tolerance {
                if !curr.text.ends_with(' ') && !span.text.starts_with(' ') {
                    curr.text.push(' ');
                }
                curr.text.push_str(&span.text);
                curr.x1 = curr.x1.max(span.x1);
                curr.y0 = curr.y0.min(span.y0);
                curr.y1 = curr.y1.max(span.y1);
            } else {
                items.push(curr);
                curr = span.clone();
            }
        }
        items.push(curr);
        items
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_table_detector_defaults_and_zero_min_cols() {
        let detector = TableDetector::new(0, f32::NAN, -5.0);
        assert_eq!(detector.min_cols, 1);
        assert_eq!(detector.col_tolerance, 3.0);
        assert_eq!(detector.row_tolerance, 2.0);

        // Empty spans returns empty results
        let (tables, non_tables) = detector.extract_tables(&[], 1);
        assert!(tables.is_empty());
        assert!(non_tables.is_empty());
    }

    #[test]
    fn test_table_detector_with_nan_and_inf_coordinates() {
        let detector = TableDetector::new(2, 5.0, 5.0);
        let spans = vec![
            RawSpan {
                x0: f32::NAN,
                y0: 10.0,
                x1: 50.0,
                y1: 20.0,
                font_size: 10.0,
                text: "NaN Col".into(),
            },
            RawSpan {
                x0: 10.0,
                y0: f32::INFINITY,
                x1: 50.0,
                y1: 20.0,
                font_size: 10.0,
                text: "Inf Row".into(),
            },
            RawSpan {
                x0: 10.0,
                y0: 100.0,
                x1: 50.0,
                y1: 110.0,
                font_size: 10.0,
                text: "Valid 1".into(),
            },
            RawSpan {
                x0: 60.0,
                y0: 100.0,
                x1: 100.0,
                y1: 110.0,
                font_size: 10.0,
                text: "Valid 2".into(),
            },
            RawSpan {
                x0: 10.0,
                y0: 120.0,
                x1: 50.0,
                y1: 130.0,
                font_size: 10.0,
                text: "Valid 3".into(),
            },
            RawSpan {
                x0: 60.0,
                y0: 120.0,
                x1: 100.0,
                y1: 130.0,
                font_size: 10.0,
                text: "Valid 4".into(),
            },
        ];

        let (tables, _) = detector.extract_tables(&spans, 1);
        assert_eq!(tables.len(), 1);
        assert_eq!(tables[0].col_count, 2);
        assert_eq!(tables[0].row_count, 2);
        assert!(tables[0].bbox.width > 0.0);
        assert!(tables[0].bbox.width.is_finite());
    }

    #[test]
    fn test_build_table_empty_safeguard() {
        let detector = TableDetector::new(2, 5.0, 5.0);
        let table = detector.build_table(&[], &[], 1);
        assert_eq!(table.col_count, 0);
        assert_eq!(table.row_count, 0);
        assert_eq!(table.bbox.width, 0.0);
    }
}

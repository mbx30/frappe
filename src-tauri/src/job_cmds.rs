//! Job production commands (#84).
//!
//! Job ticket generation and approval sheets.

use crate::security;

#[tauri::command]
pub fn generate_job_ticket(
    job_id: String,
    customer_name: String,
    product_name: String,
    quantity: i64,
    due_date: String,
    print_type: String,
    paper_stock: String,
    finishing: String,
    files: Vec<String>,
    notes: String,
    output_path: String,
) -> Result<(), String> {
    let output_path = security::validate_write_path(&output_path)?;
    let input = crate::pdf::ticket::JobTicketInput {
        job_id,
        customer_name,
        product_name,
        quantity,
        due_date,
        print_type,
        paper_stock,
        finishing,
        files,
        notes,
    };
    crate::pdf::ticket::generate_job_ticket(&input, &output_path.to_string_lossy())
}

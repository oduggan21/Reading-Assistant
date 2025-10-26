//! services/api/src/bin/openapi.rs
//!
//! This binary generates the OpenAPI 3.0 specification for the REST API
//! and saves it to a file named `openapi.json`.

use api_lib::web::rest::ApiDoc;
use utoipa::OpenApi;

/// Generates the OpenAPI specification and writes it to a file.
fn generate_spec(
    api_doc: utoipa::openapi::OpenApi,
    path: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let spec_json = api_doc.to_pretty_json()?;
    std::fs::write(path, spec_json)?;
    println!("✅ OpenAPI specification generated at {}", path);
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Generate the spec from the shared ApiDoc and save it to `openapi.json`.
    generate_spec(ApiDoc::openapi(), "openapi.json")?;
    Ok(())
}

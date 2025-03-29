use skip_swap_valence_strategist::strategist::Strategist;

#[cfg(feature = "runtime")]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    Strategist::run_from_args().await?;
    Ok(())
}

#[cfg(not(feature = "runtime"))]
fn main() {
    eprintln!("Please enable the 'runtime' feature to run the strategist binary");
    std::process::exit(1);
} 
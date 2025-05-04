use cfx_natives_data::{GamesType, NativeParser};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Instant;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    std::fs::create_dir_all("fetched")?;
    let parser = NativeParser::new();
    let natives_count = Arc::new(AtomicUsize::new(0));
    let start_time_docs = Instant::now();
    let gta5_docs = parser.fetch_markdown_filenames(GamesType::GTA5).await?;
    let cfx_docs = parser.fetch_markdown_filenames(GamesType::CFX).await?;
    let docs_join = [gta5_docs, cfx_docs].concat();
    println!("Time taken to fetch docs: {:?} | {} Total docs fetched", start_time_docs.elapsed(), docs_join.len());
    
    let start_time = Instant::now();
    let handles = vec![
        {
            let parser = parser.clone();
            let count = natives_count.clone();
            let docs_join = docs_join.clone();
            tokio::spawn(async move {
                if let Ok(n) = parser
                    .save_natives(GamesType::GTA5, "fetched/natives.gta5.json", docs_join)
                    .await
                {
                    count.fetch_add(n, Ordering::SeqCst);
                }
            })
        },
        {
            let parser = parser.clone();
            let count = natives_count.clone();
            let docs_join = docs_join.clone();
            tokio::spawn(async move {
                if let Ok(n) = parser
                    .save_natives(GamesType::RDR3, "fetched/natives.rdr3.json", docs_join)
                    .await
                {
                    count.fetch_add(n, Ordering::SeqCst);
                }
            })
        },
        {
            let parser = parser.clone();
            let count = natives_count.clone();
            tokio::spawn(async move {
                if let Ok(n) = parser
                    .save_natives(GamesType::CFX, "fetched/natives.cfx.json", docs_join)
                    .await
                {
                    count.fetch_add(n, Ordering::SeqCst);
                }
            })
        },
    ];
    
    for handle in handles {
        handle.await?;
    }

    let total_count = natives_count.load(Ordering::SeqCst);
    println!(
        "Natives fetched successfully! Time taken: {:?} | {} Total natives fetched",
        start_time.elapsed(),
        total_count
    );

    let start_time = Instant::now();
    let (gta5_count, rdr3_count) = parser.organize_natives_by_apiset().await?;
    println!(
        "Natives organized successfully! Time taken: {:?} | {} Total natives organized | {} Natives GTA5 | {} Natives RDR3",
        start_time.elapsed(),
        gta5_count + rdr3_count,
        gta5_count,
        rdr3_count
    );

    Ok(())
}

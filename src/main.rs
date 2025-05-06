use cfx_natives_data::{GamesType, NativeParser, load_or_create_metadata};
use std::sync::{Arc, atomic::{AtomicUsize, Ordering}};
use serde_json;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("Starting...");
    std::fs::create_dir_all("fetched")?;
    std::fs::create_dir_all("plugin")?;
    std::fs::create_dir_all("plugin/json")?;

    let parser = NativeParser::new();
    let mut metadata = load_or_create_metadata().await?;
    let natives_count = Arc::new(AtomicUsize::new(0));
    let gta5_docs = parser.fetch_markdown_filenames(GamesType::GTA5).await?;
    let cfx_docs = parser.fetch_markdown_filenames(GamesType::CFX).await?;
    let docs_join = [gta5_docs, cfx_docs].concat();

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
            let docs_join = docs_join.clone();
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

    
    let (gta5_count, rdr3_count) = parser.organize_natives_by_apiset(&mut metadata).await?;
    println!(
        "Natives organized successfully! {} Total natives organized | {} Natives GTA5 | {} Natives RDR3",
        gta5_count + rdr3_count,
        gta5_count,
        rdr3_count
    );

    let metadata_file = std::fs::File::create("plugin/metadata.json")?;
    serde_json::to_writer_pretty(metadata_file, &metadata)?;

    Ok(())
}

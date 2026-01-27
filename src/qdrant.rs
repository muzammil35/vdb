use fastembed::Embedding;
use qdrant_client::Qdrant;
use qdrant_client::QdrantError;
use qdrant_client::qdrant::Distance;
use qdrant_client::qdrant::QueryResponse;
use qdrant_client::qdrant::UpsertPointsBuilder;
use qdrant_client::qdrant::{CreateCollectionBuilder, VectorParamsBuilder};
use qdrant_client::qdrant::{PointStruct, Value};
use qdrant_client::qdrant::QueryPointsBuilder;
use qdrant_client::qdrant::SearchResponse;
use qdrant_client::qdrant::SearchPointsBuilder;
use std::collections::HashMap;


use crate::embed;

pub async fn setup_qdrant(
    embedded_chunks: &embed::Embeddings,
    collection_name: &str,
) -> Result<Qdrant, QdrantError> {
    let client = Qdrant::from_url("http://localhost:6334").build()?;

    delete_all_collections(&client).await;

    client
        .create_collection(
            CreateCollectionBuilder::new(collection_name).vectors_config(VectorParamsBuilder::new(
                embedded_chunks.get_dim() as u64,
                Distance::Dot,
            )),
        )
        .await?;

    Ok(client)
}

pub async fn store_embeddings(
    client: &Qdrant,
    collection_name: &str,
    embeddings: embed::Embeddings,
) -> Result<(), QdrantError> {
    // Ensure both vectors have the same length
    assert_eq!(
        embeddings.original.len(),
        embeddings.embedded.len(),
        "Original and embedded vectors must have the same length"
    );

    let points: Vec<PointStruct> = embeddings
        .original
        .into_iter()
        .zip(embeddings.embedded)
        .enumerate()
        .map(|(id, (chunk, embedding))| {
            // Create payload with original chunk data from Embeddings.original
            let mut payload = HashMap::new();
            payload.insert("text".to_string(), Value::from(chunk.content.clone()));
            // have to insert the page as a float as the qdrant crate does not provide support for u16
            payload.insert("page".to_string(), Value::from(chunk.page as f32));
            // Add any other chunk fields you want to store

            PointStruct::new(
                id as u64, // Use index as ID
                embedding, payload,
            )
        })
        .collect();

    // Insert points into collection
    let response = client
        .upsert_points(UpsertPointsBuilder::new(collection_name, points).wait(true))
        .await?;
    dbg!(response);

    Ok(())
}

pub async fn run_query(client: &Qdrant, collection_name: &str, query: &str) -> Result<SearchResponse, anyhow::Error> {
    let emb_query = match embed::embed_query(query) {
        Ok(embedding) => embedding,
        Err(e) => {
            eprintln!("Failed to embed query: {}", e);
            return Err(e);
            
        }
    };
    let search_result = client
    .search_points(
        SearchPointsBuilder::new(collection_name, emb_query, 5)
            .with_payload(true)  // This enables payload return
            .build()
    )
    .await?;

    Ok(search_result)
}

pub async fn delete_all_collections(client: &Qdrant) -> Result<(), Box<dyn std::error::Error>> {
    // Get list of all collections
    let collections = client.list_collections().await?;
    
    // Delete each collection
    for collection in collections.collections {
        println!("Deleting collection: {}", collection.name);
        client.delete_collection(&collection.name).await?;
    }
    
    println!("All collections deleted!");
    Ok(())
}
use qdrant_client::Qdrant;
use qdrant_client::QdrantError;
use qdrant_client::qdrant::Distance;
use qdrant_client::qdrant::UpsertPointsBuilder;
use qdrant_client::qdrant::{CreateCollectionBuilder, VectorParamsBuilder};
use qdrant_client::qdrant::{PointStruct, Value};
use std::collections::HashMap;

use crate::embed;

pub async fn setup_qdrant(
    embedded_chunks: &embed::Embeddings,
    collection_name: &str,
) -> Result<Qdrant, QdrantError> {
    let client = Qdrant::from_url("http://localhost:6334").build()?;

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
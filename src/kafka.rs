use anyhow::{Context, Result};
use chrono::Utc;
use rdkafka::config::ClientConfig;
use rdkafka::producer::{FutureProducer, FutureRecord, Producer};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use uuid::Uuid;
use utoipa::ToSchema;


#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MediaStartUploadEvent {
    pub file_id: String,
    pub author_id: String,
    pub filename: String,
    pub started_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MediaUploadedEvent {
    pub file_id: String,
    pub author_id: String,
    pub size_bytes: usize,
    pub original_format: String,
    pub temp_path: String,
    pub uploaded_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MediaErrorEvent {
    pub file_id: String,
    pub stage: String,
    pub error_message: String,
    pub timestamp: String,
}


const TOPIC: &str = "media";


pub struct KafkaProducer {
    producer: FutureProducer,
}

impl KafkaProducer {
    pub fn new(brokers: &str) -> Result<Self> {
        let producer = ClientConfig::new()
            .set("bootstrap.servers", brokers)
            .set("message.timeout.ms", "5000")
            .create::<FutureProducer>()
            .context("Failed to create Kafka producer")?;

        Ok(Self { producer })
    }

    pub async fn send_start_upload(
        &self,
        file_id: Uuid,
        author_id: &str,
        filename: &str,
    ) -> Result<()> {
        let event = MediaStartUploadEvent {
            file_id: file_id.to_string(),
            author_id: author_id.to_string(),
            filename: filename.to_string(),
            started_at: Utc::now().to_rfc3339(),
        };

        let payload = serde_json::to_string(&event)?;
        let record = FutureRecord::to(TOPIC)
            .key(&event.file_id)
            .payload(&payload);

        self.producer
            .send(record, Duration::from_secs(30))
            .await
            .map_err(|(err, _msg)| anyhow::anyhow!("Failed to send media.start_upload: {}", err))?;

        tracing::info!(
            "Published media.start_upload (file_id={}, author_id={})",
            file_id,
            author_id,
        );

        Ok(())
    }

    pub async fn send_uploaded(
        &self,
        file_id: Uuid,
        author_id: &str,
        size_bytes: usize,
        original_format: &str,
        temp_path: &str,
    ) -> Result<()> {
        let event = MediaUploadedEvent {
            file_id: file_id.to_string(),
            author_id: author_id.to_string(),
            size_bytes,
            original_format: original_format.to_string(),
            temp_path: temp_path.to_string(),
            uploaded_at: Utc::now().to_rfc3339(),
        };

        let payload = serde_json::to_string(&event)?;
        let record = FutureRecord::to(TOPIC)
            .key(&event.file_id)
            .payload(&payload);

        self.producer
            .send(record, Duration::from_secs(30))
            .await
            .map_err(|(err, _msg)| anyhow::anyhow!("Failed to send media.uploaded: {}", err))?;

        tracing::info!(
            "Published media.uploaded (file_id={}, size={})",
            file_id,
            size_bytes,
        );

        Ok(())
    }

    pub async fn send_error(
        &self,
        file_id: Uuid,
        stage: &str,
        error_message: &str,
    ) -> Result<()> {
        let event = MediaErrorEvent {
            file_id: file_id.to_string(),
            stage: stage.to_string(),
            error_message: error_message.to_string(),
            timestamp: Utc::now().to_rfc3339(),
        };

        let payload = serde_json::to_string(&event)?;
        let record = FutureRecord::to(TOPIC)
            .key(&event.file_id)
            .payload(&payload);

        self.producer
            .send(record, Duration::from_secs(30))
            .await
            .map_err(|(err, _msg)| anyhow::anyhow!("Failed to send media.error: {}", err))?;

        tracing::info!(
            "Published media.error (file_id={}, stage={})",
            file_id,
            stage,
        );

        Ok(())
    }

    pub fn flush(&self) -> Result<()> {
        self.producer
            .flush(Duration::from_secs(10))
            .context("Failed to flush Kafka producer")?;
        Ok(())
    }
}

pub type SharedKafkaProducer = Arc<KafkaProducer>;

pub fn new_producer(brokers: &str) -> Result<SharedKafkaProducer> {
    let producer = KafkaProducer::new(brokers)?;
    Ok(Arc::new(producer))
}

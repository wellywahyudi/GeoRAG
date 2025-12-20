-- Spatial indexes (GIST) for efficient spatial queries
CREATE INDEX idx_datasets_bbox ON datasets USING GIST(bbox);
CREATE INDEX idx_features_geometry ON features USING GIST(geometry);
CREATE INDEX idx_chunks_geometry ON chunks USING GIST(geometry);

-- Foreign key indexes for efficient joins and cascading deletes
CREATE INDEX idx_datasets_workspace ON datasets(workspace_id);
CREATE INDEX idx_features_dataset ON features(dataset_id);
CREATE INDEX idx_documents_dataset ON documents(dataset_id);
CREATE INDEX idx_chunks_document ON chunks(document_id);
CREATE INDEX idx_chunks_spatial_ref ON chunks(spatial_ref);
CREATE INDEX idx_embeddings_chunk ON embeddings(chunk_id);
CREATE INDEX idx_index_builds_workspace ON index_builds(workspace_id);

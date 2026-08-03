ALTER TABLE blob_metadata ADD COLUMN uncompressed_size INTEGER;
ALTER TABLE blob_metadata ADD COLUMN compression TEXT DEFAULT 'none';

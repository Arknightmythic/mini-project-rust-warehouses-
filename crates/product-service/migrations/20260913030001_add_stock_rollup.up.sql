-- A READ MODEL, not a source of truth. The real quantity lives in
-- inventory-service; this column is a denormalised copy kept up to date by
-- consuming events, so that listing products does not require calling another
-- service. It is allowed to lag. It is not allowed to be trusted for decisions.
ALTER TABLE public.products
ADD COLUMN IF NOT EXISTS total_stock_cached BIGINT NOT NULL DEFAULT 0;

ALTER TABLE public.products
ADD COLUMN IF NOT EXISTS stock_synced_at TIMESTAMP WITH TIME ZONE;

-- A saga needs somewhere to remember that it is halfway done. Without a persisted
-- RESERVED row there is nothing for a compensator to find after a crash, and the
-- reserved quantity would stay locked forever with no record of why.
CREATE TABLE IF NOT EXISTS public.outbound_shipments (
    id BIGSERIAL PRIMARY KEY,
    warehouse_id BIGINT NOT NULL,
    reference_no VARCHAR(100),
    status VARCHAR(20) NOT NULL DEFAULT 'RESERVED',
    idempotency_key VARCHAR(100) NOT NULL,
    created_by BIGINT,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

ALTER TABLE public.outbound_shipments
ADD CONSTRAINT uni_outbound_shipments_idempotency_key UNIQUE (idempotency_key);

ALTER TABLE public.outbound_shipments
ADD CONSTRAINT chk_shipment_status
CHECK (status IN ('RESERVED', 'SHIPPED', 'CANCELLED'));

-- The sweeper scans on exactly this pair, so it gets an index.
CREATE INDEX IF NOT EXISTS idx_shipments_status_created
ON public.outbound_shipments (status, created_at);

CREATE TABLE IF NOT EXISTS public.outbound_shipment_items (
    id BIGSERIAL PRIMARY KEY,
    shipment_id BIGINT NOT NULL,
    product_id BIGINT NOT NULL,
    quantity BIGINT NOT NULL
);

ALTER TABLE public.outbound_shipment_items
ADD CONSTRAINT fk_shipment_items_shipment
FOREIGN KEY (shipment_id) REFERENCES public.outbound_shipments (id) ON DELETE CASCADE;

ALTER TABLE public.outbound_shipment_items
ADD CONSTRAINT chk_shipment_item_quantity CHECK (quantity > 0);

CREATE INDEX IF NOT EXISTS idx_shipment_items_shipment
ON public.outbound_shipment_items (shipment_id);

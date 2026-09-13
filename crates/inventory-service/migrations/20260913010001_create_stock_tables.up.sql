-- warehouse_id lives in warehouse-service's database and product_id in
-- product-service's. Neither can have a FOREIGN KEY: the referenced tables are in
-- other processes entirely. The write-time gRPC validation in ReceiveStock IS the
-- replacement for those constraints.
CREATE TABLE IF NOT EXISTS public.stock_balances (
    id BIGSERIAL PRIMARY KEY,
    warehouse_id BIGINT NOT NULL,
    product_id BIGINT NOT NULL,
    qty_on_hand BIGINT NOT NULL DEFAULT 0,
    qty_reserved BIGINT NOT NULL DEFAULT 0,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

-- What CAN still be enforced locally, and it matters:
ALTER TABLE public.stock_balances
ADD CONSTRAINT uni_stock_warehouse_product UNIQUE (warehouse_id, product_id);

ALTER TABLE public.stock_balances
ADD CONSTRAINT chk_stock_non_negative
CHECK (qty_on_hand >= 0 AND qty_reserved >= 0 AND qty_reserved <= qty_on_hand);

CREATE INDEX IF NOT EXISTS idx_stock_balances_warehouse ON public.stock_balances (warehouse_id);
CREATE INDEX IF NOT EXISTS idx_stock_balances_product ON public.stock_balances (product_id);

-- Append-only ledger: balances can be rebuilt from this, never the other way.
CREATE TABLE IF NOT EXISTS public.stock_movements (
    id BIGSERIAL PRIMARY KEY,
    warehouse_id BIGINT NOT NULL,
    product_id BIGINT NOT NULL,
    movement_type VARCHAR(20) NOT NULL,
    quantity BIGINT NOT NULL,
    reference_type VARCHAR(30),
    reference_id BIGINT,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

ALTER TABLE public.stock_movements
ADD CONSTRAINT chk_movement_type CHECK (movement_type IN ('IN', 'OUT'));

ALTER TABLE public.stock_movements
ADD CONSTRAINT chk_movement_quantity CHECK (quantity > 0);

CREATE INDEX IF NOT EXISTS idx_stock_movements_warehouse ON public.stock_movements (warehouse_id);
CREATE INDEX IF NOT EXISTS idx_stock_movements_product ON public.stock_movements (product_id);

CREATE TABLE IF NOT EXISTS public.inbound_receipts (
    id BIGSERIAL PRIMARY KEY,
    warehouse_id BIGINT NOT NULL,
    reference_no VARCHAR(100),
    idempotency_key VARCHAR(100) NOT NULL,
    created_by BIGINT,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

-- This single constraint is what makes a retried receipt safe.
ALTER TABLE public.inbound_receipts
ADD CONSTRAINT uni_inbound_receipts_idempotency_key UNIQUE (idempotency_key);

CREATE TABLE IF NOT EXISTS public.inbound_receipt_items (
    id BIGSERIAL PRIMARY KEY,
    receipt_id BIGINT NOT NULL,
    product_id BIGINT NOT NULL,
    quantity BIGINT NOT NULL,
    unit_cost BIGINT NOT NULL DEFAULT 0
);

-- Contrast with product_id two lines up: receipt_id CAN have a foreign key,
-- because inbound_receipts is in THIS database, owned by THIS service.
ALTER TABLE public.inbound_receipt_items
ADD CONSTRAINT fk_receipt_items_receipt
FOREIGN KEY (receipt_id) REFERENCES public.inbound_receipts (id) ON DELETE CASCADE;

ALTER TABLE public.inbound_receipt_items
ADD CONSTRAINT chk_receipt_item_quantity CHECK (quantity > 0);

CREATE INDEX IF NOT EXISTS idx_receipt_items_receipt ON public.inbound_receipt_items (receipt_id);

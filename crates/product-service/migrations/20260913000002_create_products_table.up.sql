CREATE TABLE IF NOT EXISTS public.products (
    id BIGSERIAL PRIMARY KEY,
    sku VARCHAR(100) NOT NULL,
    name VARCHAR(255) NOT NULL,
    description TEXT,
    unit VARCHAR(50) NOT NULL DEFAULT 'pcs',
    barcode TEXT,
    category_id BIGINT,
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

ALTER TABLE public.products
ADD CONSTRAINT uni_products_sku UNIQUE (sku);

-- A real foreign key is fine here: categories live in THIS database, owned by
-- THIS service. Compare with inventory-service in the next phase, where
-- product_id cannot have one because the referenced table is in another process.
ALTER TABLE public.products
ADD CONSTRAINT fk_products_category
FOREIGN KEY (category_id) REFERENCES public.categories (id) ON DELETE SET NULL;

CREATE INDEX IF NOT EXISTS idx_products_is_active ON public.products (is_active);
CREATE INDEX IF NOT EXISTS idx_products_category_id ON public.products (category_id);

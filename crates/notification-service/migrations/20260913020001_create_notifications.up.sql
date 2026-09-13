CREATE TABLE IF NOT EXISTS public.notifications (
    id BIGSERIAL PRIMARY KEY,
    recipient_email VARCHAR(255) NOT NULL,
    subject VARCHAR(255) NOT NULL,
    body TEXT NOT NULL,
    event_id UUID NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_notifications_event_id ON public.notifications (event_id);

-- RabbitMQ guarantees at-least-once, not exactly-once: a broker that loses an ack
-- redelivers. This table is what turns at-least-once into effectively-once. It is
-- the async counterpart of inbound_receipts.idempotency_key on the sync side.
CREATE TABLE IF NOT EXISTS public.processed_events (
    event_id UUID PRIMARY KEY,
    event_type VARCHAR(100) NOT NULL,
    processed_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

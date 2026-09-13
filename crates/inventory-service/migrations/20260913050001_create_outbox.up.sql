-- THE FIX FOR THE DUAL-WRITE PROBLEM.
--
-- Until now the event was published AFTER the transaction committed. Those are two
-- separate systems and there is no way to make two writes atomic across them: a
-- crash in between leaves stock that exists and an event nobody ever sent.
--
-- Writing the event into THIS database, inside the SAME transaction as the stock
-- movement, makes it one write again. A separate relay then moves rows from here
-- to the broker. The event can no longer be lost, only delayed.
CREATE TABLE IF NOT EXISTS public.outbox (
    id BIGSERIAL PRIMARY KEY,
    event_id UUID NOT NULL,
    event_type VARCHAR(100) NOT NULL,
    routing_key VARCHAR(100) NOT NULL,
    payload JSONB NOT NULL,
    trace_parent VARCHAR(100),
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    sent_at TIMESTAMP WITH TIME ZONE
);

ALTER TABLE public.outbox
ADD CONSTRAINT uni_outbox_event_id UNIQUE (event_id);

-- Partial index: the relay only ever scans unsent rows, and once a row is sent it
-- stops costing anything to keep.
CREATE INDEX IF NOT EXISTS idx_outbox_unsent
ON public.outbox (id) WHERE sent_at IS NULL;

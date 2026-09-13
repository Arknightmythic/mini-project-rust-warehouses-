-- Add up migration script here
CREATE TABLE IF NOT EXISTS public.roles (
    id BIGSERIAL PRIMARY KEY,
    name VARCHAR(255) NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

ALTER TABLE public.roles
ADD CONSTRAINT uni_roles_name UNIQUE (name);
CREATE INDEX IF NOT EXISTS idx_roles_name ON public.roles (name);
create index if not exists idx_roles_created_at on public.roles (created_at);
-- Add up migration script here
create table if not exists public.warehouses (
    id bigserial primary key,
    name varchar(255) not null,
    address text not null,
    phone text null,
    photo text null,
    created_at timestamp with time zone default current_timestamp,
    updated_at timestamp with time zone default current_timestamp,
    delete_at timestamp with time zone null
);


alter table public.warehouses add constraint unique_warehouses_name unique (name);
create index if not exists idx_warehouses_delete_at on public.warehouses (delete_at);

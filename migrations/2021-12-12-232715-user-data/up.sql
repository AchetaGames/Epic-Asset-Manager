create table user_data
(
    name TEXT,
    value TEXT,
    constraint user_data
        unique (name)
);

create index user_data_index
    on user_data (name);
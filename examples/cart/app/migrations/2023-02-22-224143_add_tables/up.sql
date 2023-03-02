create table if not exists account (
  id          uuid       primary key,
  created_at  timestamp  not null,
  updated_at  timestamp  not null,
  deleted_at  timestamp,
  username    varchar(64),
  email       varchar(256)
);

create unique index account_unique_username on account (username, coalesce(deleted_at, timestamp '0001-01-01 00:00:00'));
create unique index account_unique_email on account (email, coalesce(deleted_at, timestamp '0001-01-01 00:00:00'));

create table if not exists account_audit (
  id          uuid          primary key,
  account_id  uuid          not null,
  created_at  timestamp     not null,
  updated_at  timestamp     not null,
  deleted_at  timestamp,
  username    varchar(64),
  email       varchar(256),

  foreign key (account_id) references account (id)
);

create table if not exists item (
  id           uuid          primary key,
  created_at   timestamp     not null,
  updated_at   timestamp     not null,
  name         varchar(64)   not null,
  description  varchar(256)
);

create table if not exists item_audit (
  id                                  uuid          primary key,
  item_id_arbitrary_foreign_key_name  uuid          not null,
  created_at                          timestamp     not null,
  updated_at                          timestamp     not null,
  name                                varchar(64)   not null,
  description                         varchar(256),

  foreign key (item_id_arbitrary_foreign_key_name) references item (id)
);

create table if not exists cart (
  id           uuid       primary key,
  created_at   timestamp  not null,
  updated_at   timestamp  not null,
  account_id   uuid       not null,
  used_at      timestamp,

  -- there should only be one unused cart per account
  unique nulls not distinct (account_id, used_at),

  foreign key (account_id) references account (id)
);



create table if not exists cart_item (
  id           uuid          primary key,
  created_at   timestamp     not null,
  cart_id      uuid          not null,
  item_id      uuid          not null,

  foreign key (cart_id) references cart (id),
  foreign key (item_id) references item (id)
);

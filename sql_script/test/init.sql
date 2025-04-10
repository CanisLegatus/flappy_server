drop table if exists test_score;

create table test_score (

    id serial primary key,
    player_name text not null,
    player_score INT not null,
    posted_time TIMESTAMP default now()

);


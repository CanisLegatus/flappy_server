drop table if exists flappy_dragon_score;

create table flappy_dragon_score (

    id serial primary key,
    player_name text not null,
    player_score INT not null,
    posted_time TIMESTAMP default now()

);


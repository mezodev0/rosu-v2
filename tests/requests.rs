//! Be sure the following env variables are set beforehand:
//!     - RUST_TEST_THREADS=1
//!     - RUST_LOG="rosu_v2=debug","error"

extern crate rosu_v2;

use dotenv::dotenv;
use once_cell::sync::OnceCell;
use rosu_v2::{model::GameMode, Osu};
use std::env;

#[allow(unused_imports)]
use rosu_v2::model::GameMods;

#[macro_use]
extern crate log;

macro_rules! unwind_error {
    ($log:ident, $err:ident, $($arg:tt)+) => {
        {
            $log!($($arg)+, $err);
            let mut err: &dyn ::std::error::Error = &$err;
            while let Some(source) = err.source() {
                $log!("  - caused by: {}", source);
                err = source;
            }
        }
    };
}

static OSU: OnceCell<Osu> = OnceCell::new();

async fn init() {
    if OSU.get().is_none() {
        let _ = env_logger::builder().is_test(true).try_init();
        dotenv().ok();

        let client_id = env::var("CLIENT_ID")
            .expect("missing CLIENT_ID")
            .parse()
            .expect("failed to parse client id as u64");

        let client_secret = env::var("CLIENT_SECRET").expect("missing CLIENT_SECRET");

        let osu = Osu::builder()
            .client_id(client_id)
            .client_secret(client_secret)
            .build()
            .await
            .unwrap_or_else(|err| panic!("failed to build osu! client: {}", err));

        OSU.set(osu).unwrap_or_else(|_| panic!("failed to set OSU"));
    }
}

macro_rules! get_id {
    ($name:ident, $id:literal) => {
        fn $name() -> u32 {
            $id
        }
    };
}

get_id!(adesso_balla, 171024);
get_id!(badewanne3, 2211396);
get_id!(sylas, 3906405);

fn osu() -> &'static Osu {
    OSU.get().expect("OSU not initialized")
}

#[tokio::test]
#[ignore = "specific testing"]
async fn custom() {
    init().await;

    let req_fut = osu().comments().commentable_id(1135494);

    if let Err(why) = req_fut.await {
        unwind_error!(error, why, "Error while requesting custom: {}");
        panic!()
    }
}

#[tokio::test]
async fn beatmap() {
    init().await;

    match osu().beatmap(adesso_balla()).await {
        Ok(map) => println!(
            "Received {} - {}",
            map.mapset.as_ref().unwrap().artist(),
            map.mapset.as_ref().unwrap().title(),
        ),
        Err(why) => {
            unwind_error!(error, why, "Error while requesting beatmap: {}");
            panic!()
        }
    }
}

#[tokio::test]
async fn beatmap_scores() {
    init().await;

    match osu().beatmap_scores(adesso_balla()).await {
        Ok(scores) => println!(
            "Received {} scores | user score: {}",
            scores.scores.len(),
            scores.user_score.is_some(),
        ),
        Err(why) => {
            unwind_error!(error, why, "Error while requesting beatmap scores: {}");
            panic!()
        }
    }
}

#[tokio::test]
async fn beatmap_user_score() {
    init().await;

    match osu().beatmap_user_score(adesso_balla(), badewanne3()).await {
        Ok(score) => println!(
            "Received score, pos={} | mods={}",
            score.pos, score.score.mods,
        ),
        Err(why) => {
            unwind_error!(
                println,
                why,
                "Error while requesting beatmap user scores: {}"
            );
            panic!()
        }
    }
}

#[tokio::test]
async fn comments() {
    init().await;

    match osu().comments().sort_new().await {
        Ok(bundle) => println!(
            "Received bundle, {} comments | {} users",
            bundle.comments.len(),
            bundle.users.len(),
        ),
        Err(why) => {
            unwind_error!(error, why, "Error while requesting comments: {}");
            panic!()
        }
    }
}

#[tokio::test]
async fn recent_events() {
    init().await;

    match osu().recent_events(badewanne3()).limit(10).offset(2).await {
        Ok(events) => println!("Received {} events", events.len()),
        Err(why) => {
            unwind_error!(error, why, "Error while requesting recent events: {}");
            panic!()
        }
    }
}

#[tokio::test]
async fn kudosu() {
    init().await;

    match osu().kudosu(sylas()).limit(5).offset(1).await {
        Ok(history) => {
            let sum: i32 = history.iter().map(|entry| entry.amount).sum();

            println!("Received {} entries amounting to {}", history.len(), sum);
        }
        Err(why) => {
            unwind_error!(error, why, "Error while requesting kudosu: {}");
            panic!()
        }
    }
}

#[tokio::test]
async fn rankings() {
    init().await;

    match osu()
        .rankings(GameMode::STD)
        .country("be")
        .type_performance()
        .await
    {
        Ok(rankings) => {
            let mapsets = rankings.mapsets.map_or(0, |mapsets| mapsets.len());
            let total = rankings.total;
            let rankings = rankings.ranking.len();

            println!(
                "Received value with {} mapsets, {} rankings, and a total of {}",
                mapsets, rankings, total
            );
        }
        Err(why) => {
            unwind_error!(error, why, "Error while requesting rankings: {}");
            panic!()
        }
    }
}

#[tokio::test]
#[ignore = "TODO"]
async fn score() {
    init().await;

    // match osu().score(room, playlist, score_id).await {
    //     Ok(score) => todo!(),
    //     Err(why) => {
    //         unwind_error!(error, why, "Error while requesting score: {}");
    //         panic!()
    //     }
    // }
}

#[tokio::test]
#[ignore = "TODO"]
async fn scores() {
    init().await;

    // match osu().scores(room, playlist).await {
    //     Ok(scores) => todo!(),
    //     Err(why) => {
    //         unwind_error!(error, why, "Error while requesting scores: {}");
    //         panic!()
    //     }
    // }
}

#[tokio::test]
async fn spotlights() {
    init().await;

    match osu().spotlights().await {
        Ok(spotlights) => {
            let participants: u32 = spotlights
                .iter()
                .map(|s| s.participant_count.unwrap_or(0))
                .sum();

            println!(
                "Received {} spotlights with a total of {} participants",
                spotlights.len(),
                participants
            );
        }
        Err(why) => {
            unwind_error!(error, why, "Error while requesting spotlights: {}");
            panic!()
        }
    }
}

#[tokio::test]
async fn user() {
    init().await;

    match osu().user("freddie benson").mode(GameMode::TKO).await {
        Ok(user) => println!("Received user who was last active {:?}", user.last_visit),
        Err(why) => {
            unwind_error!(error, why, "Error while requesting user: {}");
            panic!()
        }
    }
}

#[tokio::test]
async fn user_beatmapsets() {
    init().await;

    match osu()
        .user_beatmapsets(sylas())
        .limit(5)
        .ranked_and_approved()
        .offset(2)
        .await
    {
        Ok(mapsets) => println!("Received {} mapsets of the user", mapsets.len()),
        Err(why) => {
            unwind_error!(error, why, "Error while requesting user beatmapsets: {}");
            panic!()
        }
    }
}

#[tokio::test]
#[ignore = "TODO"]
async fn user_highscore() {
    init().await;

    // match osu().user_highscore(room, playlist, user_id).await {
    //     Ok(score) => todo!(),
    //     Err(why) => {
    //         unwind_error!(error, why, "Error while requesting user highscore: {}");
    //         panic!()
    //     }
    // }
}

#[tokio::test]
async fn user_most_played() {
    init().await;

    match osu()
        .user_most_played(badewanne3())
        .limit(5)
        .offset(2)
        .await
    {
        Ok(scores) => println!(
            "Received {} scores, the first is map id {}",
            scores.len(),
            scores[0].map_id
        ),
        Err(why) => {
            unwind_error!(error, why, "Error while requesting user most played: {}");
            panic!()
        }
    }
}

#[tokio::test]
async fn user_scores() {
    init().await;

    match osu()
        .user_scores(badewanne3())
        .mode(GameMode::CTB)
        .limit(10)
        .offset(1)
        .best()
        .await
    {
        Ok(scores) => {
            let pp = scores[1].pp.expect("got fewer than two scores");

            println!("Received {} scores, the second has {}pp", scores.len(), pp);
        }
        Err(why) => {
            unwind_error!(error, why, "Error while requesting user scores: {}");
            panic!()
        }
    }
}

#[tokio::test]
#[ignore = "currently unavailable"]
async fn users() {
    init().await;

    #[allow(deprecated)]
    match osu().users([badewanne3(), sylas()].iter().copied()).await {
        Ok(users) => println!("Received {} users", users.len()),
        Err(why) => {
            unwind_error!(error, why, "Error while requesting users: {}");
            panic!()
        }
    }
}

#[tokio::test]
async fn wiki() {
    init().await;

    match osu().wiki("de").page("Hit_object").await {
        Ok(page) => println!(
            "Received page {}/{}: {}",
            page.locale, page.path, page.title
        ),
        Err(why) => {
            unwind_error!(error, why, "Error while requesting wiki: {}");
            panic!()
        }
    }
}
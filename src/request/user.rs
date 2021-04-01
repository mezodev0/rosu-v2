use crate::{
    model::{
        beatmap::{Beatmapset, MostPlayedMap, RankStatus},
        kudosu::KudosuHistory,
        recent_event::RecentEvent,
        score::Score,
        user::{User, UserCompact},
        GameMode,
    },
    request::{Pending, Query, Request},
    routing::Route,
    Osu,
};

use std::fmt;

#[cfg(feature = "cache")]
use futures::future::TryFutureExt;

/// Either a user id as u32 or a username as String.
///
/// Since usernames will be stored as `String`, if possible,
/// make use of `From<String>` instead of `From<&String>`.
#[derive(Debug)]
pub enum UserId {
    Id(u32),
    Name(String),
}

impl From<u32> for UserId {
    #[inline]
    fn from(id: u32) -> Self {
        Self::Id(id)
    }
}

impl From<&str> for UserId {
    #[inline]
    fn from(name: &str) -> Self {
        Self::Name(name.to_owned())
    }
}

impl From<&String> for UserId {
    #[inline]
    fn from(name: &String) -> Self {
        Self::Name(name.to_owned())
    }
}

impl From<String> for UserId {
    #[inline]
    fn from(name: String) -> Self {
        Self::Name(name)
    }
}

impl fmt::Display for UserId {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Id(id) => write!(f, "{}", id),
            Self::Name(name) => f.write_str(name),
        }
    }
}

/// Get a [`User`](crate::model::user::User) by their id.
#[must_use = "futures do nothing unless you `.await` or poll them"]
pub struct GetUser<'a> {
    fut: Option<Pending<'a, User>>,
    osu: &'a Osu,
    user_id: Option<UserId>,
    mode: Option<GameMode>,
}

impl<'a> GetUser<'a> {
    #[inline]
    pub(crate) fn new(osu: &'a Osu, user_id: impl Into<UserId>) -> Self {
        Self {
            fut: None,
            osu,
            user_id: Some(user_id.into()),
            mode: None,
        }
    }

    #[inline]
    pub fn mode(mut self, mode: GameMode) -> Self {
        self.mode.replace(mode);

        self
    }

    fn start(&mut self) -> Pending<'a, User> {
        #[cfg(feature = "metrics")]
        self.osu.metrics.user.inc();

        let req = Request::from(Route::GetUser {
            user_id: self.user_id.take().unwrap(),
            mode: self.mode,
        });

        Box::pin(self.osu.inner.request(req))
    }
}

poll_req!(GetUser<'_> => User);

/// Get the [`Beatmapset`](crate::model::beatmap::Beatmapset)s of a user by their id.
///
/// If no map type specified, either manually through
/// [`map_type`](crate::request::GetUserBeatmapsets::map_type),
/// or through any of the methods [`loved`](crate::request::GetUserBeatmapsets::loved),
/// [`favourite`](crate::request::GetUserBeatmapsets::favourite),
/// [`graveyard`](crate::request::GetUserBeatmapsets::graveyard),
/// [`ranked_and_approved`](crate::request::GetUserBeatmapsets::ranked_and_approved),
/// [`unranked`](crate::request::GetUserBeatmapsets::unranked),
/// it defaults to `ranked_and_approved`.
#[must_use = "futures do nothing unless you `.await` or poll them"]
pub struct GetUserBeatmapsets<'a> {
    fut: Option<Pending<'a, Vec<Beatmapset>>>,
    osu: &'a Osu,
    map_type: &'static str,
    limit: Option<usize>,
    offset: Option<usize>,

    #[cfg(not(feature = "cache"))]
    user_id: u32,

    #[cfg(feature = "cache")]
    user_id: Option<UserId>,
}

impl<'a> GetUserBeatmapsets<'a> {
    #[cfg(not(feature = "cache"))]
    #[inline]
    pub(crate) fn new(osu: &'a Osu, user_id: u32) -> Self {
        Self {
            fut: None,
            osu,
            user_id,
            map_type: "ranked_and_approved",
            limit: None,
            offset: None,
        }
    }

    #[cfg(feature = "cache")]
    #[inline]
    pub(crate) fn new(osu: &'a Osu, user_id: UserId) -> Self {
        Self {
            fut: None,
            osu,
            user_id: Some(user_id),
            map_type: "ranked_and_approved",
            limit: None,
            offset: None,
        }
    }

    #[inline]
    pub fn limit(mut self, limit: usize) -> Self {
        self.limit.replace(limit);

        self
    }

    #[inline]
    pub fn offset(mut self, offset: usize) -> Self {
        self.offset.replace(offset);

        self
    }

    pub fn map_type(mut self, map_type: RankStatus) -> Self {
        self.map_type = match map_type {
            RankStatus::Approved | RankStatus::Ranked => "ranked_and_approved",
            RankStatus::Graveyard => "graveyard",
            RankStatus::Pending | RankStatus::WIP | RankStatus::Qualified => "unranked",
            RankStatus::Loved => "loved",
        };

        self
    }

    #[inline]
    pub fn loved(mut self) -> Self {
        self.map_type = "loved";

        self
    }

    #[inline]
    pub fn favourite(mut self) -> Self {
        self.map_type = "favourite";

        self
    }

    #[inline]
    pub fn graveyard(mut self) -> Self {
        self.map_type = "graveyard";

        self
    }

    #[inline]
    pub fn ranked_and_approved(mut self) -> Self {
        self.map_type = "ranked_and_approved";

        self
    }

    #[inline]
    pub fn unranked(mut self) -> Self {
        self.map_type = "unranked";

        self
    }

    fn start(&mut self) -> Pending<'a, Vec<Beatmapset>> {
        #[cfg(feature = "metrics")]
        self.osu.metrics.user_beatmapsets.inc();

        let map_type = self.map_type;
        let mut query = Query::new();

        if let Some(limit) = self.limit {
            query.push("limit", limit.to_string());
        }

        if let Some(offset) = self.offset {
            query.push("offset", offset.to_string());
        }

        #[cfg(not(feature = "cache"))]
        {
            let user_id = self.user_id;
            let req = Request::from((query, Route::GetUserBeatmapsets { user_id, map_type }));

            Box::pin(self.osu.inner.request(req))
        }

        #[cfg(feature = "cache")]
        {
            let osu = &self.osu.inner;

            let fut = self
                .osu
                .cache_user(self.user_id.take().unwrap())
                .map_ok(move |user_id| {
                    Request::from((query, Route::GetUserBeatmapsets { user_id, map_type }))
                })
                .and_then(move |req| osu.request(req));

            Box::pin(fut)
        }
    }
}

poll_req!(GetUserBeatmapsets<'_> => Vec<Beatmapset>);

/// Get a user's kudosu history by their user id in form of a vec
/// of [`KudosuHistory`](crate::model::kudosu::KudosuHistory).
#[must_use = "futures do nothing unless you `.await` or poll them"]
pub struct GetUserKudosu<'a> {
    fut: Option<Pending<'a, Vec<KudosuHistory>>>,
    osu: &'a Osu,
    limit: Option<usize>,
    offset: Option<usize>,

    #[cfg(not(feature = "cache"))]
    user_id: u32,

    #[cfg(feature = "cache")]
    user_id: Option<UserId>,
}

impl<'a> GetUserKudosu<'a> {
    #[cfg(not(feature = "cache"))]
    #[inline]
    pub(crate) fn new(osu: &'a Osu, user_id: u32) -> Self {
        Self {
            fut: None,
            osu,
            user_id,
            limit: None,
            offset: None,
        }
    }

    #[cfg(feature = "cache")]
    #[inline]
    pub(crate) fn new(osu: &'a Osu, user_id: UserId) -> Self {
        Self {
            fut: None,
            osu,
            user_id: Some(user_id),
            limit: None,
            offset: None,
        }
    }

    #[inline]
    pub fn limit(mut self, limit: usize) -> Self {
        self.limit.replace(limit);

        self
    }

    #[inline]
    pub fn offset(mut self, offset: usize) -> Self {
        self.offset.replace(offset);

        self
    }

    fn start(&mut self) -> Pending<'a, Vec<KudosuHistory>> {
        #[cfg(feature = "metrics")]
        self.osu.metrics.user_kudosu.inc();

        let mut query = Query::new();

        if let Some(limit) = self.limit {
            query.push("limit", limit.to_string());
        }

        if let Some(offset) = self.offset {
            query.push("offset", offset.to_string());
        }

        #[cfg(not(feature = "cache"))]
        {
            let user_id = self.user_id;
            let req = Request::from((query, Route::GetUserKudosu { user_id }));

            Box::pin(self.osu.inner.request(req))
        }

        #[cfg(feature = "cache")]
        {
            let osu = &self.osu.inner;

            let fut = self
                .osu
                .cache_user(self.user_id.take().unwrap())
                .map_ok(move |user_id| Request::from((query, Route::GetUserKudosu { user_id })))
                .and_then(move |req| osu.request(req));

            Box::pin(fut)
        }
    }
}

poll_req!(GetUserKudosu<'_> => Vec<KudosuHistory>);

/// Get the most played beatmaps of a user by their id in form
/// of a vec of [`MostPlayedMap`](crate::model::beatmap::MostPlayedMap).
#[must_use = "futures do nothing unless you `.await` or poll them"]
pub struct GetUserMostPlayed<'a> {
    fut: Option<Pending<'a, Vec<MostPlayedMap>>>,
    osu: &'a Osu,
    limit: Option<usize>,
    offset: Option<usize>,

    #[cfg(not(feature = "cache"))]
    user_id: u32,

    #[cfg(feature = "cache")]
    user_id: Option<UserId>,
}

impl<'a> GetUserMostPlayed<'a> {
    #[cfg(not(feature = "cache"))]
    #[inline]
    pub(crate) fn new(osu: &'a Osu, user_id: u32) -> Self {
        Self {
            fut: None,
            osu,
            user_id,
            limit: None,
            offset: None,
        }
    }

    #[cfg(feature = "cache")]
    #[inline]
    pub(crate) fn new(osu: &'a Osu, user_id: UserId) -> Self {
        Self {
            fut: None,
            osu,
            user_id: Some(user_id),
            limit: None,
            offset: None,
        }
    }

    /// The API provides at most 51 results per requests.
    #[inline]
    pub fn limit(mut self, limit: usize) -> Self {
        self.limit.replace(limit);

        self
    }

    #[inline]
    pub fn offset(mut self, offset: usize) -> Self {
        self.offset.replace(offset);

        self
    }

    fn start(&mut self) -> Pending<'a, Vec<MostPlayedMap>> {
        #[cfg(feature = "metrics")]
        self.osu.metrics.most_played.inc();

        let mut query = Query::new();

        if let Some(limit) = self.limit {
            query.push("limit", limit.to_string());
        }

        if let Some(offset) = self.offset {
            query.push("offset", offset.to_string());
        }

        #[cfg(not(feature = "cache"))]
        {
            let req = Request::from((
                query,
                Route::GetUserBeatmapsets {
                    user_id: self.user_id,
                    map_type: "most_played",
                },
            ));

            Box::pin(self.osu.inner.request(req))
        }

        #[cfg(feature = "cache")]
        {
            let osu = &self.osu.inner;

            let fut = self
                .osu
                .cache_user(self.user_id.take().unwrap())
                .map_ok(move |user_id| {
                    Request::from((
                        query,
                        Route::GetUserBeatmapsets {
                            user_id,
                            map_type: "most_played",
                        },
                    ))
                })
                .and_then(move |req| osu.request(req));

            Box::pin(fut)
        }
    }
}

poll_req!(GetUserMostPlayed<'_> => Vec<MostPlayedMap>);

/// Get a vec of [`RecentEvent`](crate::model::recent_event::RecentEvent) of a user by their id.
#[must_use = "futures do nothing unless you `.await` or poll them"]
pub struct GetRecentEvents<'a> {
    fut: Option<Pending<'a, Vec<RecentEvent>>>,
    osu: &'a Osu,
    limit: Option<usize>,
    offset: Option<usize>,

    #[cfg(not(feature = "cache"))]
    user_id: u32,

    #[cfg(feature = "cache")]
    user_id: Option<UserId>,
}

impl<'a> GetRecentEvents<'a> {
    #[cfg(not(feature = "cache"))]
    #[inline]
    pub(crate) fn new(osu: &'a Osu, user_id: u32) -> Self {
        Self {
            fut: None,
            osu,
            user_id,
            limit: None,
            offset: None,
        }
    }

    #[cfg(feature = "cache")]
    #[inline]
    pub(crate) fn new(osu: &'a Osu, user_id: UserId) -> Self {
        Self {
            fut: None,
            osu,
            user_id: Some(user_id),
            limit: None,
            offset: None,
        }
    }

    #[inline]
    pub fn limit(mut self, limit: usize) -> Self {
        self.limit.replace(limit);

        self
    }

    #[inline]
    pub fn offset(mut self, offset: usize) -> Self {
        self.offset.replace(offset);

        self
    }

    fn start(&mut self) -> Pending<'a, Vec<RecentEvent>> {
        #[cfg(feature = "metrics")]
        self.osu.metrics.recent_events.inc();

        let mut query = Query::new();

        if let Some(limit) = self.limit {
            query.push("limit", limit.to_string());
        }

        if let Some(offset) = self.offset {
            query.push("offset", offset.to_string());
        }

        #[cfg(not(feature = "cache"))]
        {
            let user_id = self.user_id;
            let req = Request::from((query, Route::GetRecentEvents { user_id }));

            Box::pin(self.osu.inner.request(req))
        }

        #[cfg(feature = "cache")]
        {
            let osu = &self.osu.inner;

            let fut = self
                .osu
                .cache_user(self.user_id.take().unwrap())
                .map_ok(move |user_id| Request::from((query, Route::GetRecentEvents { user_id })))
                .and_then(move |req| osu.request(req));

            Box::pin(fut)
        }
    }
}

poll_req!(GetRecentEvents<'_> => Vec<RecentEvent>);

#[derive(Copy, Clone, Debug)]
pub(crate) enum ScoreType {
    Best,
    First,
    Recent,
}

impl fmt::Display for ScoreType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let kind = match self {
            Self::Best => "best",
            Self::First => "firsts",
            Self::Recent => "recent",
        };

        f.write_str(kind)
    }
}

/// Get a vec of [`Score`](crate::model::score::Score) of a user by the user's id.
///
/// If no score type is specified by either
/// [`best`](crate::request::GetUserScores::best),
/// [`firsts`](crate::request::GetUserScores::firsts),
/// or [`recent`](crate::request::GetUserScores::recent), it defaults to `best`.
#[must_use = "futures do nothing unless you `.await` or poll them"]
pub struct GetUserScores<'a> {
    fut: Option<Pending<'a, Vec<Score>>>,
    osu: &'a Osu,
    score_type: ScoreType,
    limit: Option<usize>,
    offset: Option<usize>,
    include_fails: Option<bool>,
    mode: Option<GameMode>,

    #[cfg(not(feature = "cache"))]
    user_id: u32,

    #[cfg(feature = "cache")]
    user_id: Option<UserId>,
}

impl<'a> GetUserScores<'a> {
    #[cfg(not(feature = "cache"))]
    #[inline]
    pub(crate) fn new(osu: &'a Osu, user_id: u32) -> Self {
        Self {
            fut: None,
            osu,
            user_id,
            score_type: ScoreType::Best,
            limit: None,
            offset: None,
            include_fails: None,
            mode: None,
        }
    }

    #[cfg(feature = "cache")]
    #[inline]
    pub(crate) fn new(osu: &'a Osu, user_id: UserId) -> Self {
        Self {
            fut: None,
            osu,
            user_id: Some(user_id),
            score_type: ScoreType::Best,
            limit: None,
            offset: None,
            include_fails: None,
            mode: None,
        }
    }

    /// The API provides at most 51 results per requests.
    #[inline]
    pub fn limit(mut self, limit: usize) -> Self {
        self.limit.replace(limit);

        self
    }

    #[inline]
    pub fn offset(mut self, offset: usize) -> Self {
        self.offset.replace(offset);

        self
    }

    #[inline]
    pub fn mode(mut self, mode: GameMode) -> Self {
        self.mode.replace(mode);

        self
    }

    #[inline]
    pub fn include_fails(mut self, include_fails: bool) -> Self {
        self.include_fails.replace(include_fails);

        self
    }

    /// Get top scores of a user
    #[inline]
    pub fn best(mut self) -> Self {
        self.score_type = ScoreType::Best;

        self
    }

    /// Get global #1 scores of a user.
    #[inline]
    pub fn firsts(mut self) -> Self {
        self.score_type = ScoreType::First;

        self
    }

    /// Get recent scores of a user.
    #[inline]
    pub fn recent(mut self) -> Self {
        self.score_type = ScoreType::Recent;

        self
    }

    fn start(&mut self) -> Pending<'a, Vec<Score>> {
        #[cfg(feature = "metrics")]
        match self.score_type {
            ScoreType::Best => self.osu.metrics.user_top_scores.inc(),
            ScoreType::First => self.osu.metrics.user_first_scores.inc(),
            ScoreType::Recent => self.osu.metrics.user_recent_scores.inc(),
        }

        let mut query = Query::new();

        if let Some(limit) = self.limit {
            query.push("limit", limit.to_string());
        }

        if let Some(offset) = self.offset {
            query.push("offset", offset.to_string());
        }

        if let Some(mode) = self.mode {
            query.push("mode", mode.to_string());
        }

        if let Some(include_fails) = self.include_fails {
            query.push("include_fails", (include_fails as u8).to_string());
        }

        #[cfg(not(feature = "cache"))]
        {
            let req = Request::from((
                query,
                Route::GetUserScores {
                    user_id: self.user_id,
                    score_type: self.score_type,
                },
            ));

            Box::pin(self.osu.inner.request(req))
        }

        #[cfg(feature = "cache")]
        {
            let score_type = self.score_type;
            let osu = &self.osu.inner;

            let fut = self
                .osu
                .cache_user(self.user_id.take().unwrap())
                .map_ok(move |user_id| {
                    Request::from((
                        query,
                        Route::GetUserScores {
                            user_id,
                            score_type,
                        },
                    ))
                })
                .and_then(move |req| osu.request(req));

            Box::pin(fut)
        }
    }
}

poll_req!(GetUserScores<'_> => Vec<Score>);

/// Get a vec of [`UserCompact`](crate::model::user::UserCompact) by their ids.
#[must_use = "futures do nothing unless you `.await` or poll them"]
pub struct GetUsers<'a> {
    fut: Option<Pending<'a, Vec<UserCompact>>>,
    osu: &'a Osu,
    query: Option<Query>,
}

impl<'a> GetUsers<'a> {
    #[inline]
    pub(crate) fn new(osu: &'a Osu, user_ids: &[u32]) -> Self {
        let mut query = Query::new();

        let iter = user_ids
            .iter()
            .take(50)
            .map(|user_id| ("id[]", user_id.to_string()));

        query.extend(iter);

        Self {
            fut: None,
            osu,
            query: Some(query),
        }
    }

    fn start(&mut self) -> Pending<'a, Vec<UserCompact>> {
        #[cfg(feature = "metrics")]
        self.osu.metrics.users.inc();

        let query = self.query.take().unwrap();
        let req = Request::from((query, Route::GetUsers));

        Box::pin(self.osu.inner.request(req))
    }
}

poll_req!(GetUsers<'_> => Vec<UserCompact>);

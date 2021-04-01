use crate::{
    config,
    model::{nationality::Nationality, user::User},
    permissions::Permissions,
    state::PointercrateState,
    video,
    view::Page,
    Result, ViewResult,
};
use actix_web::{web::Query, HttpResponse};
use actix_web_codegen::get;
use chrono::{NaiveDate, NaiveDateTime, NaiveTime, Utc};
use maud::{html, Markup, PreEscaped};
use serde::Deserialize;
use sqlx::PgConnection;

#[derive(Debug)]
pub struct OverviewDemon {
    pub id: i32,
    pub position: i16,
    pub name: String,
    pub publisher: String,
    pub video: Option<String>,
}

#[derive(Debug)]
pub struct DemonlistOverview {
    pub demon_overview: Vec<OverviewDemon>,
    pub admins: Vec<User>,
    pub mods: Vec<User>,
    pub helpers: Vec<User>,
    pub nations: Vec<Nationality>,
    pub when: Option<NaiveDateTime>,
}

pub async fn overview_demons(connection: &mut PgConnection, at: Option<NaiveDateTime>) -> Result<Vec<OverviewDemon>> {
    match at {
        None => Ok(sqlx::query_as!(
                OverviewDemon,
                r#"SELECT demons.id, position, demons.name as "name: String", CASE WHEN verifiers.link_banned THEN NULL ELSE video::TEXT END, 
                 players.name as "publisher: String" FROM demons INNER JOIN players ON demons.publisher = players.id INNER JOIN players AS verifiers 
                 ON demons.verifier = verifiers.id WHERE position IS NOT NULL ORDER BY position"#
            )
            .fetch_all(connection)
            .await?),
        Some(time) => Ok(sqlx::query_as!(
                OverviewDemon,
                r#"SELECT demons.id as "id!", position as "position!", demons.name as "name!: String", CASE WHEN verifiers.link_banned THEN NULL ELSE video::TEXT END, 
                 players.name as "publisher: String" FROM list_at($1) AS demons INNER JOIN players ON demons.publisher = players.id INNER JOIN players AS verifiers 
                 ON demons.verifier = verifiers.id ORDER BY position"#, time
            )
            .fetch_all(connection)
            .await?)
    }
}

impl DemonlistOverview {
    pub(super) fn team_panel(&self) -> Markup {
        let maybe_link = |user: &User| -> Markup {
            html! {
                li {
                    @match user.youtube_channel {
                        Some(ref channel) => a target = "_blank" href = (channel) {
                            (user.name())
                        },
                        None => (user.name())
                    }
                }
            }
        };

        html! {
            section.panel.fade.js-scroll-anim#editors data-anim = "fade" {
                div.underlined {
                    h2 {
                        "List Editors:"
                    }
                }
                p {
                    "Contact any of these people if you have problems with the list or want to see a specific thing changed."
                }
                ul style = "line-height: 30px" {
                    @for admin in &self.admins {
                        b {
                            (maybe_link(admin))
                        }
                    }
                    @for moderator in &self.mods {
                        (maybe_link(moderator))
                    }
                }
                div.underlined {
                    h2 {
                        "List Helpers"
                    }
                }
                p {
                    "Contact these people if you have any questions regarding why a specific record was rejected. Do not needlessly bug them about checking submissions though!"
                }
                ul style = "line-height: 30px" {
                    @for helper in &self.helpers {
                        (maybe_link(helper))
                    }
                }
            }
        }
    }

    pub(super) async fn load(connection: &mut PgConnection, when: Option<NaiveDateTime>) -> Result<DemonlistOverview> {
        let admins = User::by_permission(Permissions::ListAdministrator, connection).await?;
        let mods = User::by_permission(Permissions::ListModerator, connection).await?;
        let helpers = User::by_permission(Permissions::ListHelper, connection).await?;

        let nations = Nationality::all(connection).await?;
        let demon_overview = overview_demons(connection, when).await?;

        Ok(DemonlistOverview {
            admins,
            mods,
            helpers,
            nations,
            demon_overview,
            when,
        })
    }
}

#[derive(Deserialize)]
pub struct TimeMachineData {
    when: Option<NaiveDateTime>,
}

#[get("/demonlist/")]
pub async fn index(state: PointercrateState, when: Query<TimeMachineData>) -> ViewResult<HttpResponse> {
    /* static */
    let EARLIEST_DATE: NaiveDateTime = NaiveDateTime::new(NaiveDate::from_ymd(2017, 8, 5), NaiveTime::from_hms(0, 0, 0));

    let mut connection = state.connection().await?;

    let mut specified_when = when.into_inner().when;

    if let Some(when) = specified_when {
        if when < EARLIEST_DATE {
            specified_when = Some(EARLIEST_DATE);
        }
        if when >= Utc::now().naive_utc() {
            specified_when = None;
        }
    }

    Ok(HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(DemonlistOverview::load(&mut connection, specified_when).await?.render().0))
}

impl Page for DemonlistOverview {
    fn title(&self) -> String {
        "Geometry Dash Demonlist".to_string()
    }

    fn description(&self) -> String {
        "The official pointercrate Demonlist!".to_string()
    }

    fn scripts(&self) -> Vec<&str> {
        vec!["js/modules/form.mjs", "js/modules/demonlist.mjs", "js/demonlist.v2.2.js"]
    }

    fn stylesheets(&self) -> Vec<&str> {
        vec!["css/demonlist.v2.1.css", "css/sidebar.css"]
    }

    fn body(&self) -> Markup {
        let dropdowns = super::dropdowns(&self.demon_overview, None);

        html! {
            (super::besides_sidebar_ad())
            (dropdowns)

            div.flex.m-center.container {
                main.left {
                    (super::submission_panel(&self.demon_overview))
                    (super::stats_viewer(&self.nations))
                    @if let Some(when) = self.when {
                        div.panel.fade.blue.flex style="align-items: end; " {
                             span style = "text-align: end"{"You are currently looking at the demonlist how it was on"
                             br;
                             b{(when)}}
                             a.white.button href = "/demonlist/" style = "margin-left: 15px"{ b{"Go to present" }}
                        }
                    }
                    @for demon in &self.demon_overview {
                        @if demon.position <= config::extended_list_size() {
                            section.panel.fade style="overflow:hidden" {
                                div.flex style = "align-items: center" {
                                    @if let Some(ref video) = demon.video {
                                        div.thumb."ratio-16-9"."js-delay-css" style = "position: relative" data-property = "background-image" data-property-value = {"url('" (video::thumbnail(video)) "')"} {
                                            a.play href = (video) {}
                                        }
                                        div style = "padding-left: 15px" {
                                            h2 style = "text-align: left; margin-bottom: 0px" {
                                                a href = {"/demonlist/permalink/" (demon.id) "/"} {
                                                    "#" (demon.position) (PreEscaped(" &#8211; ")) (demon.name)
                                                }
                                            }
                                            h3 style = "text-align: left" {
                                                i {
                                                    (demon.publisher)
                                                }
                                            }
                                        }
                                    }
                                    @else {
                                        h2 {
                                            a href = {"/demonlist/" (demon.position)} {
                                                "#" (demon.position) (PreEscaped(" &#8211; ")) (demon.name) " by " (demon.publisher)
                                            }
                                        }
                                    }
                                }
                            }
                            @if demon.position == 1 {
                                section.panel.fade style = "padding: 0px"{
                                (PreEscaped(r#"
                                    <script async src="https://pagead2.googlesyndication.com/pagead/js/adsbygoogle.js"></script>
                                    <!-- Demonlist Responsive Feed Ad -->
                                    <ins class="adsbygoogle"
                                         style="display:block"
                                         data-ad-client="ca-pub-3064790497687357"
                                         data-ad-slot="2819150519"
                                         data-ad-format="auto"
                                         data-full-width-responsive="true"></ins>
                                    <script>
                                         (adsbygoogle = window.adsbygoogle || []).push({});
                                    </script>
                                    "#))
                                }
                            }
                            // Place ad every 20th demon
                            @if demon.position % 20 == 0 {
                                section.panel.fade {
                                (PreEscaped(r#"
                                    <script async src="https://pagead2.googlesyndication.com/pagead/js/adsbygoogle.js"></script>
                                    <ins class="adsbygoogle"
                                         style="display:block"
                                         data-ad-format="fluid"
                                         data-ad-layout-key="-h1+40+4u-93+n"
                                         data-ad-client="ca-pub-3064790497687357"
                                         data-ad-slot="5157884729"></ins>
                                    <script>
                                         (adsbygoogle = window.adsbygoogle || []).push({});
                                    </script>
                                    "#))
                                }
                            }
                        }
                    }
                }

                aside.right {
                    (self.team_panel())
                    (super::sidebar_ad())
                    (super::rules_panel())
                    (super::submit_panel())
                    (super::stats_viewer_panel())
                    (super::discord_panel())
                }
            }

        }
    }

    fn head(&self) -> Vec<Markup> {
        vec![
            html! {
            (PreEscaped(r#"
                <link href="https://cdnjs.cloudflare.com/ajax/libs/flag-icon-css/3.4.3/css/flag-icon.min.css" rel="stylesheet">
                <script type="application/ld+json">
                {
                    "@context": "http://schema.org",
                    "@type": "WebPage",
                    "breadcrumb": {
                        "@type": "BreadcrumbList",
                        "itemListElement": [
                            {
                                "@type": "ListItem",
                                "position": 1,
                                "item": {
                                    "@id": "https://pointercrate.com/",
                                    "name": "pointercrate"
                                }
                            },
                            {
                                "@type": "ListItem",
                                "position": 2,
                                "item": {
                                    "@id": "https://pointercrate.com/demonlist/",
                                    "name": "demonlist"
                                }
                            }
                        ]
                    },
                    "name": "Geometry Dash Demonlist",
                    "description": "The official pointercrate Demonlist!",
                    "url": "https://pointercrate.com/demonlist/"
                }
                </script>
            "#))
            },
            html! {
                (PreEscaped(format!("
                    <script>
                        window.list_length = {0};
                        window.extended_list_length = {1}
                    </script>", config::list_size(), config::extended_list_size())
                ))
            },
        ]
    }
}

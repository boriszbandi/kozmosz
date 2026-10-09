//! One gallery photo, large: /projektek/<project>/kep/<slug>. Zero JavaScript: previous/next are
//! links (prefetched by the speculation rules), and the tile morphs into the large photo through
//! a cross-document view transition (style/main.css, "Photo page").

use leptos::prelude::*;
use leptos_router::hooks::use_params_map;

use super::NotFound;
use crate::{
    content::{doc, PROJECTS},
    gallery::{self, Shot},
    icons,
    layout::PageMeta,
    media::{Caption, Icon, Responsive},
};

#[component]
pub fn PhotoPage() -> impl IntoView {
    let (project, slug) = use_params_map().with_untracked(|p| (p.get("project").unwrap_or_default(), p.get("slug").unwrap_or_default()));
    let page = format!("projektek-{project}");
    let Some(&page) = PROJECTS.iter().find(|&&p| p == page) else { return view! { <NotFound/> }.into_any() };
    let shots = gallery::shots(page);
    let Some(index) = shots.iter().position(|s| s.slug == slug) else { return view! { <NotFound/> }.into_any() };
    let doc = doc(page);
    let route = doc.route.clone();
    let link = |s: &Shot| gallery::photo_href(&route, &s.slug);
    let prev = index.checked_sub(1).map(|i| link(&shots[i]));
    let next = shots.get(index + 1).map(link);
    let count = shots.len();
    let shot = shots[index].clone();

    let share = shot.img.jpeg_at_most(1600);
    // The photo fits the viewport: as wide as the stage, or as wide as 76vh allows for its
    // shape. The breakpoint between the two is where 94vw = 76vh * ratio.
    let ratio = shot.img.ratio();
    let sizes = format!(
        "(max-aspect-ratio: {}/1000) 94vw, calc(76vh * {ratio:.3})",
        (ratio * 76.0 / 94.0 * 1000.0).round() as u32,
    );
    let detail = (shot.alt != shot.title).then(|| shot.alt.clone());
    let time = match &shot.caption {
        Some(Caption::Time(iso, label)) => Some((iso.clone(), label.clone())),
        _ => None,
    };
    let back = format!("{route}#{}", gallery::tile_id(&shot.slug));
    // The frame's size is known before the file arrives (no layout shift, and the view
    // transition morphs into the right box): ratio and natural width as custom properties.
    let frame_style = format!("--r:{ratio:.4};--w:{}px", shot.img.width);
    let has_steps = prev.is_some() || next.is_some();

    view! {
        <PageMeta
            title=format!("{} | {}", shot.title, doc.title)
            description=shot.alt.clone()
            path=gallery::photo_href(&route, &shot.slug)
            image=share
        />
        <article class="photo-view">
            <div class="container photo-bar">
                <a class="photo-back" href=back>
                    <Icon svg=icons::ARROW_RIGHT/>
                    {doc.title.clone()}
                </a>
                <p class="photo-count mono">
                    <span class="visually-hidden">"Felvétel: "</span>
                    {index + 1}
                    <span aria-hidden="true">" / "</span>
                    <span class="visually-hidden">", összesen "</span>
                    {count}
                </p>
            </div>
            <figure class="photo-stage">
                <div class="photo-frame" style=frame_style>
                    <Responsive img=shot.img alt=shot.alt.clone() sizes eager=true/>
                    // Glass steps floating over the photo's edges (wide screens), labelled
                    // buttons under it (phones).
                    {has_steps.then(|| view! {
                    <nav class="photo-nav" aria-label="Lapozás a felvételek között">
                        {prev.map(|href| view! {
                            <a class="button button-line photo-step" href=href rel="prev">
                                <Icon svg=icons::ARROW_RIGHT/>
                                <span class="photo-step-label">"Előző"</span>
                            </a>
                        })}
                        {next.map(|href| view! {
                            <a class="button button-line photo-step photo-step-next" href=href rel="next">
                                <span class="photo-step-label">"Következő"</span>
                                <Icon svg=icons::ARROW_RIGHT/>
                            </a>
                        })}
                    </nav>
                    })}
                </div>
                <figcaption class="container photo-caption">
                    {match time {
                        Some((iso, label)) => view! {
                            <h1 class="photo-title">"Vétel: "<time class="mono" datetime=iso>{label}</time></h1>
                        }
                        .into_any(),
                        None => view! { <h1 class="photo-title">{shot.title}</h1> }.into_any(),
                    }}
                    // The alt text says the same to screen readers.
                    {detail.map(|d| view! { <p aria-hidden="true">{d}</p> })}
                </figcaption>
            </figure>
        </article>
    }
    .into_any()
}

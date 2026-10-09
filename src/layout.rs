//! Page chrome: head metadata, header with navigation, footer.

use leptos::prelude::*;
use leptos_meta::{Link, Meta, Title};
use leptos_router::components::A;

use crate::{icons, media::Icon, site};

/// Title, description, canonical URL and Open Graph tags of a page.
#[component]
pub fn PageMeta(
    #[prop(into)] title: String,
    #[prop(into)] description: String,
    /// Route of the page, e.g. "/rolunk".
    #[prop(into)]
    path: String,
    /// Share image: (path, width, height). Default: the site's og.jpg.
    #[prop(optional)]
    image: Option<(String, u32, u32)>,
) -> impl IntoView {
    let (image, image_w, image_h) = image.unwrap_or_else(|| ("/img/og.jpg".into(), 1200, 630));
    let full_title = if title.is_empty() { site::NAME.to_owned() } else { format!("{title} | {}", site::NAME) };
    let url = format!("{}{}", site::ORIGIN, if path == "/" { "" } else { &path });
    view! {
        <Title text=title/>
        <Meta name="description" content=description.clone()/>
        <Link rel="canonical" href=url.clone()/>
        <Meta property="og:type" content="website"/>
        <Meta property="og:site_name" content=site::NAME/>
        <Meta property="og:locale" content="hu_HU"/>
        <Meta property="og:title" content=full_title/>
        <Meta property="og:description" content=description/>
        <Meta property="og:url" content=url/>
        <Meta property="og:image" content=format!("{}{image}", site::ORIGIN)/>
        <Meta property="og:image:width" content=image_w.to_string()/>
        <Meta property="og:image:height" content=image_h.to_string()/>
    }
}

#[component]
pub fn Header() -> impl IntoView {
    view! {
        <a class="skip-link" href="#tartalom">"Ugrás a tartalomra"</a>
        <header class="site-header">
            <div class="container header-inner">
                <a href="/" class="brand">
                    <img src="/img/kozmosz-mark.svg" alt="" width="36" height="36"/>
                    <span>{site::SHORT_NAME}</span>
                </a>
                <nav class="nav-desktop" aria-label="Főmenü">
                    <NavLinks/>
                </nav>
                // Zero-JS mobile menu: <details> toggles a full-screen panel.
                <nav class="nav-mobile" aria-label="Főmenü">
                    <details>
                        <summary>
                            "Menü"
                            <span class="nav-mobile-open"><Icon svg=icons::LIST/></span>
                            <span class="nav-mobile-close"><Icon svg=icons::X/></span>
                        </summary>
                        <div class="nav-mobile-panel">
                            <NavLinks/>
                        </div>
                    </details>
                </nav>
            </div>
        </header>
    }
}

#[component]
fn NavLinks() -> impl IntoView {
    view! {
        <ul class="nav-list" role="list">
            {site::NAV
                .iter()
                .map(|item| {
                    let children = (!item.children.is_empty()).then(|| {
                        view! {
                            <ul class="nav-sub" role="list">
                                {item
                                    .children
                                    .iter()
                                    // Not exact: the photo pages under a project (/projektek/isstv/kep/…)
                                    // keep the project, and so "Projektek", marked as current.
                                    .map(|&(label, href)| view! { <li><A href=href>{label}</A></li> })
                                    .collect_view()}
                            </ul>
                        }
                    });
                    let has_sub = children.is_some();
                    let class = if has_sub { "nav-item has-sub" } else { "nav-item" };
                    view! {
                        <li class=class>
                            <A href=item.href exact=true>
                                {item.label}
                                {has_sub.then(|| view! { <Icon svg=icons::CARET_DOWN/> })}
                                // The glass pill behind the current item; it glides to the new
                                // item between pages (view transition, style/main.css).
                                <span class="nav-ink" aria-hidden="true"></span>
                            </A>
                            {children}
                        </li>
                    }
                })
                .collect_view()}
        </ul>
    }
}

#[component]
pub fn Footer() -> impl IntoView {
    view! {
        <footer class="site-footer">
            <div class="container footer-inner">
                <a class="footer-brand" href="/">
                    <img src="/img/kozmosz-mark.svg" alt="" width="32" height="32"/>
                    {site::NAME}
                </a>
                <address class="footer-contact">
                    {site::ADDRESS}
                    <br/>
                    <a href=format!("mailto:{}", site::EMAIL)>{site::EMAIL}</a>
                </address>
                <ul class="footer-social" role="list">
                    <li>
                        <a href=site::FACEBOOK_URL rel="noopener" aria-label="Facebook">
                            <Icon svg=icons::FACEBOOK_LOGO/>
                        </a>
                    </li>
                    <li>
                        <a href=site::INSTAGRAM_URL rel="noopener" aria-label="Instagram">
                            <Icon svg=icons::INSTAGRAM_LOGO/>
                        </a>
                    </li>
                </ul>
                <p class="footer-copy">"© 2026 " {site::NAME}</p>
            </div>
            // The name as a horizon: huge, tone on tone, cut by the page's bottom edge.
            <div class="container footer-mark" aria-hidden="true">
                <span>{site::SHORT_NAME}</span>
            </div>
        </footer>
    }
}
